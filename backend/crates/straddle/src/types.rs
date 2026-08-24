//! Types du straddle — annonces injectées + paramètres (définition étape 4).
//!
//! Mécanique propriétaire actée 24/08 : deux jambes au MÊME prix E (prix à
//! T-10 s), armées dès la pose — premier franchissement remplit la jambe
//! dans son sens, l'autre est annulée (OCO). SL = E ∓ 1R ; TP1 = 1R (BE à
//! l'entrée) ; TP2 = 2R (BE à TP1 + déclenchement du trailing stop au tick,
//! jamais vers l'arrière) ; TP3 supprimé. Time-stop + expiration.

/// Annonce économique tier 1 (injectée par le runtime depuis le calendrier).
#[derive(Debug, Clone)]
pub struct Annonce {
    /// Epoch secondes de l'annonce (heure exacte, ex : 08:30 ET).
    pub ts: i64,
    /// Devise concernée (ex : "USD") — le runtime filtre par asset.
    pub devise: String,
    /// Libellé (NFP, CPI, FOMC…).
    pub titre: String,
}

/// Paramètres du straddle (défauts = définition canonique / carte params).
#[derive(Debug, Clone)]
pub struct ParamsStraddle {
    /// Début de la fenêtre de préparation AVANT l'annonce (minutes) — le
    /// range sert d'observation (ATR, journal) ; les ordres n'en dépendent plus.
    pub range_avant_min: i64,
    /// Placement ET armement des 2 jambes AVANT l'annonce (SECONDES).
    /// Défaut 10 (décision propriétaire 24/08 — réglable dans la carte).
    pub placement_avant_sec: i64,
    /// R, risque unitaire = distance du SL, en × ATR14 (défaut 0,5).
    pub sl_atr: f64,
    /// Distance du trailing stop en × R — actif dès TP2 touché, suit le prix
    /// AU TICK, jamais vers l'arrière (défaut 1,0).
    pub trailing_r: f64,
    /// Time-stop après le fill (minutes) — canonique : 60.
    pub time_stop_min: i64,
    /// Annulation des deux ordres si aucun fill (minutes après l'annonce).
    pub expiration_min: i64,
}

impl Default for ParamsStraddle {
    fn default() -> Self {
        Self {
            range_avant_min: 30,
            placement_avant_sec: 10,
            sl_atr: 0.5,
            trailing_r: 1.0,
            time_stop_min: 60,
            expiration_min: 30,
        }
    }
}
