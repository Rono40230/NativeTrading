//! Moteur de génération automatique de signaux SMC.
//!
//! Boucle Tokio toutes les 5 minutes — analyse 13 assets × M5/M15.
//! Guard Kill Zone intégré dans `SmcDirectionalStrategy`.
//! Anti-doublon 30 min via requête DB avant insertion.
use common::{Asset, Candle, Signal, Timeframe};
use db::Database;
use indicators::calculer_atr;
use smc::scorer;
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

/// Assets analysés automatiquement
const ASSETS: &[Asset] = &[
    Asset::BTC,
    Asset::ETH,
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
        "🤖 Signal Engine démarré — cycle {}s | {} assets × {} TF",
        INTERVALLE_SECS,
        ASSETS.len(),
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
    for asset in ASSETS {
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

    // Enrichissement LLM optionnel — non bloquant (timeout 45s)
    let strategie_nom = enrichir_avec_ollama(asset, timeframe, &signal_strat, &bougies).await;

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

    db.inserer_signal(&signal).await?;

    tracing::info!(
        "✅ Signal {}/{} {:?} score={:.1} entry={:.4}",
        asset.as_str(),
        timeframe.as_str(),
        signal.direction,
        signal.score,
        signal.prix_entree
    );

    // Broadcast vers les WS abonnés (erreur ignorée si aucun abonné)
    let _ = tx.send(signal);

    Ok(())
}

/// Appelle Ollama pour confirmer/enrichir un signal SMC validé.
///
/// Timeout 45s — en cas de dépassement ou d'indisponibilité Ollama,
/// retourne `"SMC Directionnel"` pour conserver le signal pur.
async fn enrichir_avec_ollama(
    asset: &Asset,
    timeframe: &Timeframe,
    signal: &strategies::Signal,
    bougies: &[Candle],
) -> &'static str {
    // Recalcul ATR14 et score SMC pour contextualiser le prompt
    let atr_vals = calculer_atr(bougies, 14);
    let atr_val = atr_vals.last().copied().unwrap_or(0.0);
    let (score_total, kill_zone, sweep) = match scorer(bougies) {
        Some(s) => (s.total, s.kill_zone_active, s.sweep_detecte),
        None => (signal.confiance * 100.0, false, false),
    };
    let dir = format!("{:?}", signal.direction);

    let confirmation = tokio::time::timeout(
        Duration::from_secs(45),
        crate::ollama::confirmer_signal_smc(
            asset.as_str(),
            timeframe.as_str(),
            score_total,
            &dir,
            signal.prix_entree,
            signal.stop_loss,
            signal.take_profit,
            signal.confiance,
            atr_val,
            kill_zone,
            sweep,
        ),
    )
    .await;

    match confirmation {
        Ok(Some(raisonnement)) => {
            tracing::info!(
                "🤖 LLM confirmé {}/{}: {}",
                asset.as_str(),
                timeframe.as_str(),
                &raisonnement[..raisonnement.len().min(100)]
            );
            "SMC+IA"
        }
        Ok(None) => "SMC Directionnel",
        Err(_) => {
            tracing::warn!(
                "Timeout Ollama (45s) {}/{} — signal SMC conservé",
                asset.as_str(),
                timeframe.as_str()
            );
            "SMC Directionnel"
        }
    }
}
