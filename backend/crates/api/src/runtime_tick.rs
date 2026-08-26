//! Runtime tick — câblage du cœur temps réel (Phases 1-2 de la ROADMAP).
//!
//! Architecture (règle R4 : le chemin du signal ne traverse ni DB ni timer) :
//!
//! ```text
//! Bybit WS (klines, formation + confirmations)
//!     │  canal mpsc non borné
//!     ▼
//! engine::Runtime (en mémoire)
//!     ├─ bus_signaux  → (phase 2+) API WS, notifications
//!     └─ bus_bougies  → journal d'observation (Gate 1)
//! ```
//!
//! Phase 2.6 — SHADOW MODE : chaque couple (asset × TF) porte un moteur
//! v12 (`MoteurV12`). Ses signaux et événements live sont journalisés
//! (table `runtime_emissions`) mais NE déclenchent RIEN — ni Telegram, ni
//! table `signaux`. Le journal alimente le test de vérité (Gate 2).
//!
//! La config (assets × timeframes) est relue en DB toutes les 60 s —
//! activer un asset dans l'UI l'ajoute au runtime avec cold start (replay)
//! en moins d'une minute, sans redémarrage.

use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use common::{Asset, Timeframe};
use sqlx::Row;
use db::Database;
use engine::{EvenementPrix, Runtime};
use tokio::sync::mpsc;

/// Profondeur de replay adaptée au moteur v12 : ~7 jours d'historique par
/// TF (ATR14 stabilisé, pivots, PDH/PDL/PWH/PWL), plafonné.
/// Amorce MTF pour le runtime live — H1/H4/W1 + MN agrégée de D1 (600 bars
/// max par TF, échec DB → amorce vide : replay dégradé, jamais de panic).
/// Annonces tier 1 (impact High) des prochaines 24 h — pour le straddle.
/// Le moteur filtre par devise pertinente pour l'asset côté runtime.
async fn annonces_tier1(db: &db::Database) -> Vec<straddle::Annonce> {
    let Ok(rows) = db.lire_calendrier_cache(6 * 3600).await else {
        return Vec::new();
    };
    let maintenant = chrono::Utc::now().timestamp();
    rows.iter()
        .filter_map(|r| {
            let impact = r.get("impact").and_then(|v| v.as_str()).unwrap_or("");
            if impact != "High" {
                return None;
            }
            let dh = r.get("date_heure").and_then(|v| v.as_str())?;
            let ts = chrono::DateTime::parse_from_rfc3339(dh)
                .or_else(|_| {
                    chrono::DateTime::parse_from_rfc3339(&format!(
                        "{}:{}",
                        dh[..dh.len() - 2].to_string(),
                        &dh[dh.len() - 2..]
                    ))
                })
                .ok()?
                .timestamp();
            if ts <= maintenant || ts > maintenant + 7 * 24 * 3600 {
                return None;
            }
            Some(straddle::Annonce {
                ts,
                devise: r.get("devise").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                titre: r.get("titre").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            })
        })
        .collect()
}

async fn charger_amorce_mtf_runtime(db: &db::Database, asset: &Asset) -> smc::v12::AmorceMtf {
    use common::Timeframe;
    use smc::v12::{agreger_mensuel, AmorceMtf, BarInput};
    const MAX_BARS: i64 = 600;

    let vers_bars = |bougies: Vec<common::Candle>| -> Vec<BarInput> {
        bougies
            .into_iter()
            .map(|b| BarInput {
                timestamp: b.timestamp.timestamp(),
                open: b.open,
                high: b.high,
                low: b.low,
                close: b.close,
                volume: b.volume,
            })
            .collect()
    };
    let charger = |tf: Timeframe| async move {
        vers_bars(
            db.obtenir_bougies(asset, &tf, MAX_BARS)
                .await
                .unwrap_or_default(),
        )
    };
    let (h1, h4) = tokio::join!(charger(Timeframe::H1), charger(Timeframe::H4));
    // D1 profond (2000) : agrégée en MN pour la confluence +6.
    let d1 = vers_bars(
        db.obtenir_bougies(asset, &Timeframe::D1, 2000)
            .await
            .unwrap_or_default(),
    );
    let w1 = charger(Timeframe::W1).await;
    AmorceMtf {
        h1,
        h4,
        w1,
        mn: agreger_mensuel(&d1),
    }
}

fn barres_replay_v12(tf: Timeframe) -> i64 {
    (7 * 1440 / tf.minutes() as i64).clamp(60, 10_080)
}

/// Période de relecture de la config workers (assets × timeframes).
const RELECTURE_CONFIG_SEC: u64 = 60;

/// Garde anti-double-start (pattern identique aux autres workers).
static RUNTIME_DEMARRE: AtomicBool = AtomicBool::new(false);

/// Poignées du runtime, à conserver par l'appelant pour exposer les bus
/// (API WS, notifications, journal d'observation — consommées à partir de
/// la phase 1.5).
#[allow(dead_code)]
#[derive(Clone)]
pub struct PoigneesRuntime {
    pub bus_signaux: engine::BusSignaux,
    pub bus_bougies: engine::BusBougies,
    pub bus_evenements: engine::BusEvenements,
    /// Canal prix brut (EvenementPrix) — partagé avec le collecteur MT5.
    pub tx_prix: tokio::sync::mpsc::UnboundedSender<engine::types::EvenementPrix>,
}

/// Démarre le runtime tick : construit le runtime, lance la boucle de
/// consommation des événements prix, démarre le worker Bybit WS qui
/// l'alimente, et le journal d'observation (Gate 1). Non bloquant,
/// idempotent.
pub fn demarrer_runtime_tick(db: Arc<Database>) -> PoigneesRuntime {
    if !RUNTIME_DEMARRE.swap(true, Ordering::SeqCst) {
        let (tx, rx) = mpsc::unbounded_channel::<EvenementPrix>();
        let runtime = Runtime::nouveau();
        let poignees = PoigneesRuntime {
            bus_signaux: runtime.bus().clone(),
            bus_bougies: runtime.bus_bougies().clone(),
            bus_evenements: runtime.bus_evenements().clone(),
            tx_prix: tx.clone(),
        };
        // Phase 5 — collecteur MT5 : l'EA pousse ses bougies par ce canal.
        crate::mt5_collecteur::brancher_canal(tx.clone());

        // Le worker Bybit WS alimente le runtime (klines formation + confirm).
        data::bybit_ws::demarrer_worker_bybit(db.clone(), Some(tx));

        tokio::spawn(boucle_runtime(db.clone(), runtime, rx));
        // Signaux OFFICIELS : table signaux + Telegram (bus < 1 s).
        crate::signaux_officiels::demarrer(
            db.clone(),
            poignees.bus_signaux.clone(),
            poignees.bus_evenements.clone(),
        );
        tokio::spawn(journal_observation(db.clone(), poignees.bus_bougies.clone()));
        tokio::spawn(journal_emissions(db, poignees.bus_signaux.clone(), poignees.bus_evenements.clone()));

        tracing::info!("⚡ Runtime tick démarré (shadow mode : moteurs v12 par couple, journalisation seule — aucune action)");
        poignees
    } else {
        tracing::warn!("⚠️  Runtime tick déjà démarré — second appel ignoré");
        // Les bus sont des poignées broadcast clonables : en renvoyer des
        // neuves serait mensonger ; on ne peut pas récupérer les originales
        // sans état partagé — le second appel est une erreur de câblage.
        PoigneesRuntime {
            bus_signaux: engine::BusSignaux::nouveau(),
            bus_bougies: engine::BusBougies::nouveau(),
            bus_evenements: engine::BusEvenements::nouveau(),
            tx_prix: tokio::sync::mpsc::unbounded_channel().0,
        }
    }
}

/// Journal d'observation (mode shadow) : chaque bougie clôturée par le
/// runtime est persistée avec son mode de clôture. La comparaison avec les
/// bougies officielles se fait à la demande (GET /api/runtime/concordance).
async fn journal_observation(db: Arc<Database>, bus: engine::BusBougies) {
    let mut rx = bus.abonner();
    tracing::info!("📓 Journal d'observation du runtime actif (table runtime_observation)");
    loop {
        match rx.recv().await {
            Ok(b) => {
                if let Err(e) = db
                    .inserer_observation_runtime(
                        &b.asset,
                        &b.tf,
                        &b.bougie,
                        b.mode.libelle(),
                        chrono::Utc::now(),
                    )
                    .await
                {
                    tracing::warn!(
                        "Journal observation: erreur DB {} {} ts={} : {}",
                        b.asset.as_str(),
                        b.tf.as_str(),
                        b.bougie.timestamp.timestamp(),
                        e
                    );
                }
            }
            Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                tracing::warn!("Journal observation: {} bougies sautées (lecteur lent)", n);
            }
            Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                tracing::error!("Journal observation: bus fermé — writer arrêté");
                return;
            }
        }
    }
}

/// Journal des émissions LIVE du runtime (shadow mode 2.6) : chaque signal
/// et chaque événement lifecycle publiés sur les bus est persisté — c'est
/// la matière brute du test de vérité (Gate 2). Aucune action n'en découle.
async fn journal_emissions(
    db: Arc<Database>,
    bus_signaux: engine::BusSignaux,
    bus_evenements: engine::BusEvenements,
) {
    let mut rx_signaux = bus_signaux.abonner();
    let mut rx_evenements = bus_evenements.abonner();
    tracing::info!("📓 Journal des émissions live actif (table runtime_emissions)");

    loop {
        tokio::select! {
            peut_etre = rx_signaux.recv() => {
                match peut_etre {
                    Ok(s) => {
                        let tps = serde_json::to_string(&s.take_profits).unwrap_or_default();
                        if let Err(e) = db.inserer_emission_signal(
                            &s.moteur, s.asset.as_str(), s.tf.as_str(),
                            format!("{:?}", s.direction), s.prix_entree, s.stop_loss,
                            &tps, s.score, &s.raison, s.debut_barre, s.emis_le,
                        ).await {
                            tracing::warn!("Journal émissions (signal): {}", e);
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!("Journal émissions: {} signaux sautés", n);
                    }
                    Err(_) => return,
                }
            }
            peut_etre = rx_evenements.recv() => {
                match peut_etre {
                    Ok(e) => {
                        if let Err(err) = db.inserer_emission_evenement(
                            &e.moteur, e.asset.as_str(), e.tf.as_str(),
                            &e.cle_trade, format!("{:?}", e.evenement), &e.detail,
                            e.prix, e.debut_barre, e.emis_le,
                        ).await {
                            tracing::warn!("Journal émissions (événement): {}", err);
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!("Journal émissions: {} événements sautés", n);
                    }
                    Err(_) => return,
                }
            }
        }
    }
}

/// Boucle principale du runtime : consommation des événements prix +
/// resynchronisation périodique de la config (nouveaux assets UI).
async fn boucle_runtime(
    db: Arc<Database>,
    mut runtime: Runtime,
    mut rx: mpsc::UnboundedReceiver<EvenementPrix>,
) {
    // Watcher alertes prix (cache 60 s, vérifié à chaque prix live).
    let mut cache_alertes = crate::alertes_prix::cache_vide();
    cache_alertes.recharger(&db).await;
    // Cold start initial.
    synchroniser_config(&db, &mut runtime).await;

    let mut tick_config = tokio::time::interval(Duration::from_secs(RELECTURE_CONFIG_SEC));
    tick_config.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            biased; // les événements prix passent toujours en premier
            peut_etre = rx.recv() => {
                if let Some(ev) = peut_etre {
                    crate::alertes_prix::verifier(&db, &mut cache_alertes, &ev).await;
                    runtime.traiter_evenement(ev);
                }
            }
            _ = tick_config.tick() => {
                synchroniser_config(&db, &mut runtime).await;
                cache_alertes.recharger(&db).await;
            }
        }
    }
}

/// Resynchronise les couples (asset × TF) du runtime avec la config DB.
async fn synchroniser_config(db: &Arc<Database>, runtime: &mut Runtime) {
    let assets = assets_runtime(db).await;
    let timeframes = data::worker_config::lire_timeframes(db).await;
    if assets.is_empty() || timeframes.is_empty() {
        tracing::debug!("Runtime tick: aucun asset/timeframe configuré — synchronisation vide");
        return;
    }

    let mt5_ids: HashSet<String> = assets_mt5(db).await;
    let cibles: HashSet<(Asset, Timeframe)> = assets
        .iter()
        .flat_map(|a| {
            // MT5 v12 (XAU/XAG) : tous les TF configurés — comme Bybit.
            // MT5 straddle-seul (NAS/SP/DAX) : M1 uniquement.
            let tfs: Vec<Timeframe> = if mt5_ids.contains(a.as_str()) && !MT5_V12.contains(&a.as_str()) {
                vec![common::Timeframe::M1]
            } else {
                timeframes.clone()
            };
            tfs.into_iter().map(move |tf| (a.clone(), tf))
        })
        .collect();

    // Retraits (asset décoché ou TF retiré dans l'UI).
    for cle in runtime.cles() {
        if !cibles.contains(&cle) {
            runtime.retirer(cle.0.clone(), cle.1);
            tracing::info!("Runtime tick: {} {} retiré (config DB)", cle.0.as_str(), cle.1.as_str());
        }
    }

    // Ajouts avec cold start (replay).
    let actuelles: HashSet<(Asset, Timeframe)> = runtime.cles().into_iter().collect();
    let mut ajouts = 0;
    for (asset, tf) in &cibles {
        if actuelles.contains(&(asset.clone(), *tf)) {
            continue;
        }
        // Phase 5.2 — MT5 v12 (XAU/XAG) : moteurs SMC complets sur prix Axi,
        // replays depuis l'historique EA (garde ~1 min après boot), amorce
        // MTF incluse (W1/D1 Axi). Straddle XAU M1 (annonces US, acté).
        if mt5_ids.contains(asset.as_str()) && MT5_V12.contains(&asset.as_str()) {
            if !crate::mt5_collecteur::historique_mt5_pret(db, asset.as_str(), *tf).await {
                tracing::info!(
                    "Runtime tick: {} {} MT5 en attente d'historique Axi (l'EA le pousse)",
                    asset.as_str(),
                    tf.as_str()
                );
                continue;
            }
            let amorce = charger_amorce_mtf_runtime(db, asset).await;
            let mut moteurs: Vec<Box<dyn engine::Engine>> = vec![Box::new(
                engine_v12::MoteurV12::nouveau(asset.clone(), *tf)
                    .avec_amorce(amorce)
                    // Décision 26/08 (étude comparatif_be) : BE forcé sur BOS
                    // opposé supprimé (+36R, R/trade +67 % vs Pine Classique).
                    .avec_mode_be_force(smc::v12::lifecycle::ModeBeForce::Supprime),
            )];
            if *tf == common::Timeframe::M1 && asset.as_str() == "XAUUSD" {
                let annonces: Vec<straddle::Annonce> = annonces_tier1(db)
                    .await
                    .into_iter()
                    .filter(|a| a.devise == "USD")
                    .collect();
                let p = db::strategies_params::lire_straddle_params(db.pool()).await;
                moteurs.push(Box::new(
                    straddle::StraddleEngine::nouveau(asset.clone(), *tf)
                        .avec_params(straddle::ParamsStraddle {
                            sl_atr: p.sl_mult,
                            trailing_r: p.trailing_r,
                            placement_avant_sec: p.placement_sec,
                            ..Default::default()
                        })
                        .avec_atr_h1(crate::straddle_atr::atr_h1(db, asset.as_str()).await)
                        .avec_annonces(annonces),
                ));
            }
            runtime.enregistrer(asset.clone(), *tf, moteurs);
            ajouts += 1;
            continue;
        }

        // Phase 5 — indices MT5 : moteur STRADDLE seul sur M1 (périmètre
        // acté : NAS100/SP500 sur annonces US, DAX sur ouverture européenne
        // 9h Paris). Warm-up ATR ~15 min en live.
        if mt5_ids.contains(asset.as_str()) {
            let annonces: Vec<straddle::Annonce> = if asset.as_str() == "DAX" {
                crate::mt5_collecteur::annonces_ouverture_europeenne()
            } else {
                annonces_tier1(db)
                    .await
                    .into_iter()
                    .filter(|a| a.devise == "USD")
                    .collect()
            };
            let p = db::strategies_params::lire_straddle_params(db.pool()).await;
            let params = straddle::ParamsStraddle {
                sl_atr: p.sl_mult,
                trailing_r: p.trailing_r,
                placement_avant_sec: p.placement_sec,
                ..Default::default()
            };
            tracing::info!(
                "Runtime tick: {} M1 moteur straddle armé (MT5 — {} annonce(s), R={:.2}×ATR)",
                asset.as_str(),
                annonces.len(),
                params.sl_atr
            );
            runtime.enregistrer(
                asset.clone(),
                *tf,
                vec![Box::new(
                    straddle::StraddleEngine::nouveau(asset.clone(), *tf)
                        .avec_params(params)
                        .avec_atr_h1(crate::straddle_atr::atr_h1(db, asset.as_str()).await)
                        .avec_annonces(annonces),
                )],
            );
            ajouts += 1;
            continue;
        }

        // Shadow mode (2.6) : moteur v12 par couple (couverture M1→D1,
        // propriétaire) — émissions journalisées, sans action. Amorce MTF
        // (H1/H4/W1 + MN) : sans elle, confluences W1 (+5) / MN (+6) froides.
        let amorce = charger_amorce_mtf_runtime(db, asset).await;
        // Phase 3.1 — plugin STRADDLE sur M1/M5 (news trading) : calendrier
        // tier 1 amorcé depuis le cache (jamais de DB dans le moteur, R4).
        let mut moteurs: Vec<Box<dyn engine::Engine>> = vec![Box::new(
            engine_v12::MoteurV12::nouveau(asset.clone(), *tf)
                .avec_amorce(amorce)
                .avec_mode_be_force(smc::v12::lifecycle::ModeBeForce::Supprime),
        )];
        // Étape 4 — verticale Straddle : périmètre acté = XAU + BTC sur
        // annonces US fortes (Bybit alimente ces deux-là en temps réel).
        // NAS100/SP500 (annonces US) et DAX (ouverture européenne 9h Paris)
        // attendent le branchement MT5 (phase 5) — moteurs prêts, pas armés.
        if matches!(tf, common::Timeframe::M1)
            && matches!(asset.as_str(), "XAUUSD" | "BTC")
        {
            let annonces: Vec<straddle::Annonce> = annonces_tier1(db)
                .await
                .into_iter()
                .filter(|a| a.devise == "USD")
                .collect();
            // Audit étape 2 : le moteur lisait des constantes — désormais
            // branché sur la carte Paramètres › Straddle (table DB).
            let p = db::strategies_params::lire_straddle_params(db.pool()).await;
            let params = straddle::ParamsStraddle {
                sl_atr: p.sl_mult,
                trailing_r: p.trailing_r,
                placement_avant_sec: p.placement_sec,
                ..Default::default()
            };
            tracing::info!(
                "Runtime tick: {} {} moteur straddle armé ({} annonce(s) US à venir, R={:.2}×ATR, T-{:.0}s, trailing {:.1}R)",
                asset.as_str(),
                tf.as_str(),
                annonces.len(),
                params.sl_atr,
                params.placement_avant_sec,
                params.trailing_r,
            );
            moteurs.push(Box::new(
                straddle::StraddleEngine::nouveau(asset.clone(), *tf)
                    .avec_params(params)
                    .avec_atr_h1(crate::straddle_atr::atr_h1(db, asset.as_str()).await)
                    .avec_annonces(annonces),
            ));
        }
        runtime.enregistrer(asset.clone(), *tf, moteurs);
        // Backfill automatique : comble les trous (nuits, week-ends, pannes)
        // via le REST Bybit avant le cold start — comme TradingView.
        match data::backfill::combler_historique(db, asset.clone(), *tf).await {
            Ok(n) if n > 0 => {
                tracing::info!(
                    "Runtime tick: {} {} backfill : {} bougies récupérées",
                    asset.as_str(),
                    tf.as_str(),
                    n
                )
            }
            Ok(_) => {}
            Err(e) => {
                tracing::warn!(
                    "Runtime tick: {} {} backfill échoué ({}) — replay sur l'existant",
                    asset.as_str(),
                    tf.as_str(),
                    e
                )
            }
        }
        let profondeur = barres_replay_v12(*tf);
        if let Ok(bougies) = db.obtenir_bougies(asset, tf, profondeur).await {
            let debut = std::time::Instant::now();
            let historique = runtime.rejouer(asset.clone(), *tf, &bougies);
            // Réconciliation : les clôtures survenues PENDANT la fenêtre de
            // replay referment leurs lignes en base (trade ouvert avant un
            // arrêt, clôturé pendant). Pas de nouveaux signaux, pas de
            // Telegram — la clé stable (open_ts) fait la correspondance.
            let nb_reconciliees = reconcilier_clotures(db, &historique.evenements).await;
            if nb_reconciliees > 0 {
                tracing::info!(
                    "Runtime tick: {} {} réconciliation replay : {} clôture(s) appliquée(s)",
                    asset.as_str(),
                    tf.as_str(),
                    nb_reconciliees
                );
            }
            tracing::info!(
                "Runtime tick: {} {} moteur v12 armé (replay {} bougies, {} signaux historiques, {:?})",
                asset.as_str(),
                tf.as_str(),
                bougies.len(),
                historique.signaux.len(),
                debut.elapsed()
            );
        }
        ajouts += 1;
    }
    if ajouts > 0 {
        tracing::info!(
            "Runtime tick: {} nouveau(x) couple(s) — total {}",
            ajouts,
            runtime.cles().len()
        );
    }
}

/// Réconciliation du replay : applique en base les clôtures survenues pendant
/// la fenêtre rejouée (trades ouverts avant un arrêt). Silencieux — pas de
/// Telegram, pas de nouveaux signaux. Retourne le nombre de lignes fermées.
async fn reconcilier_clotures(
    db: &Arc<Database>,
    evenements: &[engine::EvenementTrade],
) -> u64 {
    let mut total = 0u64;
    for e in evenements {
        if !matches!(e.evenement, engine::TypeEvenementTrade::Cloture) {
            continue;
        }
        let verdict = e.detail.split('|').next().unwrap_or("Expire");
        let r = e
            .detail
            .split('|')
            .nth(1)
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);
        match db
            .fermer_signal_par_cle(&e.cle_trade, verdict, e.prix, r, e.emis_le.timestamp())
            .await
        {
            Ok(n) => total += n,
            Err(err) => tracing::warn!("Réconciliation replay ({}): {}", e.cle_trade, err),
        }
    }
    total
}

/// Actifs MT5 armés aussi en moteur v12 (calibration SMC dédiée). Les
/// indices MT5 restent straddle-seul (périmètre acté phase 5).
const MT5_V12: [&str; 2] = ["XAUUSD", "XAGUSD"];

/// Ids des actifs source MT5 actifs (armement straddle M1 seul).
async fn assets_mt5(db: &Arc<Database>) -> HashSet<String> {
    sqlx::query("SELECT id FROM assets WHERE source = 'mt5' AND actif = 1")
        .fetch_all(db.pool())
        .await
        .map(|rows| rows.iter().map(|r| r.get::<String, _>("id")).collect())
        .unwrap_or_default()
}

/// Assets Bybit actifs depuis la config DB (même source que le worker WS).
async fn assets_runtime(db: &Arc<Database>) -> Vec<Asset> {
    match db.lister_assets_worker().await {
        Ok(assets) => assets
            .into_iter()
            .filter(|a| a.actif && (a.source == "binance" || a.source == "mt5"))
            .filter_map(|a| Asset::try_from(a.id.as_str()).ok())
            .collect(),
        Err(e) => {
            tracing::warn!("Runtime tick: lecture DB des actifs impossible ({})", e);
            Vec::new()
        }
    }
}
