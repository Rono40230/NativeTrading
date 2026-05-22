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

/// Barèmes publics du scoring SMC (source de vérité unique).
pub const SCORE_MAX_TENDANCE: f64 = 25.0;
pub const SCORE_MAX_ORDER_BLOCK: f64 = 25.0;
pub const SCORE_MAX_IFVG: f64 = 20.0;
pub const SCORE_MAX_IMBALANCE: f64 = 15.0;
pub const SCORE_MAX_FIBONACCI: f64 = 15.0;
/// Score total max (somme des composantes ci-dessus)
pub const SCORE_TOTAL_MAX: f64 = 100.0;

/// Score de confluence SMC (0–100). Seuil déclencheur stratégie : ≥70.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreSmc {
    /// Score total sur 100
    pub total: f64,
    /// Points tendance (0–25)
    pub tendance: f64,
    /// Points order block (0–25)
    pub order_block: f64,
    /// Points IFVG (0–20) — gradué : 0=absent, 1 IFVG=10, 2+=20
    pub ifvg: f64,
    /// Points Fibonacci (0–15)
    pub fibonacci: f64,
    /// Points Imbalance/FVG (0–15) — gradué : 0=absent, 1 zone=8, 2+=15
    pub imbalance: f64,
    /// Prérequis ICT : Kill Zone active au moment du calcul
    pub kill_zone_active: bool,
    /// Prérequis ICT : Liquidity Sweep détecté sur les dernières bougies
    pub sweep_detecte: bool,
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

    // IFVG : 0–20 pts gradué (0 IFVG aligné=0, 1=10, 2+=20)
    let ifvgs = ifvg::detecter(bougies, 5, true, 0.25);
    let nb_ifvg = ifvgs.iter().filter(|fg| fg.direction == direction).count();
    let pts_ifvg = match nb_ifvg {
        0 => 0.0,
        1 => 10.0,
        _ => 20.0,
    };

    // Imbalance/FVG : 0–15 pts gradué (0 zone=0, 1=8, 2+=15)
    let pts_imbalance = imbalance::score_pour_direction(bougies, direction);

    // Fibonacci : 0, 8 ou 15 pts selon la zone de retrace atteinte
    let prix_actuel = bougies.last()?.close;
    let pts_fib = fibonacci::calculer(bougies)
        .map(|n| fibonacci::score_fib(prix_actuel, &n))
        .unwrap_or(0.0);

    let total = pts_tendance + pts_ob + pts_ifvg + pts_imbalance + pts_fib;

    // Prérequis ICT — gates binaires (non inclus dans le score, affichés séparément)
    let last_ts = bougies.last()?.timestamp;
    let kill_zone_active = kill_zone::est_en_kill_zone(last_ts);
    // Sweep directionnalisé : SSL (ssl_swepe=true) → Long, BSL (ssl_swepe=false) → Short
    let sweep_detecte = sweep::detecter_sweep(bougies)
        .map(|s| sweep_coherent_avec_direction(s.ssl_swepe, direction))
        .unwrap_or(false);

    tracing::debug!(
        "ScoreSmc {:?}: total={:.1} (tend={:.1} ob={:.1} ifvg={:.1} imb={:.1} fib={:.1})",
        direction,
        total,
        pts_tendance,
        pts_ob,
        pts_ifvg,
        pts_imbalance,
        pts_fib
    );

    Some(ScoreSmc {
        total,
        tendance: pts_tendance,
        order_block: pts_ob,
        imbalance: pts_imbalance,
        ifvg: pts_ifvg,
        fibonacci: pts_fib,
        direction,
        confluence: total >= 70.0,
        kill_zone_active,
        sweep_detecte,
    })
}

/// Vérifie la cohérence direction sweep ↔ direction signal.
/// SSL sweep (ssl_swepe=true) précède un Long, BSL (ssl_swepe=false) précède un Short.
fn sweep_coherent_avec_direction(ssl_swepe: bool, direction: Direction) -> bool {
    match direction {
        Direction::Long  => ssl_swepe,
        Direction::Short => !ssl_swepe,
        Direction::Both  => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sweep_ssl_valide_direction_long() {
        // SSL sweep (wick bas sous swing low) → signal Long attendu
        assert!(sweep_coherent_avec_direction(true, Direction::Long));
    }

    #[test]
    fn sweep_bsl_invalide_pour_long() {
        // BSL sweep (wick haut) ne doit PAS valider un Long
        assert!(!sweep_coherent_avec_direction(false, Direction::Long));
    }

    #[test]
    fn sweep_bsl_valide_direction_short() {
        // BSL sweep (wick haut au-dessus swing high) → signal Short attendu
        assert!(sweep_coherent_avec_direction(false, Direction::Short));
    }

    #[test]
    fn sweep_ssl_invalide_pour_short() {
        // SSL sweep ne doit PAS valider un Short
        assert!(!sweep_coherent_avec_direction(true, Direction::Short));
    }
}
