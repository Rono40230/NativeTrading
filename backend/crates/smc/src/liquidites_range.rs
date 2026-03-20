use common::Candle;
use serde::{Deserialize, Serialize};

use super::liquidites_tz::heure_paris;

// ─── Paramètres ────────────────────────────────────────────────────────────────

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
        Self {
            heure_debut: 20,
            heure_fin: 1,
            deviations_nb: 2,
        }
    }
}

// ─── Types de sortie ──────────────────────────────────────────────────────────

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

// ─── Détection ────────────────────────────────────────────────────────────────

/// Détecte les N derniers ranges de session Asie complets + la session en cours.
pub fn detecter_ranges_asie(
    bougies: &[Candle],
    params: ParamsRangeAsie,
    nb_sessions: usize,
) -> Vec<RangeAsie> {
    if bougies.is_empty() {
        return Vec::new();
    }

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
                debut_ts = ts;
                haut = b.high;
                bas = b.low;
            } else {
                haut = haut.max(b.high);
                bas = bas.min(b.low);
            }
            fin_ts = ts;
        } else if dans_session {
            dans_session = false;
            sessions.push(construire_range(
                debut_ts,
                fin_ts,
                haut,
                bas,
                params.deviations_nb,
            ));
            haut = f64::NEG_INFINITY;
            bas = f64::INFINITY;
        }
    }

    // Session en cours (pas encore clôturée)
    if dans_session && haut.is_finite() {
        sessions.push(construire_range(
            debut_ts,
            fin_ts,
            haut,
            bas,
            params.deviations_nb,
        ));
    }

    let skip = sessions.len().saturating_sub(nb_sessions);
    sessions.into_iter().skip(skip).collect()
}

fn construire_range(
    debut_ts: i64,
    fin_ts: i64,
    haut: f64,
    bas: f64,
    deviations_nb: usize,
) -> RangeAsie {
    let hauteur = haut - bas;
    let mut deviations = Vec::new();
    for n in 1..=deviations_nb {
        let nf = n as f64;
        deviations.push(DeviationAsie {
            prix: haut + nf * hauteur,
            direction: "H".into(),
            numero: n as u32,
        });
        deviations.push(DeviationAsie {
            prix: bas - nf * hauteur,
            direction: "L".into(),
            numero: n as u32,
        });
    }
    RangeAsie {
        timestamp_debut: debut_ts,
        timestamp_fin: fin_ts,
        haut,
        bas,
        deviations,
    }
}
