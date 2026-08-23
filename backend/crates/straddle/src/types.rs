//! Types du straddle — annonces injectées + paramètres.

/// Annonce économique tier 1 (injectée par le runtime depuis le calendrier).
#[derive(Debug, Clone)]
pub struct Annonce {
    /// Epoch secondes de l'annonce (heure exacte, ex : 08:30 ET).
    pub ts: i64,
    /// Devise concernée (ex : "USD") — le runtime filtrie par asset.
    pub devise: String,
    /// Libellé (NFP, CPI, FOMC…).
    pub titre: String,
}

/// Paramètres du straddle (défauts = définition canonique / params DB).
#[derive(Debug, Clone)]
pub struct ParamsStraddle {
    /// Début de la fenêtre de range AVANT l'annonce (minutes).
    pub range_avant_min: i64,
    /// Placement des ordres AVANT l'annonce (minutes) — fin du range.
    pub placement_avant_min: i64,
    /// Offset des stops au-delà du range, en × ATR14.
    pub offset_atr: f64,
    /// SL en × ATR14 (défaut DB : 0,5).
    pub sl_atr: f64,
    /// TP1/2/3 en × ATR14 (défauts DB : 1,5 / 2,5 / 5,0).
    pub tp1_atr: f64,
    pub tp2_atr: f64,
    pub tp3_atr: f64,
    /// Time-stop après le fill (minutes) — canonique : 60.
    pub time_stop_min: i64,
    /// Annulation des deux ordres si aucun fill (minutes après l'annonce).
    pub expiration_min: i64,
}

impl Default for ParamsStraddle {
    fn default() -> Self {
        Self {
            range_avant_min: 30,
            placement_avant_min: 5,
            offset_atr: 0.25,
            sl_atr: 0.5,
            tp1_atr: 1.5,
            tp2_atr: 2.5,
            tp3_atr: 5.0,
            time_stop_min: 60,
            expiration_min: 30,
        }
    }
}
