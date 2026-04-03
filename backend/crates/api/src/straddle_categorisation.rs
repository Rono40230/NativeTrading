//! Catégorisation des pics de volatilité Straddle.
//!
//! Détermine la cause probable d'un pic ATR selon une priorité stricte :
//! annonce_high > annonce_medium > overlap_lnd_ny > ny_open > london_open
//!   > tokyo_open > creneau_recurrent > choc_isole
use chrono::{DateTime, Datelike, Timelike, Utc};
use db::straddle::StraddleCreneau;

// ── Enum catégorie ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CategoriePic {
    AnnonceHigh,
    AnnonceMedium,
    OverlapLndNy,
    NyOpen,
    LondonOpen,
    TokyoOpen,
    CreneauRecurrent,
    ChocIsole,
}

impl CategoriePic {
    pub fn as_str(&self) -> &'static str {
        match self {
            CategoriePic::AnnonceHigh => "annonce_high",
            CategoriePic::AnnonceMedium => "annonce_medium",
            CategoriePic::OverlapLndNy => "overlap_lnd_ny",
            CategoriePic::NyOpen => "ny_open",
            CategoriePic::LondonOpen => "london_open",
            CategoriePic::TokyoOpen => "tokyo_open",
            CategoriePic::CreneauRecurrent => "creneau_recurrent",
            CategoriePic::ChocIsole => "choc_isole",
        }
    }
}

// ── Résultat de la catégorisation ─────────────────────────────────────────────

pub struct ResultatCategorisation {
    pub categorie: CategoriePic,
    pub evenement_nom: Option<String>,
    pub evenement_devise: Option<String>,
    pub evenement_impact: Option<String>,
    pub minutes_avant_evt: Option<i64>,
    pub session_active: String,
}

// ── Fonction principale ───────────────────────────────────────────────────────

/// Catégorise un pic ATR selon la priorité définie.
///
/// `annonces` : événements HIGH/MEDIUM du calendrier économique en mémoire.
/// `creneaux_valides` : créneaux Straddle en DB avec statut "valide".
pub fn categoriser(
    annonces: &[serde_json::Value],
    maintenant: DateTime<Utc>,
    creneaux_valides: &[StraddleCreneau],
    asset: &str,
) -> ResultatCategorisation {
    let session = session_active(maintenant);
    let ts_now = maintenant.timestamp();

    // 1. Annonce HIGH impact dans les ±90 min
    for ann in annonces {
        if ann["impact"].as_str() != Some("High") {
            continue;
        }
        if let Some(minutes) = minutes_avant(ann, ts_now) {
            if (-15..=90).contains(&minutes) {
                return ResultatCategorisation {
                    categorie: CategoriePic::AnnonceHigh,
                    evenement_nom: ann["titre"].as_str().map(str::to_string),
                    evenement_devise: ann["devise"].as_str().map(str::to_string),
                    evenement_impact: Some("High".into()),
                    minutes_avant_evt: Some(minutes),
                    session_active: session,
                };
            }
        }
    }

    // 2. Annonce MEDIUM impact dans les ±60 min
    for ann in annonces {
        if ann["impact"].as_str() != Some("Medium") {
            continue;
        }
        if let Some(minutes) = minutes_avant(ann, ts_now) {
            if (-10..=60).contains(&minutes) {
                return ResultatCategorisation {
                    categorie: CategoriePic::AnnonceMedium,
                    evenement_nom: ann["titre"].as_str().map(str::to_string),
                    evenement_devise: ann["devise"].as_str().map(str::to_string),
                    evenement_impact: Some("Medium".into()),
                    minutes_avant_evt: Some(minutes),
                    session_active: session,
                };
            }
        }
    }

    let h = maintenant.hour();
    let m = maintenant.minute();
    let hm = h * 60 + m;

    // 3. Overlap London / New York (prioritaire sur les ouvertures isolées)
    // 13:00–16:00 UTC
    if (13 * 60..=16 * 60).contains(&hm) {
        return ResultatCategorisation {
            categorie: CategoriePic::OverlapLndNy,
            evenement_nom: Some("Overlap London/NY".into()),
            evenement_devise: None,
            evenement_impact: None,
            minutes_avant_evt: None,
            session_active: session,
        };
    }

    // 4. NY Open : 13:15–14:30 UTC (hors overlap déjà traité ci-dessus)
    if (13 * 60 + 15..=14 * 60 + 30).contains(&hm) {
        return ResultatCategorisation {
            categorie: CategoriePic::NyOpen,
            evenement_nom: Some("NY Open".into()),
            evenement_devise: None,
            evenement_impact: None,
            minutes_avant_evt: None,
            session_active: session.clone(),
        };
    }

    // 5. London Open : 07:00–08:30 UTC
    if (7 * 60..=8 * 60 + 30).contains(&hm) {
        return ResultatCategorisation {
            categorie: CategoriePic::LondonOpen,
            evenement_nom: Some("London Open".into()),
            evenement_devise: None,
            evenement_impact: None,
            minutes_avant_evt: None,
            session_active: session.clone(),
        };
    }

    // 6. Tokyo Open : 23:30–01:00 UTC (couvre minuit)
    if hm >= 23 * 60 + 30 || hm <= 60 {
        return ResultatCategorisation {
            categorie: CategoriePic::TokyoOpen,
            evenement_nom: Some("Tokyo Open".into()),
            evenement_devise: None,
            evenement_impact: None,
            minutes_avant_evt: None,
            session_active: session.clone(),
        };
    }

    // 7. Créneau récurrent en DB pour cet asset à cette heure
    let jour_courant = maintenant.weekday().num_days_from_monday() as i64;
    for c in creneaux_valides {
        if c.asset != asset {
            continue;
        }
        if let Some(jour) = c.jour_semaine {
            if jour != jour_courant {
                continue;
            }
        }
        if heure_dans_creneau(hm, &c.heure_debut, &c.heure_fin) {
            return ResultatCategorisation {
                categorie: CategoriePic::CreneauRecurrent,
                evenement_nom: Some(format!("Créneau {}–{}", c.heure_debut, c.heure_fin)),
                evenement_devise: None,
                evenement_impact: None,
                minutes_avant_evt: None,
                session_active: session,
            };
        }
    }

    // 8. Choc isolé — aucune cause identifiée
    ResultatCategorisation {
        categorie: CategoriePic::ChocIsole,
        evenement_nom: None,
        evenement_devise: None,
        evenement_impact: None,
        minutes_avant_evt: None,
        session_active: session,
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Session de marché active à un instant UTC donné.
pub fn session_active(dt: DateTime<Utc>) -> String {
    let hm = dt.hour() * 60 + dt.minute();
    if (13 * 60..=16 * 60).contains(&hm) {
        "Overlap".into()
    } else if (13 * 60 + 15..=21 * 60).contains(&hm) {
        "NewYork".into()
    } else if (7 * 60..=16 * 60).contains(&hm) {
        "London".into()
    } else if hm >= 23 * 60 + 30 || hm <= 8 * 60 {
        "Tokyo".into()
    } else {
        "Off".into()
    }
}

/// Distance en minutes entre maintenant et un événement calendrier.
/// Retourne None si le champ `date_heure` est absent ou non parsable.
fn minutes_avant(ann: &serde_json::Value, ts_now: i64) -> Option<i64> {
    let s = ann["date_heure"].as_str()?;
    let dt = chrono::DateTime::parse_from_rfc3339(s).ok()?;
    Some((dt.timestamp() - ts_now) / 60)
}

/// Vérifie si `hm` (heure×60 + minute) tombe dans le créneau "HH:MM"–"HH:MM".
fn heure_dans_creneau(hm: u32, debut: &str, fin: &str) -> bool {
    fn parse_hm(s: &str) -> u32 {
        let mut it = s.splitn(2, ':');
        let h: u32 = it.next().and_then(|v| v.parse().ok()).unwrap_or(0);
        let m: u32 = it.next().and_then(|v| v.parse().ok()).unwrap_or(0);
        h * 60 + m
    }
    let d = parse_hm(debut);
    let f = parse_hm(fin);
    if d <= f {
        hm >= d && hm <= f
    } else {
        // Créneau qui couvre minuit (ex: 23:00–01:00)
        hm >= d || hm <= f
    }
}
