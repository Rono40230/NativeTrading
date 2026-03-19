use common::Candle;
use serde::{Deserialize, Serialize};

const TOLERANCE_PCT: f64 = 0.001;

// ─── Paramètres ────────────────────────────────────────────────────────────────

pub struct ParamsLiquidites {
    /// Nombre de bougies de chaque côté pour valider un swing (défaut 10)
    pub swing_lookback: usize,
    pub swings_actif: bool,
    pub sessions_actif: bool,
    pub session_asie: bool,
    pub session_london: bool,
    pub session_ny: bool,
    pub session_lc: bool,
    pub dwm_actif: bool,
    /// Nombre de jours H/L à afficher (défaut 2)
    pub dwm_nb_jours: usize,
}

impl Default for ParamsLiquidites {
    fn default() -> Self {
        Self {
            swing_lookback: 10,
            swings_actif: true,
            sessions_actif: true,
            session_asie: true,
            session_london: true,
            session_ny: true,
            session_lc: true,
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
    /// "swing" | "asie" | "london" | "ny" | "lc" | "daily"
    pub categorie: String,
    pub equal: bool,
    pub swepe: bool,
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn heure_utc(ts: i64) -> u32 {
    ((ts % 86400 + 86400) % 86400 / 3600) as u32
}

/// Session ICT (UTC). Mutuellement exclusive par priorité décroissante.
fn session_de(heure: u32) -> Option<&'static str> {
    if heure >= 15 && heure < 17 { Some("lc") }      // London Close 15h-17h UTC
    else if heure >= 12 && heure < 20 { Some("ny") }  // NY session 12h-20h UTC
    else if heure >= 7 && heure < 12 { Some("london") } // London Open 7h-12h UTC
    else if heure >= 22 || heure < 7 { Some("asie") } // Asia 22h-7h UTC
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
            let equal = bougies[i.saturating_sub(20)..i]
                .iter().any(|x| (x.high - b.high).abs() / b.high.max(f64::EPSILON) <= TOLERANCE_PCT);
            niveaux.push(NiveauLiquidite {
                prix: b.high, cote: "BSL".into(), categorie: "swing".into(),
                equal, swepe: est_sweep_haut(bougies, i + 1, b.high),
            });
        }
        // SSL : swing low
        if bougies[i - lookback..i].iter().all(|x| x.low >= b.low)
            && bougies[i + 1..=i + lookback].iter().all(|x| x.low >= b.low)
        {
            let equal = bougies[i.saturating_sub(20)..i]
                .iter().any(|x| (x.low - b.low).abs() / b.low.max(f64::EPSILON) <= TOLERANCE_PCT);
            niveaux.push(NiveauLiquidite {
                prix: b.low, cote: "SSL".into(), categorie: "swing".into(),
                equal, swepe: est_sweep_bas(bougies, i + 1, b.low),
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
                    "asie"   => params.session_asie,
                    "london" => params.session_london,
                    "ny"     => params.session_ny,
                    "lc"     => params.session_lc,
                    _ => false,
                };
                if actif {
                    let suivant = sess_fin + 1;
                    niveaux.push(NiveauLiquidite {
                        prix: sess_high, cote: "BSL".into(), categorie: cs.into(),
                        equal: false, swepe: est_sweep_haut(bougies, suivant, sess_high),
                    });
                    niveaux.push(NiveauLiquidite {
                        prix: sess_low, cote: "SSL".into(), categorie: cs.into(),
                        equal: false, swepe: est_sweep_bas(bougies, suivant, sess_low),
                    });
                }
                current_sess = sess;
                sess_high = b.high;
                sess_low = b.low;
                sess_fin = i;
            }
            (None, Some(_)) => {
                current_sess = sess;
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

    let mut jours: Vec<(i64, f64, f64, usize)> = Vec::new();
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
        jours.push((jour, b.high, b.low, i));
    }
    let n = jours.len().saturating_sub(1); // exclure le jour incomplet
    let debut = n.saturating_sub(nb_jours);
    for (_, high, low, fin_idx) in &jours[debut..n] {
        let suivant = fin_idx + 1;
        niveaux.push(NiveauLiquidite {
            prix: *high, cote: "BSL".into(), categorie: "daily".into(),
            equal: false, swepe: est_sweep_haut(bougies, suivant, *high),
        });
        niveaux.push(NiveauLiquidite {
            prix: *low, cote: "SSL".into(), categorie: "daily".into(),
            equal: false, swepe: est_sweep_bas(bougies, suivant, *low),
        });
    }
    niveaux
}

// ─── Point d'entrée public ────────────────────────────────────────────────────

pub fn detecter(bougies: &[Candle], params: ParamsLiquidites) -> Vec<NiveauLiquidite> {
    if bougies.is_empty() { return Vec::new(); }
    let prix_actuel = bougies.last().map(|b| b.close).unwrap_or(0.0);
    let mut niveaux: Vec<NiveauLiquidite> = Vec::new();

    if params.swings_actif {
        niveaux.extend(detecter_swings(bougies, params.swing_lookback));
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
