use common::Candle;

pub const TOLERANCE_PCT: f64 = 0.001;
pub const EQUAL_PCT: f64 = 0.003;

// ─── Helpers temps ────────────────────────────────────────────────────────────

pub fn heure_utc(ts: i64) -> u32 {
    (ts.rem_euclid(86400) / 3600) as u32
}

/// Retourne le timestamp Unix (UTC, à `heure_utc_h`h) du dernier dimanche du mois.
fn dernier_dimanche(annee: i32, mois: u32, heure_utc_h: i64) -> i64 {
    let dernier_jour: u32 = match mois {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if (annee % 4 == 0 && annee % 100 != 0) || annee % 400 == 0 {
                29
            } else {
                28
            }
        }
        _ => 30,
    };
    let a = (14 - mois as i32) / 12;
    let y = annee + 4800 - a;
    let m = mois as i32 + 12 * a - 3;
    let jdn = dernier_jour as i64 + (153 * m as i64 + 2) / 5 + 365 * y as i64 + y as i64 / 4
        - y as i64 / 100
        + y as i64 / 400
        - 32045;
    let ts_dernier = (jdn - 2440588) * 86400 + heure_utc_h * 3600;
    let dow = (ts_dernier / 86400 + 4).rem_euclid(7);
    ts_dernier - dow * 86400
}

/// Décalage UTC → heure Paris selon DST européen (CEST +2 / CET +1).
fn offset_paris(ts: i64) -> i32 {
    let annee = (ts / 31_557_600 + 1970) as i32;
    let debut_ete = dernier_dimanche(annee, 3, 1);
    let fin_ete = dernier_dimanche(annee, 10, 1);
    if ts >= debut_ete && ts < fin_ete {
        2
    } else {
        1
    }
}

/// Heure locale Paris (CET/CEST) depuis un timestamp Unix.
pub fn heure_paris(ts: i64) -> u32 {
    ((ts / 3600 + offset_paris(ts) as i64).rem_euclid(24)) as u32
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
