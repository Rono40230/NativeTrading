use common::{Candle, Direction};
use serde::{Deserialize, Serialize};

pub mod fibonacci;
pub mod ifvg;
pub mod imbalance;
pub mod order_blocks;
pub mod tendances;

pub use fibonacci::NiveauxFibonacci;
pub use ifvg::Ifvg;
pub use imbalance::Imbalance;
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
    /// Points imbalance/FVG (0–20)
    pub imbalance: f64,
    /// Points IFVG (0–15)
    pub ifvg: f64,
    /// Points Fibonacci (0–15)
    pub fibonacci: f64,
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

    // Order Block : 0–25 pts selon la force du meilleur OB aligné
    let obs = order_blocks::detecter(bougies);
    let pts_ob = (order_blocks::score_pour_direction(&obs, direction) / 100.0) * 25.0;

    // Imbalance/FVG : 0 ou 20 pts
    let imbs = imbalance::detecter(bougies);
    let pts_imb = imbalance::score_pour_direction(&imbs, direction);

    // IFVG : 0 ou 15 pts
    let ifvgs = ifvg::detecter(bougies);
    let pts_ifvg = ifvg::score_pour_direction(&ifvgs, direction);

    // Fibonacci : 0 ou 15 pts si prix proche d'un niveau clé
    let prix_actuel = bougies.last()?.close;
    let pts_fib = fibonacci::calculer(bougies)
        .and_then(|n| fibonacci::prix_sur_niveau(prix_actuel, &n, 0.002))
        .map(|_| 15.0)
        .unwrap_or(0.0);

    let total = pts_tendance + pts_ob + pts_imb + pts_ifvg + pts_fib;

    tracing::debug!(
        "ScoreSmc {:?}: total={:.1} (tendance={:.1} ob={:.1} imb={:.1} ifvg={:.1} fib={:.1})",
        direction, total, pts_tendance, pts_ob, pts_imb, pts_ifvg, pts_fib
    );

    Some(ScoreSmc {
        total,
        tendance: pts_tendance,
        order_block: pts_ob,
        imbalance: pts_imb,
        ifvg: pts_ifvg,
        fibonacci: pts_fib,
        direction,
        confluence: total >= 70.0,
    })
}
