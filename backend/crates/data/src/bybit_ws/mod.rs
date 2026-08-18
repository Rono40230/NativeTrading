//! Worker d'ingestion Bybit WebSocket — crypto + métaux, 24/7, sans clé API.
//!
//! Connexion publique à `wss://stream.bybit.com/v5/public/linear`, souscription
//! aux topics `kline.{interval}.{symbol}` pour les actifs × timeframes lus en
//! DB (`assets.symbol_bybit` × `configuration.worker_timeframes`).
//!
//! Deux sorties par kline reçue :
//! - **flux runtime** (optionnel, `EvenementPrix`) : TOUTES les klines,
//!   confirmées ou non — la bougie en formation pousse l'évaluation intrabar
//!   du runtime tick, la confirmation clôture avec les valeurs officielles ;
//! - **DB** : uniquement les bougies fermées (`confirm: true`), inchangé.
//!
//! Aucune liste d'actifs n'est hardcodée : chaque session relit la DB —
//! activer/désactiver un asset ou changer les timeframes depuis l'UI est pris
//! en compte à la reconnexion suivante (backoff max 60 s).
//!
//! Résilience : reconnect automatique avec backoff exponentiel (2 s → 60 s),
//! réinitialisé après une session stable. Répond aux pings JSON de Bybit
//! (`{"op":"ping"}` → `{"op":"pong"}`) et aux pings protocolaires, plus un
//! heartbeat applicatif périodique pour maintenir la connexion ouverte.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use common::{Asset, Candle, Timeframe};
use db::Database;
use engine::{EvenementPrix, PrixEvent};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio::time::sleep;
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::worker_config::{lire_actif, lire_timeframes, CLE_ACTIF_BYBIT};
use crate::worker_status::STATUT_BYBIT;

mod messages;
#[cfg(test)]
mod tests;

use messages::{ActionWs, traiter_texte};

/// URLs par marché — décision propriétaire 2026-08-18 : UNE seule série de
/// prix par asset, alignée sur la référence TV (`BYBIT:BTCUSDT` = spot pour
/// les cryptos). Les métaux XAU/XAG n'existent chez Bybit qu'en linear.
/// Avant : tout le monde en linear → la DB mélangeait spot (backfill REST)
/// et perp (WS) à ~30 $ d'écart — cause racine de la divergence des signaux.
const BYBIT_WS_URL_SPOT: &str = "wss://stream.bybit.com/v5/public/spot";
const BYBIT_WS_URL_LINEAR: &str = "wss://stream.bybit.com/v5/public/linear";

/// Un symbole relève du marché linear (contrats — métaux) plutôt que spot.
fn est_metal_linear(symbole: &str) -> bool {
    matches!(symbole, "XAUUSDT" | "XAGUSDT")
}

/// Source enregistrée en DB pour les bougies issues de ce worker.
const SOURCE: &str = "bybit_ws";

/// Backoff initial entre deux tentatives de reconnexion (secondes).
const BACKOFF_DEPART_SEC: u64 = 2;
/// Backoff maximal entre deux tentatives de reconnexion (secondes).
const BACKOFF_MAX_SEC: u64 = 60;
/// Durée minimale d'une session pour la considérer « stable » et réinitialiser
/// le backoff (secondes).
const SESSION_STABLE_SEC: u64 = 30;
/// Nombre maximal d'args (topics) par message `subscribe` (limite Bybit = 100).
const NB_ARGS_MAX: usize = 60;

/// Garde anti-double-start. Le worker doit n'être spawné qu'une fois.
/// Pattern identique à `SMC_DEMARREE` dans `api::smc_boucle`.
static BYBIT_WS_DEMARRE: AtomicBool = AtomicBool::new(false);

/// Marque le worker comme démarré. Retourne `true` s'il s'agit du premier
/// appel (le spawn doit avoir lieu), `false` sinon.
fn marquer_demarre() -> bool {
    !BYBIT_WS_DEMARRE.swap(true, Ordering::SeqCst)
}

// ─────────────────────────────────────────────────────────────────────────────
// Actifs et timeframes dynamiques (source : DB)
// ─────────────────────────────────────────────────────────────────────────────

/// Lit depuis la DB les actifs à ingérer via Bybit : couples
/// `(symbol_bybit, asset_id)` filtrés sur `source='binance' AND actif AND
/// symbol_bybit IS NOT NULL`. Toute erreur DB retourne une liste vide — le
/// worker retentera à la session suivante.
async fn assets_bybit_depuis_db(db: &Arc<Database>) -> Vec<(String, String)> {
    match db.lister_assets_worker().await {
        Ok(assets) => assets
            .into_iter()
            .filter(|a| a.actif && a.source == "binance")
            .filter_map(|a| a.symbol_bybit.map(|s| (s, a.id)))
            .collect(),
        Err(e) => {
            tracing::warn!("Bybit WS: lecture DB des actifs impossible ({}) — retry plus tard", e);
            Vec::new()
        }
    }
}

/// Mappe un interval Bybit vers le timeframe DB.
fn bybit_interval_vers_tf(interval: &str) -> Option<&'static str> {
    match interval {
        "1" => Some("M1"),
        "5" => Some("M5"),
        "15" => Some("M15"),
        "30" => Some("M30"),
        "60" => Some("H1"),
        "240" => Some("H4"),
        "D" => Some("D1"),
        "W" => Some("W1"),
        _ => None,
    }
}

/// Mappe un timeframe DB vers l'interval Bybit (inverse du précédent).
fn tf_vers_bybit_interval(tf: &Timeframe) -> Option<&'static str> {
    match tf {
        Timeframe::M1 => Some("1"),
        Timeframe::M5 => Some("5"),
        Timeframe::M15 => Some("15"),
        Timeframe::M30 => Some("30"),
        Timeframe::H1 => Some("60"),
        Timeframe::H4 => Some("240"),
        Timeframe::D1 => Some("D"),
        Timeframe::W1 => Some("W"),
    }
}

/// Construit la liste des topics `kline.{interval}.{symbol}` pour les actifs ×
/// timeframes donnés. Fonction pure → testable.
fn construire_topics(assets: &[(String, String)], timeframes: &[Timeframe]) -> Vec<String> {
    let mut v = Vec::with_capacity(assets.len() * timeframes.len());
    for tf in timeframes {
        let Some(interval) = tf_vers_bybit_interval(tf) else {
            continue;
        };
        for (symbol, _) in assets {
            v.push(format!("kline.{}.{}", interval, symbol));
        }
    }
    v
}

/// Empreinte de la config d'une session (assets × timeframes) — comparée
/// périodiquement pour détecter un ajout/retrait d'asset et forcer une
/// reconnexion propre (resouscription avec la nouvelle liste).
fn empreinte_session(assets: &[(String, String)], timeframes: &[Timeframe]) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    let mut symboles: Vec<&String> = assets.iter().map(|(s, _)| s).collect();
    symboles.sort();
    for s in symboles {
        s.hash(&mut h);
    }
    for tf in timeframes {
        tf.as_str().hash(&mut h);
    }
    h.finish()
}

/// Construit le mapping symbol Bybit → asset DB pour le parsing des messages.
/// Un doublon de symbole écrase silencieusement le premier (dernier gagnant).
fn construire_mapping(assets: &[(String, String)]) -> HashMap<String, String> {
    assets.iter().cloned().collect()
}

// ─────────────────────────────────────────────────────────────────────────────
// Boucle principale + reconnect
// ─────────────────────────────────────────────────────────────────────────────

/// Démarre le worker en arrière-plan — non bloquant. Idempotent : un second
/// appel est un no-op + avertissement.
///
/// `flux_runtime` : si fourni, TOUTES les klines (confirmées et non
/// confirmées) y sont poussées comme [`EvenementPrix`] pour le runtime
/// tick — c'est le chemin temps réel (évaluation intrabar). La DB ne
/// reçoit toujours que les bougies confirmées.
pub fn demarrer_worker_bybit(
    db: Arc<Database>,
    flux_runtime: Option<mpsc::UnboundedSender<EvenementPrix>>,
) {
    if !marquer_demarre() {
        tracing::warn!("⚠️  Worker Bybit WS déjà démarré — second spawn ignoré");
        return;
    }
    // Une boucle par marché : spot (cryptos — la référence TV) et linear
    // (métaux). Chacune gère sa reconnexion et son empreinte de session.
    // La garde reste levée jusqu'au redémarrage du process (boucles infinies).
    let db_spot = db.clone();
    let flux_spot = flux_runtime.clone();
    tokio::spawn(async move {
        boucle_reconnect(db_spot, flux_spot, BYBIT_WS_URL_SPOT, false).await;
        tracing::error!("Bybit WS spot: boucle terminée — ingestion spot arrêtée");
    });
    tokio::spawn(async move {
        boucle_reconnect(db, flux_runtime, BYBIT_WS_URL_LINEAR, true).await;
        tracing::error!("Bybit WS linear: boucle terminée — ingestion linear arrêtée");
    });
}

/// Boucle de reconnexion infinie d'UN marché. À chaque session terminée
/// (erreur ou déconnexion), on attend `backoff` secondes puis on retente. Le
/// backoff croît exponentiellement (×2, plafonné à 60 s) et se réinitialise
/// après une session stable.
async fn boucle_reconnect(
    db: Arc<Database>,
    flux_runtime: Option<mpsc::UnboundedSender<EvenementPrix>>,
    url: &str,
    linear: bool,
) {
    let marche = if linear { "linear" } else { "spot" };
    tracing::info!("🌐 Bybit WS {marche}: démarrage worker ingestion (actifs/timeframes pilotés en DB)");

    let mut backoff = BACKOFF_DEPART_SEC;
    loop {
        let debut_session = Instant::now();
        let resultat = session_unique(&db, &flux_runtime, url, linear).await;
        STATUT_BYBIT.marque_deconnecte();

        let duree = debut_session.elapsed();
        match resultat {
            Ok(()) => {
                tracing::info!(
                    "Bybit WS {marche}: session fermée proprement après {:?}, reconnect dans {}s",
                    duree,
                    backoff
                );
            }
            Err(e) => {
                tracing::warn!(
                    "Bybit WS {marche}: session perdue après {:?} ({}), reconnect dans {}s",
                    duree,
                    e,
                    backoff
                );
            }
        }

        // Réinitialise le backoff si la session a duré assez longtemps pour
        // être considérée stable ; sinon on l'agrandit.
        if duree.as_secs() >= SESSION_STABLE_SEC {
            backoff = BACKOFF_DEPART_SEC;
        } else {
            backoff = (backoff.saturating_mul(2)).min(BACKOFF_MAX_SEC);
        }

        sleep(Duration::from_secs(backoff)).await;
    }
}

/// Établit une connexion, souscrit aux topics, et traite les messages jusqu'à
/// déconnexion ou erreur. Retourne `Ok(())` sur fermeture propre ou si le
/// worker est désactivé / n'a aucun actif à suivre (sans se connecter).
async fn session_unique(
    db: &Arc<Database>,
    flux_runtime: &Option<mpsc::UnboundedSender<EvenementPrix>>,
    url: &str,
    linear: bool,
) -> anyhow::Result<()> {
    // Interrupteur UI : worker désactivé → pas de connexion, retry au backoff max.
    if !lire_actif(db, CLE_ACTIF_BYBIT).await {
        tracing::debug!("Bybit WS: worker désactivé (worker_actif_bybit=0) — session sautée");
        return Ok(());
    }

    // Actifs et timeframes relus à CHAQUE session : la config UI s'applique
    // en ≤ 60 s sans redémarrage. Filtrage par marché : spot (cryptos,
    // référence TV) ou linear (métaux).
    let assets: Vec<(String, String)> = assets_bybit_depuis_db(db)
        .await
        .into_iter()
        .filter(|(symbole, _)| est_metal_linear(symbole) == linear)
        .collect();
    let timeframes = lire_timeframes(db).await;
    if assets.is_empty() || timeframes.is_empty() {
        tracing::debug!(
            "Bybit WS {}: aucun actif ou timeframe à suivre — session sautée",
            if linear { "linear" } else { "spot" }
        );
        return Ok(());
    }
    let topics = construire_topics(&assets, &timeframes);
    let mapping = construire_mapping(&assets);
    tracing::info!(
        "Bybit WS: session prévue pour {} actifs × {} timeframes = {} topics",
        assets.len(),
        timeframes.len(),
        topics.len()
    );

    tracing::info!("Bybit WS: connexion à {}", url);
    let (ws, reponse) = match connect_async(url).await {
        Ok((ws, reponse)) => (ws, reponse),
        Err(e) => {
            return Err(anyhow::anyhow!("connexion WS échouée: {}", e));
        }
    };
    tracing::info!("Bybit WS: connecté (HTTP {})", reponse.status());
    STATUT_BYBIT.marque_connecte(assets.len() as u64);

    let (mut sortie, mut entree) = ws.split();

    // Souscription aux topics, par morceaux si NB_ARGS_MAX < nb topics.
    let nb_morceaux = topics.len().div_ceil(NB_ARGS_MAX).max(1);
    for morceau in topics.chunks(NB_ARGS_MAX) {
        let souscription = serde_json::json!({ "op": "subscribe", "args": morceau });
        let payload = serde_json::to_string(&souscription).unwrap_or_default();
        sortie
            .send(Message::Text(payload))
            .await
            .map_err(|e| anyhow::anyhow!("échec envoi souscription: {}", e))?;
    }
    tracing::info!(
        "Bybit WS: souscription envoyée ({} topics en {} message(s))",
        topics.len(),
        nb_morceaux
    );

    // ── Boucle principale : lecture WS + relecture périodique de la config ────
    // Bybit SERVEUR envoie {"op":"ping","args":[<ts>]} toutes les 20s.
    // On répond {"op":"pong","args":[<ts>]}. Le client NE DOIT PAS initier
    // un ping (sinon Bybit répond error:invalid op).
    //
    // Toutes les 30 s, l'empreinte assets × timeframes est relue en DB : un
    // ajout/retrait d'asset (modale UI) force une reconnexion propre — le
    // nouvel asset est souscrit en ≤ 30 s + backoff, sans redémarrage.
    let mut dernier_hash = empreinte_session(&assets, &timeframes);
    let mut tick_config = tokio::time::interval(std::time::Duration::from_secs(30));
    tick_config.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        let message = tokio::select! {
            biased;
            _ = tick_config.tick() => {
                // Relire la config AVEC le même filtre de marché que la
                // session : comparer 4 assets à un sous-ensemble filtré
                // provoquait une différence permanente → reconnexion en
                // boucle sans jamais ingérer (bug 2026-08-18 soir).
                let assets_maj: Vec<(String, String)> = assets_bybit_depuis_db(db)
                    .await
                    .into_iter()
                    .filter(|(symbole, _)| est_metal_linear(symbole) == linear)
                    .collect();
                let tfs_maj = crate::worker_config::lire_timeframes(db).await;
                if !assets_maj.is_empty() && empreinte_session(&assets_maj, &tfs_maj) != dernier_hash {
                    tracing::info!(
                        "Bybit WS: config assets×TF modifiée ({} → {} actifs) — reconnexion pour resouscription",
                        assets.len(),
                        assets_maj.len()
                    );
                    return Ok(());
                }
                continue;
            }
            peut_etre = entree.next() => match peut_etre {
                Some(m) => m,
                None => return Err(anyhow::anyhow!("flux WS fermé par le serveur")),
            },
        };
        match message {
            Ok(Message::Text(texte)) => {
                match traiter_texte(&texte, &mapping) {
                    ActionWs::Pong => {
                        let pong = {
                            let parsed = serde_json::from_str::<serde_json::Value>(&texte);
                            match parsed {
                                Ok(v) => match v.get("args").and_then(|a| a.as_array()).and_then(|a| a.first()) {
                                    Some(ts) => format!(r#"{{"op":"pong","args":[{}]}}"#, ts),
                                    None => r#"{"op":"pong"}"#.to_string(),
                                },
                                Err(_) => r#"{"op":"pong"}"#.to_string(),
                            }
                        };
                        if sortie.send(Message::Text(pong)).await.is_err() {
                            return Err(anyhow::anyhow!("échec envoi pong"));
                        }
                    }
                    ActionWs::Klines(klines) => {
                        // 1. Runtime tick : TOUTES les klines (formation +
                        //    confirmations) — chemin temps réel intrabar.
                        if let Some(tx) = flux_runtime {
                            for k in &klines {
                                let _ = tx.send(EvenementPrix {
                                    asset: k.asset.clone(),
                                    tf: k.tf,
                                    debut_bougie: k.debut,
                                    event: PrixEvent::Kline {
                                        ouverture: k.open,
                                        haut: k.high,
                                        bas: k.low,
                                        cloture: k.close,
                                        volume: k.volume,
                                        confirmee: k.confirmee,
                                    },
                                    recu_le: Utc::now(),
                                });
                            }
                        }
                        // 2. DB : uniquement les bougies confirmées (inchangé).
                        let confirmees: Vec<(Asset, Timeframe, Candle)> = klines
                            .iter()
                            .filter(|k| k.confirmee)
                            .map(|k| {
                                (
                                    k.asset.clone(),
                                    k.tf,
                                    Candle {
                                        timestamp: DateTime::<Utc>::from_timestamp(k.debut, 0)
                                            .unwrap_or_else(Utc::now),
                                        open: k.open,
                                        high: k.high,
                                        low: k.low,
                                        close: k.close,
                                        volume: k.volume,
                                    },
                                )
                            })
                            .collect();
                        inserer_bougies(db, confirmees).await;
                    }
                    ActionWs::Ignorer => {}
                }
            }
            Ok(Message::Ping(payload)) => {
                let _ = sortie.send(Message::Pong(payload)).await;
            }
            Ok(Message::Pong(_) | Message::Binary(_)) => {}
            Ok(Message::Close(_)) => {
                tracing::info!("Bybit WS: frame Close reçue du serveur");
                return Ok(());
            }
            Ok(Message::Frame(_)) => {}
            Err(e) => {
                return Err(anyhow::anyhow!("erreur lecture WS: {}", e));
            }
        }
    }
}

/// Insère en DB les bougies fermées issues d'un message, une par une, via le
/// pool SQLite existant. Les erreurs d'insertion sont loggées mais ne font pas
/// tomber la connexion.
async fn inserer_bougies(db: &Arc<Database>, bougies: Vec<(Asset, Timeframe, Candle)>) {
    for (asset, tf, bougie) in bougies {
        let ts = bougie.timestamp.timestamp();
        let unique = [bougie];
        match db
            .inserer_bougies_avec_source(&asset, &tf, &unique, SOURCE)
            .await
        {
            Ok(_) => {
                STATUT_BYBIT.consigne_bougie(ts);
                tracing::debug!(
                    "Bybit WS: bougie fermée insérée {} {} ts={}",
                    asset.as_str(),
                    tf.as_str(),
                    ts
                );
            }
            Err(e) => {
                tracing::warn!(
                    "Bybit WS: erreur DB {} {}: {}",
                    asset.as_str(),
                    tf.as_str(),
                    e
                );
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests unitaires (pas de réseau, pas de DB)
// ─────────────────────────────────────────────────────────────────────────────

