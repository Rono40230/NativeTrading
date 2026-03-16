use serde::{Deserialize, Serialize};

/// Force d'un signal : Faible (isolé), Moyen (confirmé), Fort (confluence ≥2)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NiveauForce {
    Faible,
    Moyen,
    Fort,
}

/// Direction du signal
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DirectionSignal {
    Bullish,
    Bearish,
    Neutre,
}

/// Signal généré par un indicateur technique
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalIndicateur {
    /// Timestamp UNIX (secondes)
    pub timestamp: i64,
    /// Source : "EMA" | "RSI" | "MACD" | "Bollinger" | "ATR"
    pub source: String,
    /// Identifiant du type de signal : "golden_cross", "survente_sortie", ...
    pub type_signal: String,
    pub direction: DirectionSignal,
    pub force: NiveauForce,
    /// Description lisible pour le trader
    pub description: String,
    /// Valeur de l'indicateur au moment du signal
    pub valeur: f64,
    /// Prix de clôture au moment du signal (base pour calcul SL/TP)
    pub prix_entree: f64,
}
