//! Moteur de génération automatique de signaux SMC.
//!
//! Boucle Tokio toutes les 5 minutes — analyse 13 assets × M5/M15.
//! Guard Kill Zone intégré dans `SmcDirectionalStrategy`.
//! Anti-doublon 30 min via requête DB avant insertion.
use common::{Asset, Signal, Timeframe};
use db::Database;
use ml::PipelineML;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

/// Intervalle entre chaque cycle d'analyse complet
pub(crate) const INTERVALLE_SECS: u64 = 300; // 5 minutes

/// Durée de la fenêtre anti-doublon (en minutes)
pub(crate) const DOUBLON_MINUTES: i64 = 60;

/// Assets analysés automatiquement — chargés dynamiquement depuis la DB
/// (les assets actifs = actif=1 dans la table assets)
/// Liste statique de fallback si la DB est indisponible
pub(crate) const ASSETS_FALLBACK: &[Asset] = &[
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
pub(crate) const TIMEFRAMES: &[Timeframe] = &[Timeframe::M5, Timeframe::M15];

/// Moteur de génération automatique de signaux SMC.
///
/// Démarre une tâche Tokio en background et expose un canal broadcast
/// pour que les WebSocket clients reçoivent chaque nouveau signal.
pub struct SignalEngine {
    pub running: Arc<AtomicBool>,
    tx: broadcast::Sender<Signal>,
    /// Timestamp du prochain cycle (Unix seconds)
    pub prochain_cycle: Arc<std::sync::Mutex<i64>>,
    /// Score max des news actives (-1 = inconnu)
    pub score_news: Arc<std::sync::atomic::AtomicI32>,
    /// Valeur Fear & Greed en cours (-1 = inconnue)
    pub fg_valeur: Arc<std::sync::atomic::AtomicI32>,
}

impl SignalEngine {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(64);
        SignalEngine {
            running: Arc::new(AtomicBool::new(false)),
            tx,
            prochain_cycle: Arc::new(std::sync::Mutex::new(0)),
            score_news: Arc::new(std::sync::atomic::AtomicI32::new(-1)),
            fg_valeur: Arc::new(std::sync::atomic::AtomicI32::new(-1)),
        }
    }

    /// Abonnement au canal broadcast — chaque signal généré est envoyé ici.
    pub fn abonner(&self) -> broadcast::Receiver<Signal> {
        self.tx.subscribe()
    }

    /// Démarrage de la boucle en background.
    /// Sans effet si déjà actif.
    pub fn demarrer(&self, db: Arc<Database>, pipeline_ml: Arc<RwLock<PipelineML>>) {
        if self.running.swap(true, Ordering::SeqCst) {
            return; // Déjà actif
        }
        let running = self.running.clone();
        let prochain = self.prochain_cycle.clone();
        let tx = self.tx.clone();
        let score_news = self.score_news.clone();
        let fg_valeur = self.fg_valeur.clone();
        tokio::spawn(crate::signal_engine_analyse::boucle_detection(
            running,
            prochain,
            db,
            pipeline_ml,
            tx,
            score_news,
            fg_valeur,
        ));
    }

    /// Arrêt propre de la boucle.
    pub fn arreter(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    pub fn est_actif(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Mise à jour du contexte news (appelé par le handler /api/news/alertes).
    pub fn mettre_a_jour_score_news(&self, score: i32) {
        self.score_news
            .store(score, std::sync::atomic::Ordering::Relaxed);
    }

    /// Publie un signal dans le canal broadcast → WebSocket → modale d'alerte.
    /// À utiliser par toutes les stratégies pour un pipeline unifié.
    pub fn publier(&self, signal: Signal) {
        let _ = self.tx.send(signal);
    }

    /// Mise à jour du Fear & Greed (appelé par le handler /api/news/fear-greed).
    pub fn mettre_a_jour_fg(&self, valeur: i32) {
        self.fg_valeur
            .store(valeur, std::sync::atomic::Ordering::Relaxed);
    }

    /// Timestamp Unix du prochain cycle d'analyse.
    pub fn ts_prochain_cycle(&self) -> i64 {
        *self
            .prochain_cycle
            .lock()
            .unwrap_or_else(|p| p.into_inner())
    }
}
