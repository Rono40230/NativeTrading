//! KDJ/Halftrend Intraday (« 550 % ») — moteur de rejeu fidèle à l'étalon Pine.
//!
//! Étalon : `docs/reference/strategie_550_pourcent_v4.pine`
//! (MD5 `be3343edd854d0759c6ac23bb2ec2b69`).
//! Définition de référence (formules, exécution, pièges) :
//! `docs/reference/definition_kdj_halftrend.md` — toute divergence est un bug.
//!
//! Modèle d'exécution (piège 1 de la définition) : AUCUN ordre limit/stop —
//! signaux et crosses détectés à la clôture, exécution à l'open de la barre
//! suivante. Le P&L réel est `open(sortie) − open(entrée)`, jamais le niveau
//! TP/SL (qui ne sont que des seuils de détection).

pub mod mesures;
pub mod moteur;
pub mod moteur_live;

pub use mesures::{appliquer_frais, mesurer, Frais, Mesures};
pub use moteur::rejouer;
pub use moteur_live::{KdjEngine, NOM};

use chrono::{DateTime, Utc};

/// Paramètres de la stratégie — defaults = valeurs du screening phase A.
/// `adx_min` = filtre de tendance franche (scanner 7.E) : ADX(14) au
/// moment du signal ≥ seuil ; `None` = étalon strict, aucun filtre.
#[derive(Debug, Clone)]
pub struct ParamsKdj {
    /// Fenêtre high/low du KDJ (`period`).
    pub period: usize,
    /// Lissage K et D (`signal`).
    pub signal: usize,
    /// Fenêtre highma/lowma du HalfTrend (`Amplitude`).
    pub amplitude: usize,
    /// TP = RatioRisk × distance entrée→EMA200 (`RatioRisk`).
    pub ratio_risk: f64,
    /// Filtre ADX(14) minimum au signal (None = désactivé).
    pub adx_min: Option<f64>,
}

impl Default for ParamsKdj {
    fn default() -> Self {
        Self { period: 20, signal: 7, amplitude: 2, ratio_risk: 2.0, adx_min: None }
    }
}

/// Verdict de clôture — convention : si TP et SL sont crossés à la même
/// clôture, le verdict est `Tp` (ordre de l'expression `TPlong or StopLong`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Tp,
    Sl,
    Retournement,
    /// Position encore ouverte à la fin des données (exclue des stats).
    Ouvert,
}

impl Verdict {
    pub fn as_str(&self) -> &'static str {
        match self {
            Verdict::Tp => "TP",
            Verdict::Sl => "SL",
            Verdict::Retournement => "Retournement",
            Verdict::Ouvert => "Ouvert",
        }
    }
}

/// Un trade rejoué — niveaux et risque FIGÉS à la barre d'entrée (piège 3).
#[derive(Debug, Clone)]
pub struct TradeKdj {
    /// 1 = long, −1 = short.
    pub direction: i32,
    pub ts_entree: DateTime<Utc>,
    pub ts_sortie: Option<DateTime<Utc>>,
    /// open de la barre d'entrée.
    pub prix_entree: f64,
    /// open de la barre de sortie (None si `Ouvert`).
    pub prix_sortie: Option<f64>,
    /// Seuils de DÉTECTION (pas des prix de remplissage — piège 1).
    pub niveau_tp: f64,
    pub niveau_sl: f64,
    /// |E − EMA200_E| — l'unité de risque du trade.
    pub risque: f64,
    pub verdict: Verdict,
    /// R brut = dir × (open_sortie − E) / risque.
    pub r_brut: Option<f64>,
    /// R net après frais — rempli par `mesures::appliquer_frais`.
    pub r_net: Option<f64>,
}
