use common::{Candle, Direction};
use serde::{Deserialize, Serialize};

pub mod bpr;
pub mod fibonacci;
pub mod ifvg;
pub mod imbalance;
pub mod kill_zone;
pub mod liquidites;
pub mod liquidites_range;
pub mod liquidites_tz;
pub mod order_blocks;
pub mod sweep;
pub mod tendances;

pub use bpr::Bpr;
pub use fibonacci::NiveauxFibonacci;
pub use ifvg::Ifvg;
pub use imbalance::ZoneImbalance;
pub use liquidites::DeviationAsie;
pub use liquidites::NiveauLiquidite;
pub use liquidites::RangeAsie;
pub use order_blocks::OrderBlock;
pub use tendances::ResultatTendance;

/// Score de confluence SMC (0–100). Seuil déclencheur stratégie : ≥70.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreSmc {
    /// Score total sur 100
    pub total: f64,
    /// Points tendance (0–25)
    pub tendance: f64,
    /// Points order block (0–25)
    pub order_block: f64,
    /// Points IFVG (0–35)
    pub ifvg: f64,
    /// Points Fibonacci (0–15)
    pub fibonacci: f64,
    /// Points imbalance/FVG — conservé à 0 (indicateur supprimé)
    pub imbalance: f64,
    /// Direction dominante détectée
    pub direction: Direction,
    /// Vrai si total >= 70 (seuil déclencheur)
    pub confluence: bool,
}

/// Calcule le score de confluence SMC pour un jeu de bougies.
///
/// Retourne `None` si la tendance est indécise (`Direction::Both`)
/// ou si les données sont insuffisantes.
pub fn scorer(bougies: &[Candle]) -> Option<ScoreSmc> {
    if bougies.len() < 20 {
        return None;
    }

    let tendance_res = tendances::analyser(bougies)?;
    let direction = tendance_res.direction;
    if direction == Direction::Both {
        return None; // Marché indécis — pas de signal SMC
    }

    // Tendance : 0–25 pts selon force (0=indécis, 1=partiel, 2=confirmé HH+HL)
    let pts_tendance = (tendance_res.force / 2.0) * 25.0;

    // Order Block : 0–25 pts selon la force du meilleur OB aligné (sensibilité défaut)
    let obs = order_blocks::detecter(bougies, 28.0, true);
    let pts_ob = (order_blocks::score_pour_direction(&obs, direction) / 100.0) * 25.0;

    // IFVG : 0 ou 35 pts (absorbe l'ancien slot FVG 20pts + IFVG 15pts)
    let ifvgs = ifvg::detecter(bougies, 5, true, 0.25);
    let pts_ifvg = if ifvg::score_pour_direction(&ifvgs, direction) > 0.0 {
        35.0
    } else {
        0.0
    };

    // Fibonacci : 0, 8 ou 15 pts selon la zone de retrace atteinte
    let prix_actuel = bougies.last()?.close;
    let pts_fib = fibonacci::calculer(bougies)
        .map(|n| fibonacci::score_fib(prix_actuel, &n))
        .unwrap_or(0.0);

    let total = pts_tendance + pts_ob + pts_ifvg + pts_fib;

    tracing::debug!(
        "ScoreSmc {:?}: total={:.1} (tendance={:.1} ob={:.1} ifvg={:.1} fib={:.1})",
        direction,
        total,
        pts_tendance,
        pts_ob,
        pts_ifvg,
        pts_fib
    );

    Some(ScoreSmc {
        total,
        tendance: pts_tendance,
        order_block: pts_ob,
        imbalance: 0.0,
        ifvg: pts_ifvg,
        fibonacci: pts_fib,
        direction,
        confluence: total >= 70.0,
    })
}
