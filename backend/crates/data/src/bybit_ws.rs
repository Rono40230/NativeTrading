//! Worker d'ingestion Bybit WebSocket — crypto + métaux, 24/7, sans clé API.
//!
//! Connexion publique à `wss://stream.bybit.com/v5/public/linear`, souscription
//! aux topics `kline.{interval}.{symbol}` pour les 12 actifs × 5 timeframes
//! (60 topics), et écriture en DB des **bougies fermées** uniquement
//! (champ `confirm: true`).
//!
//! Résilience : reconnect automatique avec backoff exponentiel (2 s → 60 s),
//! réinitialisé après une session stable. Répond aux pings JSON de Bybit
//! (`{"op":"ping"}` → `{"op":"pong"}`) et aux pings protocolaires, plus un
//! heartbeat applicatif périodique pour maintenir la connexion ouverte.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use common::{Asset, Candle, Timeframe};
use db::Database;
use futures_util::{SinkExt, StreamExt};
use tokio::time::sleep;
use tokio_tungstenite::{connect_async, tungstenite::Message};

/// URL du WebSocket public Bybit (linear perpetuals — crypto + métaux XAU/XAG).
const BYBIT_WS_URL: &str = "wss://stream.bybit.com/v5/public/linear";

/// Source enregistrée en DB pour les bougies issues de ce worker.
const SOURCE: &str = "bybit_ws";

/// Backoff initial entre deux tentatives de reconnexion (secondes).
const BACKOFF_DEPART_SEC: u64 = 2;
/// Backoff maximal entre deux tentatives de reconnexion (secondes).
const BACKOFF_MAX_SEC: u64 = 60;
/// Durée minimale d'une session pour la considérer « stable » et réinitialiser
/// le backoff (secondes).
const SESSION_STABLE_SEC: u64 = 30;
/// Période du heartbeat applicatif (ping JSON) envoyé à Bybit (secondes).
/// Bybit ferme la connexion après ~30 s d'inactivité côté serveur.
const HEARTBEAT_SEC: u64 = 20;
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
// Mappings Bybit → DB
// ─────────────────────────────────────────────────────────────────────────────

/// Mappe un symbole Bybit vers l'identifiant d'asset utilisé en DB.
fn bybit_vers_asset(symbol: &str) -> Option<&'static str> {
    match symbol {
        "BTCUSDT" => Some("BTC"),
        "ETHUSDT" => Some("ETH"),
        "SOLUSDT" => Some("SOL"),
        "BNBUSDT" => Some("BNB"),
        "XRPUSDT" => Some("XRP"),
        "ADAUSDT" => Some("ADA"),
        "DOGEUSDT" => Some("DOGE"),
        "AVAXUSDT" => Some("AVAX"),
        "LINKUSDT" => Some("LINK"),
        "DOTUSDT" => Some("DOT"),
        "XAUUSDT" => Some("XAUUSD"),
        "XAGUSDT" => Some("XAGUSD"),
        _ => None,
    }
}

/// Mappe un interval Bybit vers le timeframe DB.
fn bybit_interval_vers_tf(interval: &str) -> Option<&'static str> {
    match interval {
        "1" => Some("M1"),
        "5" => Some("M5"),
        "15" => Some("M15"),
        "60" => Some("H1"),
        "D" => Some("D1"),
        _ => None,
    }
}

/// Symboles Bybit couverts (10 cryptos + 2 métaux).
const SYMBOLES: &[&str] = &[
    "BTCUSDT", "ETHUSDT", "SOLUSDT", "BNBUSDT", "XRPUSDT", "ADAUSDT", "DOGEUSDT", "AVAXUSDT",
    "LINKUSDT", "DOTUSDT", "XAUUSDT", "XAGUSDT",
];

/// Intervals Bybit couverts (5 timeframes du moteur SMC v12 + sentiment).
const INTERVALS: &[&str] = &["1", "5", "15", "60", "D"];

/// Génère la liste des topics `kline.{interval}.{symbol}` à souscrire.
fn topics() -> Vec<String> {
    let mut v = Vec::with_capacity(SYMBOLES.len() * INTERVALS.len());
    for &interval in INTERVALS {
        for &symbol in SYMBOLES {
            v.push(format!("kline.{}.{}", interval, symbol));
        }
    }
    v
}

/// Extrait `(interval, symbol)` d'un topic `kline.{interval}.{symbol}`.
fn extraire_topic(topic: &str) -> Option<(&str, &str)> {
    let reste = topic.strip_prefix("kline.")?;
    let (interval, symbol) = reste.split_once('.')?;
    if interval.is_empty() || symbol.is_empty() {
        return None;
    }
    Some((interval, symbol))
}

// ─────────────────────────────────────────────────────────────────────────────
// Parsing des messages WS
// ─────────────────────────────────────────────────────────────────────────────

/// Action à exécuter après parsing d'un message texte WS.
#[derive(Debug)]
enum ActionWs {
    /// Rien à faire (message ignoré : bougie non fermée, ack souscription, etc.).
    Ignorer,
    /// Bybit a envoyé un ping applicatif → répondre `{"op":"pong"}`.
    Pong,
    /// Une ou plusieurs bougies fermées à insérer en DB.
    Bougies(Vec<(Asset, Timeframe, Candle)>),
}

/// Parse un message texte brut Bybit en `ActionWs`. Fonction pure → testable
/// sans DB ni réseau.
fn traiter_texte(message: &str) -> ActionWs {
    let valeur: serde_json::Value = match serde_json::from_str(message) {
        Ok(v) => v,
        Err(e) => {
            // Message non-JSON (rare) : on ignore sans crasher.
            tracing::debug!(
                "Bybit WS: message non-JSON ignoré ({}): {:?}",
                e,
                message.get(..80).unwrap_or(message)
            );
            return ActionWs::Ignorer;
        }
    };

    // Ping applicatif Bybit → on devra répondre pong.
    if valeur.get("op").and_then(|o| o.as_str()) == Some("ping") {
        return ActionWs::Pong;
    }

    // Autres messages `op` (pong, ack subscribe). On logge seulement les échecs.
    if valeur.get("op").is_some() {
        if valeur.get("success").and_then(|s| s.as_bool()) == Some(false) {
            let msg = valeur
                .get("ret_msg")
                .and_then(|m| m.as_str())
                .unwrap_or("raison inconnue");
            tracing::warn!("Bybit WS: opération échouée: {}", msg);
        }
        return ActionWs::Ignorer;
    }

    // Message kline.
    let Some(topic) = valeur.get("topic").and_then(|t| t.as_str()) else {
        return ActionWs::Ignorer;
    };
    let Some((interval, symbol)) = extraire_topic(topic) else {
        return ActionWs::Ignorer;
    };
    let Some(asset_str) = bybit_vers_asset(symbol) else {
        return ActionWs::Ignorer; // symbole non suivi
    };
    let Some(tf_str) = bybit_interval_vers_tf(interval) else {
        return ActionWs::Ignorer; // interval non suivi
    };
    let Ok(asset) = Asset::try_from(asset_str) else {
        return ActionWs::Ignorer;
    };
    let Ok(tf) = Timeframe::try_from(tf_str) else {
        return ActionWs::Ignorer;
    };

    let Some(blocs) = valeur.get("data").and_then(|d| d.as_array()) else {
        return ActionWs::Ignorer;
    };

    let mut bougies = Vec::with_capacity(blocs.len());
    for bloc in blocs {
        // On n'écrit QUE les bougies fermées (confirm: true).
        if bloc.get("confirm").and_then(|c| c.as_bool()) != Some(true) {
            continue;
        }
        let Some(start) = bloc.get("start").and_then(|s| s.as_i64()) else {
            continue;
        };
        // Bybit WS kline v5 : `start` peut être en secondes (10 chiffres) OU
        // millisecondes (13 chiffres) selon la version du topic. On détecte.
        let start_sec = if start > 1_000_000_000_000 { start / 1000 } else { start };
        let Some(timestamp) = DateTime::<Utc>::from_timestamp(start_sec, 0) else {
            continue;
        };
        let champ_f64 = |cle: &str| bloc.get(cle).and_then(|x| x.as_str()).and_then(|s| s.parse::<f64>().ok());
        let Some(open) = champ_f64("open") else {
            continue;
        };
        let Some(high) = champ_f64("high") else {
            continue;
        };
        let Some(low) = champ_f64("low") else {
            continue;
        };
        let Some(close) = champ_f64("close") else {
            continue;
        };
        let volume = champ_f64("volume").unwrap_or(0.0);
        bougies.push((
            asset.clone(),
            tf,
            Candle {
                timestamp,
                open,
                high,
                low,
                close,
                volume,
            },
        ));
    }

    if bougies.is_empty() {
        ActionWs::Ignorer
    } else {
        ActionWs::Bougies(bougies)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Boucle principale + reconnect
// ─────────────────────────────────────────────────────────────────────────────

/// Démarre le worker en arrière-plan — non bloquant. Idempotent : un second
/// appel est un no-op + avertissement.
pub fn demarrer_worker_bybit(db: Arc<Database>) {
    if !marquer_demarre() {
        tracing::warn!("⚠️  Worker Bybit WS déjà démarré — second spawn ignoré");
        return;
    }
    tokio::spawn(async move {
        boucle_reconnect(db).await;
        // Ne devrait jamais arriver : la boucle est infinie. Si elle sort, on
        // libère le garde pour permettre un redémarrage manuel ultérieur.
        BYBIT_WS_DEMARRE.store(false, Ordering::SeqCst);
        tracing::error!("Bybit WS: boucle principale terminée — ingestion arrêtée");
    });
}

/// Boucle de reconnexion infinie. À chaque session terminée (erreur ou
/// déconnexion), on attend `backoff` secondes puis on retente. Le backoff
/// croît exponentiellement (×2, plafonné à 60 s) et se réinitialise après une
/// session stable.
async fn boucle_reconnect(db: Arc<Database>) {
    tracing::info!(
        "🌐 Bybit WS: démarrage worker ingestion ({} topics, {} actifs × {} timeframes)",
        topics().len(),
        SYMBOLES.len(),
        INTERVALS.len()
    );

    let mut backoff = BACKOFF_DEPART_SEC;
    loop {
        let debut_session = Instant::now();
        let resultat = session_unique(&db).await;

        let duree = debut_session.elapsed();
        match resultat {
            Ok(()) => {
                tracing::info!(
                    "Bybit WS: session fermée proprement après {:?}, reconnect dans {}s",
                    duree,
                    backoff
                );
            }
            Err(e) => {
                tracing::warn!(
                    "Bybit WS: session perdue après {:?} ({}), reconnect dans {}s",
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
/// déconnexion ou erreur. Retourne `Ok(())` sur fermeture propre, `Err` sinon.
async fn session_unique(db: &Arc<Database>) -> anyhow::Result<()> {
    tracing::info!("Bybit WS: connexion à {}", BYBIT_WS_URL);
    let (ws, reponse) = match connect_async(BYBIT_WS_URL).await {
        Ok((ws, reponse)) => (ws, reponse),
        Err(e) => {
            return Err(anyhow::anyhow!("connexion WS échouée: {}", e));
        }
    };
    tracing::info!("Bybit WS: connecté (HTTP {})", reponse.status());

    let (mut sortie, mut entree) = ws.split();

    // Souscription aux topics, par morceaux si NB_ARGS_MAX < nb topics.
    let topics = topics();
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

    // ── Boucle principale : lecture WS uniquement ─────────────────────────────
    // Bybit SERVEUR envoie {"op":"ping","args":[<ts>]} toutes les 20s.
    // On répond {"op":"pong","args":[<ts>]}. Le client NE DOIT PAS initier
    // un ping (sinon Bybit répond error:invalid op).
    loop {
        match entree.next().await {
            Some(Ok(Message::Text(texte))) => {
                match traiter_texte(&texte) {
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
                    ActionWs::Bougies(bougies) => {
                        inserer_bougies(db, bougies).await;
                    }
                    ActionWs::Ignorer => {}
                }
            }
            Some(Ok(Message::Ping(payload))) => {
                let _ = sortie.send(Message::Pong(payload)).await;
            }
            Some(Ok(Message::Pong(_) | Message::Binary(_))) => {}
            Some(Ok(Message::Close(_))) => {
                tracing::info!("Bybit WS: frame Close reçue du serveur");
                return Ok(());
            }
            Some(Ok(Message::Frame(_))) => {}
            Some(Err(e)) => {
                return Err(anyhow::anyhow!("erreur lecture WS: {}", e));
            }
            None => {
                return Err(anyhow::anyhow!("flux WS fermé par le serveur"));
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
// Tests unitaires (pas de réseau)
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nombre_de_topics_est_60() {
        assert_eq!(topics().len(), 60);
        // Tous les topics sont bien formés.
        assert!(topics().iter().all(|t| t.starts_with("kline.")));
    }

    #[test]
    fn mapping_symboles_bybit_vers_db() {
        assert_eq!(bybit_vers_asset("BTCUSDT"), Some("BTC"));
        assert_eq!(bybit_vers_asset("DOGEUSDT"), Some("DOGE"));
        assert_eq!(bybit_vers_asset("XAUUSDT"), Some("XAUUSD"));
        assert_eq!(bybit_vers_asset("XAGUSDT"), Some("XAGUSD"));
        assert_eq!(bybit_vers_asset("EURUSDT"), None); // non suivi
        assert_eq!(bybit_vers_asset("MACHIN"), None);
    }

    #[test]
    fn mapping_intervals_bybit_vers_db() {
        assert_eq!(bybit_interval_vers_tf("1"), Some("M1"));
        assert_eq!(bybit_interval_vers_tf("5"), Some("M5"));
        assert_eq!(bybit_interval_vers_tf("15"), Some("M15"));
        assert_eq!(bybit_interval_vers_tf("60"), Some("H1"));
        assert_eq!(bybit_interval_vers_tf("D"), Some("D1"));
        assert_eq!(bybit_interval_vers_tf("240"), None); // H4 non couvert ici
        assert_eq!(bybit_interval_vers_tf("W"), None);
    }

    #[test]
    fn extraction_topic() {
        assert_eq!(extraire_topic("kline.15.XAUUSDT"), Some(("15", "XAUUSDT")));
        assert_eq!(extraire_topic("kline.D.BTCUSDT"), Some(("D", "BTCUSDT")));
        assert_eq!(extraire_topic("kline.1.DOGEUSDT"), Some(("1", "DOGEUSDT")));
        // Topics non-kline ou malformés.
        assert_eq!(extraire_topic("tickers.BTCUSDT"), None);
        assert_eq!(extraire_topic("kline."), None);
        assert_eq!(extraire_topic("kline.15."), None);
        assert_eq!(extraire_topic("autrechose"), None);
    }

    #[test]
    fn parsing_message_kline_confirmee() {
        // Exemple tiré de la spec Bybit (champ confirm: true).
        let message = r#"{
            "topic": "kline.15.XAUUSDT",
            "type": "snapshot",
            "data": [{
                "start": 1786521600,
                "end": 1786522500,
                "interval": "15",
                "open": "4409.66",
                "high": "4414.0",
                "low": "4409.45",
                "close": "4412.28",
                "volume": "114.787",
                "turnover": "505540.3",
                "confirm": true,
                "timestamp": 1786521700000
            }]
        }"#;
        match traiter_texte(message) {
            ActionWs::Bougies(bougies) => {
                assert_eq!(bougies.len(), 1, "une bougie attendue");
                let (asset, tf, c) = &bougies[0];
                assert_eq!(*asset, Asset::XAUUSD);
                assert_eq!(*tf, Timeframe::M15);
                assert_eq!(c.timestamp.timestamp(), 1786521600);
                assert!((c.open - 4409.66).abs() < 1e-6);
                assert!((c.high - 4414.0).abs() < 1e-6);
                assert!((c.low - 4409.45).abs() < 1e-6);
                assert!((c.close - 4412.28).abs() < 1e-6);
                assert!((c.volume - 114.787).abs() < 1e-6);
            }
            autre => panic!("attendu ActionWs::Bougies, obtenu {:?}", autre),
        }
    }

    #[test]
    fn parsing_message_kline_non_confirmee_est_ignoree() {
        // Bougie en cours (confirm: false) → ne doit jamais atteindre la DB.
        let message = r#"{
            "topic": "kline.1.BTCUSDT",
            "type": "delta",
            "data": [{
                "start": 100,
                "interval": "1",
                "open": "1.0",
                "high": "2.0",
                "low": "0.5",
                "close": "1.5",
                "volume": "10.0",
                "confirm": false
            }]
        }"#;
        assert!(matches!(traiter_texte(message), ActionWs::Ignorer));
    }

    #[test]
    fn parsing_melange_confirmees_et_non_confirmees() {
        let message = r#"{
            "topic": "kline.5.ETHUSDT",
            "data": [
                {"start": 1000, "interval": "5", "open": "10", "high": "11", "low": "9", "close": "10.5", "volume": "5", "confirm": true},
                {"start": 2000, "interval": "5", "open": "20", "high": "21", "low": "19", "close": "20.5", "volume": "6", "confirm": false}
            ]
        }"#;
        match traiter_texte(message) {
            ActionWs::Bougies(bougies) => {
                assert_eq!(bougies.len(), 1, "seule la bougie confirmée doit passer");
                let (asset, tf, _) = &bougies[0];
                assert_eq!(*asset, Asset::ETH);
                assert_eq!(*tf, Timeframe::M5);
            }
            autre => panic!("attendu ActionWs::Bougies, obtenu {:?}", autre),
        }
    }

    #[test]
    fn parsing_ping_applicatif_renvoie_pong() {
        assert!(matches!(traiter_texte(r#"{"op":"ping"}"#), ActionWs::Pong));
    }

    #[test]
    fn parsing_pong_et_ack_subscribe_ignores() {
        assert!(matches!(traiter_texte(r#"{"op":"pong"}"#), ActionWs::Ignorer));
        assert!(matches!(
            traiter_texte(r#"{"op":"subscribe","success":true}"#),
            ActionWs::Ignorer
        ));
    }

    #[test]
    fn parsing_message_non_json_ignore() {
        assert!(matches!(traiter_texte("not json {{"), ActionWs::Ignorer));
        assert!(matches!(traiter_texte(""), ActionWs::Ignorer));
    }

    #[test]
    fn parsing_symbole_non_suivi_ignore() {
        // Topic bien formé mais actif non couvert (ex. EURUSD n'existe pas en linear Bybit ici).
        let message = r#"{
            "topic": "kline.15.EURUSDT",
            "data": [{"start": 1, "interval": "15", "open": "1", "high": "1", "low": "1", "close": "1", "volume": "1", "confirm": true}]
        }"#;
        assert!(matches!(traiter_texte(message), ActionWs::Ignorer));
    }

    #[test]
    fn garde_anti_double_start() {
        // On manipule directement la statique pour ce test ; on la remet dans
        // son état initial ensuite afin de ne pas polluer les autres tests.
        let avant = BYBIT_WS_DEMARRE
            .compare_exchange(false, false, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok();
        // Premier « démarrage ».
        let premier = marquer_demarre();
        // Second appel doit être ignoré.
        let second = marquer_demarre();
        // Restauration.
        BYBIT_WS_DEMARRE.store(false, Ordering::SeqCst);
        // `avant` vaut true si la statique était bien à false au départ.
        assert!(avant, "la garde devait être à false au départ du test");
        assert!(premier, "le premier marquage doit renvoyer true");
        assert!(!second, "le second marquage doit renvoyer false");
    }
}
