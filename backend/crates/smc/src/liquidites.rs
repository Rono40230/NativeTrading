use common::Candle;
use serde::{Deserialize, Serialize};

const TOLERANCE_PCT: f64 = 0.001;
/// Tolérance pour la détection Equal Highs/Lows (0.3% — proche de l'indicateur Kasper)
const EQUAL_PCT: f64 = 0.003;

// ─── Paramètres ────────────────────────────────────────────────────────────────

pub struct ParamsLiquidites {
    /// Nombre de bougies de chaque côté pour valider un swing (défaut 10)
    pub swing_lookback: usize,
    pub swings_actif: bool,
    pub sessions_actif: bool,
    pub session_asie: bool,
    pub dwm_actif: bool,
    /// Nombre de jours H/L à afficher (défaut 2)
    pub dwm_nb_jours: usize,
}

impl Default for ParamsLiquidites {
    fn default() -> Self {
        Self {
            swing_lookback: 50,
            swings_actif: true,
            sessions_actif: true,
            session_asie: true,
            dwm_actif: false,
            dwm_nb_jours: 2,
        }
    }
}

// ─── Type de sortie ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NiveauLiquidite {
    pub prix: f64,
    /// "BSL" (swing high / session high) ou "SSL" (swing low / session low)
    pub cote: String,
    /// "swing" | "asie" | "daily"
    pub categorie: String,
    pub equal: bool,
    pub swepe: bool,
    /// Unix secondes — bougie ou session de formation
    pub timestamp: i64,
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn heure_utc(ts: i64) -> u32 {
    (ts.rem_euclid(86400) / 3600) as u32
}

/// Convertit un timestamp Unix en timestamp du début du jour UTC.
fn debut_jour(ts: i64) -> i64 { ts - ts.rem_euclid(86400) }

/// Retourne le timestamp Unix (UTC, à `heure_utc`h) du dernier dimanche du mois.
fn dernier_dimanche(annee: i32, mois: u32, heure_utc_h: i64) -> i64 {
    let dernier_jour: u32 = match mois {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => if (annee % 4 == 0 && annee % 100 != 0) || annee % 400 == 0 { 29 } else { 28 },
        _ => 30,
    };
    // JDN du dernier jour du mois
    let a = (14 - mois as i32) / 12;
    let y = annee + 4800 - a;
    let m = mois as i32 + 12 * a - 3;
    let jdn = dernier_jour as i64 + (153 * m as i64 + 2) / 5
        + 365 * y as i64 + y as i64 / 4 - y as i64 / 100 + y as i64 / 400 - 32045;
    let ts_dernier = (jdn - 2440588) * 86400 + heure_utc_h * 3600;
    // Jour de semaine : 0 = dimanche (epoch = jeudi = 4)
    let dow = (ts_dernier / 86400 + 4).rem_euclid(7);
    ts_dernier - dow * 86400
}

/// Décalage UTC → heure Paris selon DST européen :
/// CEST (+2) du dernier dimanche de mars 01h00 UTC au dernier dimanche d'octobre 01h00 UTC.
fn offset_paris(ts: i64) -> i32 {
    // Année approximative (suffisant pour les transitions)
    let annee = (ts / 31_557_600 + 1970) as i32;
    let debut_ete = dernier_dimanche(annee, 3, 1);  // dernier dim mars 01:00 UTC
    let fin_ete   = dernier_dimanche(annee, 10, 1); // dernier dim oct  01:00 UTC
    if ts >= debut_ete && ts < fin_ete { 2 } else { 1 }
}

/// Heure locale Paris (CET/CEST) depuis un timestamp Unix.
fn heure_paris(ts: i64) -> u32 {
    ((ts / 3600 + offset_paris(ts) as i64).rem_euclid(24)) as u32
}

/// Session ICT (UTC). Mutuellement exclusive par priorité décroissante.
fn session_de(heure: u32) -> Option<&'static str> {
    if !(7..22).contains(&heure) { Some("asie") } // Asia 22h-7h UTC
    else { None }
}

fn est_sweep_haut(bougies: &[Candle], depuis: usize, prix: f64) -> bool {
    bougies[depuis..].iter().any(|b| b.high > prix * (1.0 + TOLERANCE_PCT))
}

fn est_sweep_bas(bougies: &[Candle], depuis: usize, prix: f64) -> bool {
    bougies[depuis..].iter().any(|b| b.low < prix * (1.0 - TOLERANCE_PCT))
}

// ─── Swings ───────────────────────────────────────────────────────────────────

fn detecter_swings(bougies: &[Candle], lookback: usize) -> Vec<NiveauLiquidite> {
    let mut niveaux = Vec::new();
    if bougies.len() < lookback * 2 + 1 { return niveaux; }

    for i in lookback..bougies.len().saturating_sub(lookback) {
        let b = &bougies[i];
        // BSL : swing high
        if bougies[i - lookback..i].iter().all(|x| x.high <= b.high)
            && bougies[i + 1..=i + lookback].iter().all(|x| x.high <= b.high)
        {
            let equal = bougies[i.saturating_sub(100)..i]
                .iter().any(|x| (x.high - b.high).abs() / b.high.max(f64::EPSILON) <= EQUAL_PCT);
            niveaux.push(NiveauLiquidite {
                prix: b.high, cote: "BSL".into(), categorie: "swing".into(),
                equal, swepe: est_sweep_haut(bougies, i + 1, b.high),
                timestamp: b.timestamp.timestamp(),
            });
        }
        // SSL : swing low
        if bougies[i - lookback..i].iter().all(|x| x.low >= b.low)
            && bougies[i + 1..=i + lookback].iter().all(|x| x.low >= b.low)
        {
            let equal = bougies[i.saturating_sub(100)..i]
                .iter().any(|x| (x.low - b.low).abs() / b.low.max(f64::EPSILON) <= EQUAL_PCT);
            niveaux.push(NiveauLiquidite {
                prix: b.low, cote: "SSL".into(), categorie: "swing".into(),
                equal, swepe: est_sweep_bas(bougies, i + 1, b.low),
                timestamp: b.timestamp.timestamp(),
            });
        }
    }
    niveaux
}

// ─── Sessions ─────────────────────────────────────────────────────────────────

fn detecter_sessions(bougies: &[Candle], params: &ParamsLiquidites) -> Vec<NiveauLiquidite> {
    let mut niveaux = Vec::new();
    let mut current_sess: Option<&'static str> = None;
    let mut sess_high = f64::NEG_INFINITY;
    let mut sess_low = f64::INFINITY;
    let mut sess_fin = 0usize;
    let mut sess_debut_ts: i64 = 0;

    for (i, b) in bougies.iter().enumerate() {
        let sess = session_de(heure_utc(b.timestamp.timestamp()));
        match (current_sess, sess) {
            (Some(cs), Some(s)) if cs == s => {
                sess_high = sess_high.max(b.high);
                sess_low = sess_low.min(b.low);
                sess_fin = i;
            }
            (Some(cs), _) => {
                let actif = match cs {
                    "asie" => params.session_asie,
                    _ => false,
                };
                if actif {
                    let suivant = sess_fin + 1;
                    niveaux.push(NiveauLiquidite {
                        prix: sess_high, cote: "BSL".into(), categorie: cs.into(),
                        equal: false, swepe: est_sweep_haut(bougies, suivant, sess_high),
                        timestamp: sess_debut_ts,
                    });
                    niveaux.push(NiveauLiquidite {
                        prix: sess_low, cote: "SSL".into(), categorie: cs.into(),
                        equal: false, swepe: est_sweep_bas(bougies, suivant, sess_low),
                        timestamp: sess_debut_ts,
                    });
                }
                current_sess = sess;
                sess_debut_ts = b.timestamp.timestamp();
                sess_high = b.high;
                sess_low = b.low;
                sess_fin = i;
            }
            (None, Some(_)) => {
                current_sess = sess;
                sess_debut_ts = b.timestamp.timestamp();
                sess_high = b.high;
                sess_low = b.low;
                sess_fin = i;
            }
            (None, None) => {}
        }
    }
    niveaux
}

// ─── Daily H/L ───────────────────────────────────────────────────────────────

fn detecter_daily(bougies: &[Candle], nb_jours: usize) -> Vec<NiveauLiquidite> {
    let mut niveaux = Vec::new();
    if bougies.is_empty() { return niveaux; }

    let mut jours: Vec<(i64, f64, f64, usize, i64)> = Vec::new();
    for (i, b) in bougies.iter().enumerate() {
        let jour = b.timestamp.timestamp() / 86400;
        if let Some(last) = jours.last_mut() {
            if last.0 == jour {
                last.1 = last.1.max(b.high);
                last.2 = last.2.min(b.low);
                last.3 = i;
                continue;
            }
        }
        jours.push((jour, b.high, b.low, i, b.timestamp.timestamp()));
    }
    let n = jours.len().saturating_sub(1); // exclure le jour incomplet
    let debut = n.saturating_sub(nb_jours);
    for (_, high, low, fin_idx, debut_ts) in &jours[debut..n] {
        let suivant = fin_idx + 1;
        niveaux.push(NiveauLiquidite {
            prix: *high, cote: "BSL".into(), categorie: "daily".into(),
            equal: false, swepe: est_sweep_haut(bougies, suivant, *high),
            timestamp: *debut_ts,
        });
        niveaux.push(NiveauLiquidite {
            prix: *low, cote: "SSL".into(), categorie: "daily".into(),
            equal: false, swepe: est_sweep_bas(bougies, suivant, *low),
            timestamp: *debut_ts,
        });
    }
    niveaux
}

// ─── Range Asie ───────────────────────────────────────────────────────────────

/// Paramètres pour le range de session Asie
pub struct ParamsRangeAsie {
    /// Heure Paris de début (CET/CEST auto-détecté)
    pub heure_debut: u32,
    /// Heure Paris de fin
    pub heure_fin: u32,
    /// Nombre de déviations (extensions) au-dessus/en-dessous du range (0 = aucune)
    pub deviations_nb: usize,
}

impl Default for ParamsRangeAsie {
    fn default() -> Self {
        Self { heure_debut: 20, heure_fin: 1, deviations_nb: 2 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RangeAsie {
    /// Unix secondes — première bougie de la session
    pub timestamp_debut: i64,
    /// Unix secondes — dernière bougie de la session (ou bougie courante si session en cours)
    pub timestamp_fin: i64,
    pub haut: f64,
    pub bas: f64,
    /// Déviations : (prix, direction "H"|"L", numéro 1..N)
    pub deviations: Vec<DeviationAsie>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviationAsie {
    pub prix: f64,
    pub direction: String, // "H" = au-dessus | "L" = en-dessous
    pub numero: u32,
}

/// Détecte les N derniers ranges de session Asie complets + la session en cours.
/// Retourne au maximum `nb_sessions` ranges.
pub fn detecter_ranges_asie(bougies: &[Candle], params: ParamsRangeAsie, nb_sessions: usize) -> Vec<RangeAsie> {
    if bougies.is_empty() { return Vec::new(); }

    let est_asie = |ts: i64| -> bool {
        let heure = heure_paris(ts);
        if params.heure_debut > params.heure_fin {
            heure >= params.heure_debut || heure < params.heure_fin
        } else {
            heure >= params.heure_debut && heure < params.heure_fin
        }
    };

    let mut sessions: Vec<RangeAsie> = Vec::new();
    let mut dans_session = false;
    let mut debut_ts: i64 = 0;
    let mut haut = f64::NEG_INFINITY;
    let mut bas = f64::INFINITY;
    let mut fin_ts: i64 = 0;

    for b in bougies.iter() {
        let ts = b.timestamp.timestamp();
        if est_asie(ts) {
            if !dans_session {
                dans_session = true;
                debut_ts = b.timestamp.timestamp();
                haut = b.high;
                bas = b.low;
            } else {
                haut = haut.max(b.high);
                bas = bas.min(b.low);
            }
            fin_ts = b.timestamp.timestamp();
        } else if dans_session {
            dans_session = false;
            let hauteur = haut - bas;
            let mut deviations = Vec::new();
            for n in 1..=params.deviations_nb {
                let nf = n as f64;
                deviations.push(DeviationAsie { prix: haut + nf * hauteur, direction: "H".into(), numero: n as u32 });
                deviations.push(DeviationAsie { prix: bas - nf * hauteur, direction: "L".into(), numero: n as u32 });
            }
            sessions.push(RangeAsie { timestamp_debut: debut_ts, timestamp_fin: fin_ts, haut, bas, deviations });
            haut = f64::NEG_INFINITY;
            bas = f64::INFINITY;
        }
    }

    // Session en cours (pas encore clôturée)
    if dans_session && haut.is_finite() {
        let hauteur = haut - bas;
        let mut deviations = Vec::new();
        for n in 1..=params.deviations_nb {
            let nf = n as f64;
            deviations.push(DeviationAsie { prix: haut + nf * hauteur, direction: "H".into(), numero: n as u32 });
            deviations.push(DeviationAsie { prix: bas - nf * hauteur, direction: "L".into(), numero: n as u32 });
        }
        sessions.push(RangeAsie { timestamp_debut: debut_ts, timestamp_fin: fin_ts, haut, bas, deviations });
    }

    // Conserver uniquement les N dernières sessions
    let skip = sessions.len().saturating_sub(nb_sessions);
    sessions.into_iter().skip(skip).collect()
}

// ─── Point d'entrée public ────────────────────────────────────────────────────

pub fn detecter(bougies: &[Candle], params: ParamsLiquidites) -> Vec<NiveauLiquidite> {
    if bougies.is_empty() { return Vec::new(); }
    let prix_actuel = bougies.last().map(|b| b.close).unwrap_or(0.0);
    let mut niveaux: Vec<NiveauLiquidite> = Vec::new();

    if params.swings_actif {
        niveaux.extend(detecter_swings(bougies, params.swing_lookback).into_iter().filter(|n| n.equal));
    }
    if params.sessions_actif {
        niveaux.extend(detecter_sessions(bougies, &params));
    }
    if params.dwm_actif {
        niveaux.extend(detecter_daily(bougies, params.dwm_nb_jours));
    }

    niveaux.sort_by(|a, b| {
        let da = (a.prix - prix_actuel).abs();
        let db = (b.prix - prix_actuel).abs();
        let sa = if a.swepe { da * 2.0 } else { da };
        let sb = if b.swepe { db * 2.0 } else { db };
        sa.partial_cmp(&sb).unwrap_or(std::cmp::Ordering::Equal)
    });
    niveaux.dedup_by(|a, b| {
        (a.prix - b.prix).abs() / b.prix.max(f64::EPSILON) < TOLERANCE_PCT * 2.0
    });
    niveaux.truncate(25);
    niveaux
}
