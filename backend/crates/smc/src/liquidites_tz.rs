use common::Candle;

pub const TOLERANCE_PCT: f64 = 0.001;
pub const EQUAL_PCT: f64 = 0.003;

// ─── Helpers temps ────────────────────────────────────────────────────────────

pub fn heure_utc(ts: i64) -> u32 {
    (ts.rem_euclid(86400) / 3600) as u32
}

/// Décalage UTC → heure Paris (CEST +2 / CET +1) en secondes, calculé via le
/// helper unifié `common::time` (chrono-tz / base IANA) — DST automatique.
/// Conservation de la signature publique pour les appelants existants.
pub fn offset_paris(ts: i64) -> i32 {
    common::time::offset_paris_seconds(ts) as i32
}

/// Heure locale Paris (CET/CEST) depuis un timestamp Unix.
/// Délègue à `common::time::heure_paris` (DST automatique Europe/Paris).
pub fn heure_paris(ts: i64) -> u32 {
    common::time::heure_paris(ts)
}

/// Session ICT (UTC) : hors [7h-22h[ → "asie".
pub fn session_de(heure: u32) -> Option<&'static str> {
    if !(7..22).contains(&heure) {
        Some("asie")
    } else {
        None
    }
}

// ─── Sweep helpers ────────────────────────────────────────────────────────────

pub fn est_sweep_haut(bougies: &[Candle], depuis: usize, prix: f64) -> bool {
    bougies[depuis..]
        .iter()
        .any(|b| b.high > prix * (1.0 + TOLERANCE_PCT))
}

pub fn est_sweep_bas(bougies: &[Candle], depuis: usize, prix: f64) -> bool {
    bougies[depuis..]
        .iter()
        .any(|b| b.low < prix * (1.0 - TOLERANCE_PCT))
}
