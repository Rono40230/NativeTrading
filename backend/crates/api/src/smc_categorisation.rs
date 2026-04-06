//! Catégorisation des signaux SMC selon la confluence d'indicateurs.
//!
//! Priorité décroissante :
//!   triple_confluence > ob_ifvg > ob_imbalance > ob_seul > fib_confluence > fib_seul > choc_isole
use chrono::{DateTime, Timelike, Utc};

// ── Enum catégorie ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub enum CategorieSmc {
    TripleConfluence,
    ObIfvg,
    ObImbalance,
    ObSeul,
    FibConfluence,
    FibSeul,
    ChocIsole,
}

impl CategorieSmc {
    pub fn as_str(&self) -> &'static str {
        match self {
            CategorieSmc::TripleConfluence => "triple_confluence",
            CategorieSmc::ObIfvg => "ob_ifvg",
            CategorieSmc::ObImbalance => "ob_imbalance",
            CategorieSmc::ObSeul => "ob_seul",
            CategorieSmc::FibConfluence => "fib_confluence",
            CategorieSmc::FibSeul => "fib_seul",
            CategorieSmc::ChocIsole => "choc_isole",
        }
    }
}

// ── Résultat de la catégorisation ─────────────────────────────────────────────

pub struct ResultatCategSmc {
    pub categorie: CategorieSmc,
    pub session_active: String,
}

// ── Fonction principale ───────────────────────────────────────────────────────

/// Catégorise un signal SMC selon les composants actifs.
///
/// Les seuils sont issus du scoring de la crate `smc` :
/// - `score_ob`        : score order block (0–100)
/// - `score_ifvg`      : score IFVG / imbalance (0–100)
/// - `score_imbalance` : score imbalance seul (0–100)
/// - `fib_actif`       : niveau Fibonacci 38.2–61.8% actif
pub fn categoriser_smc(
    score_ob: f64,
    score_ifvg: f64,
    score_imbalance: f64,
    fib_actif: bool,
    kill_zone_active: bool,
    sweep_detecte: bool,
    now: DateTime<Utc>,
) -> ResultatCategSmc {
    let ob_present = score_ob > 20.0;
    let ifvg_present = score_ifvg > 20.0;
    let imb_present = score_imbalance > 20.0;

    let mut confluences: Vec<String> = Vec::new();
    if ob_present {
        confluences.push("OB".into());
    }
    if ifvg_present {
        confluences.push("IFVG".into());
    }
    if imb_present {
        confluences.push("Imbalance".into());
    }
    if fib_actif {
        confluences.push("Fib".into());
    }
    if kill_zone_active {
        confluences.push("KillZone".into());
    }
    if sweep_detecte {
        confluences.push("Sweep".into());
    }

    let categorie = if ob_present && ifvg_present && imb_present {
        CategorieSmc::TripleConfluence
    } else if ob_present && ifvg_present {
        CategorieSmc::ObIfvg
    } else if ob_present && imb_present {
        CategorieSmc::ObImbalance
    } else if ob_present {
        CategorieSmc::ObSeul
    } else if fib_actif && (ifvg_present || imb_present) {
        CategorieSmc::FibConfluence
    } else if fib_actif {
        CategorieSmc::FibSeul
    } else {
        CategorieSmc::ChocIsole
    };

    let _ = confluences;
    ResultatCategSmc {
        categorie,
        session_active: session_active(now),
    }
}

// ── Kill Zone ICT ─────────────────────────────────────────────────────────────

/// Retourne `true` si l'instant UTC est dans une Kill Zone ICT.
/// London: 07:00–10:00 UTC | New York: 13:30–16:30 UTC
#[allow(dead_code)]
pub fn kill_zone(now: DateTime<Utc>) -> bool {
    let hm = now.hour() * 60 + now.minute();
    let london = (7 * 60)..(10 * 60);
    let new_york = (13 * 60 + 30)..(16 * 60 + 30);
    london.contains(&hm) || new_york.contains(&hm)
}

/// Session de marché active — réutilise la même logique que straddle_categorisation.
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
