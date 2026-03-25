//! Moteur de génération automatique de signaux SMC.
//!
//! Boucle Tokio toutes les 5 minutes — analyse 13 assets × M5/M15.
//! Guard Kill Zone intégré dans `SmcDirectionalStrategy`.
//! Anti-doublon 30 min via requête DB avant insertion.
use crate::signal_filtre::sauvegarder_signal_avec_filtre;
use common::{Asset, Signal, Timeframe};
use db::Database;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use strategies::smc_directional::SmcDirectionalStrategy;
use strategies::Strategy;
use tokio::sync::broadcast;

/// Intervalle entre chaque cycle d'analyse complet
const INTERVALLE_SECS: u64 = 300; // 5 minutes

/// Durée de la fenêtre anti-doublon (en minutes)
const DOUBLON_MINUTES: i64 = 30;

/// Assets analysés automatiquement — chargés dynamiquement depuis la DB
/// (les assets actifs = actif=1 dans la table assets)
/// Liste statique de fallback si la DB est indisponible
const ASSETS_FALLBACK: &[Asset] = &[
    Asset::BTC,
    Asset::ETH,
    Asset::SOL,
    Asset::BNB,
    Asset::XRP,
    Asset::ADA,
    Asset::DOGE,
    Asset::AVAX,
    Asset::LINK,
    Asset::DOT,
    Asset::XAUUSD,
    Asset::XAGUSD,
    Asset::EURUSD,
    Asset::GBPJPY,
    Asset::CADJPY,
    Asset::NZDJPY,
    Asset::USDCAD,
    Asset::USDJPY,
    Asset::DAX,
    Asset::NAS100,
    Asset::SP500,
];

/// Timeframes analysés automatiquement
const TIMEFRAMES: &[Timeframe] = &[Timeframe::M5, Timeframe::M15];

/// Moteur de génération automatique de signaux SMC.
///
/// Démarre une tâche Tokio en background et expose un canal broadcast
/// pour que les WebSocket clients reçoivent chaque nouveau signal.
pub struct SignalEngine {
    pub running: Arc<AtomicBool>,
    tx: broadcast::Sender<Signal>,
    /// Timestamp du prochain cycle (Unix seconds)
    pub prochain_cycle: Arc<std::sync::Mutex<i64>>,
}

impl SignalEngine {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(64);
        SignalEngine {
            running: Arc::new(AtomicBool::new(false)),
            tx,
            prochain_cycle: Arc::new(std::sync::Mutex::new(0)),
        }
    }

    /// Abonnement au canal broadcast — chaque signal généré est envoyé ici.
    pub fn abonner(&self) -> broadcast::Receiver<Signal> {
        self.tx.subscribe()
    }

    /// Démarrage de la boucle en background.
    /// Sans effet si déjà actif.
    pub fn demarrer(&self, db: Arc<Database>) {
        if self.running.swap(true, Ordering::SeqCst) {
            return; // Déjà actif
        }
        let running = self.running.clone();
        let prochain = self.prochain_cycle.clone();
        let tx = self.tx.clone();
        tokio::spawn(boucle_detection(running, prochain, db, tx));
    }

    /// Arrêt propre de la boucle.
    pub fn arreter(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    pub fn est_actif(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Timestamp Unix du prochain cycle d'analyse.
    pub fn ts_prochain_cycle(&self) -> i64 {
        *self
            .prochain_cycle
            .lock()
            .unwrap_or_else(|p| p.into_inner())
    }
}

// ── Boucle interne ───────────────────────────────────────────────────────────

async fn boucle_detection(
    running: Arc<AtomicBool>,
    prochain: Arc<std::sync::Mutex<i64>>,
    db: Arc<Database>,
    tx: broadcast::Sender<Signal>,
) {
    tracing::info!(
        "🤖 Signal Engine démarré — cycle {}s | assets dynamiques × {} TF",
        INTERVALLE_SECS,
        TIMEFRAMES.len()
    );

    let strategie = SmcDirectionalStrategy;

    while running.load(Ordering::SeqCst) {
        // Calcule le prochain cycle avant l'analyse
        let ts_debut = chrono::Utc::now().timestamp();
        {
            if let Ok(mut guard) = prochain.lock() {
                *guard = ts_debut + INTERVALLE_SECS as i64;
            }
        }

        analyser_tous_assets(&strategie, &db, &tx).await;

        // Attente fractionnée (5 s) pour permettre un arrêt rapide
        let steps = INTERVALLE_SECS / 5;
        for _ in 0..steps {
            if !running.load(Ordering::SeqCst) {
                break;
            }
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    }

    tracing::info!("🛑 Signal Engine arrêté");
}

async fn analyser_tous_assets(
    strategie: &SmcDirectionalStrategy,
    db: &Arc<Database>,
    tx: &broadcast::Sender<Signal>,
) {
    // Charge les assets actifs depuis la DB — fallback sur liste statique
    let assets_actifs = match db.lister_assets().await {
        Ok(rows) => rows
            .into_iter()
            .filter_map(|r| crate::utils::parse_asset(&r.id))
            .collect::<Vec<Asset>>(),
        Err(e) => {
            tracing::warn!(
                "Signal Engine — impossible de charger les assets DB: {} — fallback",
                e
            );
            ASSETS_FALLBACK.to_vec()
        }
    };

    tracing::debug!("Signal Engine — analyse {} assets", assets_actifs.len());

    for asset in &assets_actifs {
        for timeframe in TIMEFRAMES {
            if let Err(e) = analyser_asset(strategie, db, tx, asset, timeframe).await {
                tracing::warn!(
                    "Signal Engine — {}/{}: {}",
                    asset.as_str(),
                    timeframe.as_str(),
                    e
                );
            }
        }
    }
}

async fn analyser_asset(
    strategie: &SmcDirectionalStrategy,
    db: &Arc<Database>,
    tx: &broadcast::Sender<Signal>,
    asset: &Asset,
    timeframe: &Timeframe,
) -> common::Result<()> {
    let bougies = db.obtenir_bougies(asset, timeframe, 200).await?;
    if bougies.len() < 30 {
        return Ok(());
    }

    // La stratégie inclut déjà les guards Kill Zone et Sweep
    let signal_strat = match strategie.analyze(&bougies)? {
        Some(s) => s,
        None => return Ok(()),
    };

    // Anti-doublon 30 min — évite les signaux redondants
    if db
        .signal_recent_existe(asset, timeframe, DOUBLON_MINUTES)
        .await?
    {
        tracing::debug!("Doublon ignoré {}/{}", asset.as_str(), timeframe.as_str());
        return Ok(());
    }

    let mut tp_list = vec![signal_strat.take_profit];
    if let Some(tp2) = signal_strat.take_profit_2 {
        tp_list.push(tp2);
    }
    if let Some(tp3) = signal_strat.take_profit_3 {
        tp_list.push(tp3);
    }

    // Contexte historique : 5 derniers signaux de cet asset → nourrit le LLM
    let historique_raw = db.obtenir_contexte_llm(asset.as_str(), 5).await;
    let contexte = crate::ollama::formater_contexte_historique(
        asset.as_str(),
        "SMC Directionnel",
        &historique_raw,
    );

    // Enrichissement LLM optionnel — délégué au module ollama
    let strategie_nom = crate::ollama::enrichir_signal_avec_ollama(
        asset.as_str(),
        timeframe.as_str(),
        &signal_strat,
        &bougies,
        &contexte,
    )
    .await;

    let signal = Signal::nouveau(
        asset.clone(),
        *timeframe,
        signal_strat.direction,
        signal_strat.confiance * 100.0,
        signal_strat.prix_entree,
        signal_strat.stop_loss,
        tp_list,
        strategie_nom,
    );

    // ── Filtre LLM pré-sauvegarde (conviction ≥ 65) ─────────────────────────
    let atr_vals = indicators::calculer_atr(&bougies, 14);
    let atr_now = atr_vals.last().copied().unwrap_or(0.0);
    let atr_moyen = if atr_vals.len() >= 14 {
        atr_vals[atr_vals.len().saturating_sub(14)..].iter().sum::<f64>() / 14.0
    } else {
        atr_now
    };
    let atr_ratio = if atr_moyen > 0.0 { atr_now / atr_moyen } else { 1.0 };

    let rsi_vals = indicators::calculer_rsi(&bougies, 14);
    let rsi = rsi_vals.last().copied().unwrap_or(50.0);

    let (score_smc, kill_zone, sweep) = match smc::scorer(&bougies) {
        Some(s) => (s.total, s.kill_zone_active, s.sweep_detecte),
        None => (signal_strat.confiance * 100.0, false, false),
    };

    let historique_smc = db.obtenir_historique_smc(asset.as_str(), 10).await;
    let historique_filtre: Vec<crate::ollama::smc_filtre::HistoriqueSMCSignal> = historique_smc
        .into_iter()
        .map(|(direction, tf, score, statut)| crate::ollama::smc_filtre::HistoriqueSMCSignal {
            direction,
            timeframe: tf,
            score,
            statut,
        })
        .collect();

    let candidat = crate::ollama::smc_filtre::SignalSMCCandidat {
        asset: asset.as_str().to_string(),
        timeframe: timeframe.as_str().to_string(),
        direction: format!("{:?}", signal_strat.direction),
        score_smc,
        confiance_ml: signal_strat.confiance,
        prix_entree: signal_strat.prix_entree,
        stop_loss: signal_strat.stop_loss,
        tp1: signal_strat.take_profit,
        atr14: atr_now,
        atr_ratio,
        rsi,
        kill_zone_active: kill_zone,
        sweep_detecte: sweep,
    };

    sauvegarder_signal_avec_filtre(db, tx, &signal, asset, timeframe, &candidat, &historique_filtre).await
}
