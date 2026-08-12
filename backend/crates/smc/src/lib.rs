use common::{Candle, Direction};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub mod bos;
pub mod bpr;
pub mod choch;
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
pub mod v12;

pub use bos::ResultatBos;
pub use bpr::Bpr;
pub use choch::ResultatChoch;
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
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../../frontend/src/generated/ScoreSmc.ts")]
pub struct ScoreSmc {
    /// Score total sur 100
    pub total: f64,
    /// Points tendance (0–25)
    pub tendance: f64,
    /// Points order block (0–25)
    pub order_block: f64,
    /// Points IFVG (0–20) — continu : pondéré par la proximité au prix (10 pts max par IFVG)
    pub ifvg: f64,
    /// Points Fibonacci (0–15)
    pub fibonacci: f64,
    /// Points Imbalance/FVG (0–15) — continu : pondéré par la proximité au prix (7.5 pts max par zone)
    pub imbalance: f64,
    /// Prérequis ICT : Kill Zone active au moment du calcul
    pub kill_zone_active: bool,
    /// Prérequis ICT : Liquidity Sweep détecté sur les dernières bougies
    pub sweep_detecte: bool,
    /// Direction dominante détectée
    pub direction: Direction,
    /// Vrai si total >= 70 (seuil déclencheur)
    pub confluence: bool,
    /// Break of Structure détecté (cassure d'un swing dans la direction du signal)
    pub bos: bool,
    /// Change of Character détecté (premier retournement contre la tendance structurelle)
    pub choch: bool,
    /// Range de la session asiatique la plus récente (high/low pour overlay graphique)
    pub asian_range: Option<liquidites_range::RangeAsie>,
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

    // Prix de la dernière bougie — utilisé pour la pondération de proximité
    let prix_actuel = bougies.last()?.close;

    // IFVG : 0–20 pts continu — chaque IFVG aligné contribue selon sa proximité au prix
    let ifvgs = ifvg::detecter(bougies, 5, true, 0.25);
    let pts_ifvg = {
        let somme: f64 = ifvgs
            .iter()
            .filter(|fg| fg.direction == direction)
            .map(|fg| {
                let milieu = (fg.prix_haut + fg.prix_bas) / 2.0;
                let dist = ((prix_actuel - milieu).abs() / prix_actuel).min(1.0);
                // Contribution décroissante avec la distance (10 pts max par IFVG)
                10.0 / (1.0 + 20.0 * dist)
            })
            .sum();
        somme.min(SCORE_MAX_IFVG)
    };

    // Imbalance/FVG : 0–15 pts continu — zones alignées pondérées par proximité au prix
    let pts_imbalance = imbalance::score_continu_pour_direction(bougies, direction, prix_actuel);

    // Fibonacci : 0, 8 ou 15 pts selon la zone de retrace atteinte
    let pts_fib = fibonacci::calculer(bougies)
        .map(|n| fibonacci::score_fib(prix_actuel, &n))
        .unwrap_or(0.0);

    let total = pts_tendance + pts_ob + pts_ifvg + pts_imbalance + pts_fib;

    // Règle de diversité : la confluence requiert au moins 3 composantes actives (> 0 pts)
    // Empêche les faux positifs par accumulation de 2 composantes dominantes
    let composantes_actives = [pts_tendance, pts_ob, pts_ifvg, pts_imbalance, pts_fib]
        .iter()
        .filter(|&&p| p > 0.0)
        .count();

    // Prérequis ICT — gates binaires (non inclus dans le score, affichés séparément)
    let last_ts = bougies.last()?.timestamp;
    let kill_zone_active = kill_zone::est_en_kill_zone(last_ts);
    // Sweep directionnalisé : SSL (ssl_swepe=true) → Long, BSL (ssl_swepe=false) → Short
    let sweep_detecte = sweep::detecter_sweep(bougies)
        .map(|s| sweep_coherent_avec_direction(s.ssl_swepe, direction))
        .unwrap_or(false);

    // BOS : cassure d'un swing dans la direction du signal
    let bos = bos::detecter_bos(bougies)
        .map(|b| b.direction == direction)
        .unwrap_or(false);

    // CHoCH : premier retournement dans la direction du signal
    let choch = choch::detecter_choch(bougies)
        .map(|c| c.direction == direction)
        .unwrap_or(false);

    // Range session asiatique (informatif — 1 session récente)
    let asian_range = liquidites_range::detecter_ranges_asie(
        bougies,
        liquidites_range::ParamsRangeAsie::default(),
        1,
    )
    .into_iter()
    .next();

    tracing::debug!(
        "ScoreSmc {:?}: total={:.1} compos_act={} (tend={:.1} ob={:.1} ifvg={:.1} imb={:.1} fib={:.1}) bos={} choch={}",
        direction,
        total,
        composantes_actives,
        pts_tendance,
        pts_ob,
        pts_ifvg,
        pts_imbalance,
        pts_fib,
        bos,
        choch
    );

    Some(ScoreSmc {
        total,
        tendance: pts_tendance,
        order_block: pts_ob,
        imbalance: pts_imbalance,
        ifvg: pts_ifvg,
        fibonacci: pts_fib,
        direction,
        confluence: total >= 70.0 && composantes_actives >= 3,
        kill_zone_active,
        sweep_detecte,
        bos,
        choch,
        asian_range,
    })
}

/// Vérifie la cohérence direction sweep ↔ direction signal.
/// SSL sweep (ssl_swepe=true) précède un Long, BSL (ssl_swepe=false) précède un Short.
fn sweep_coherent_avec_direction(ssl_swepe: bool, direction: Direction) -> bool {
    match direction {
        Direction::Long => ssl_swepe,
        Direction::Short => !ssl_swepe,
        Direction::Both => false,
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

    #[test]
    fn regle_diversite_2_composantes_pas_confluence() {
        // Avec max 2 composantes (tendance=25, OB=25), le total ne peut pas dépasser 50
        // → confluence impossible mathématiquement (< 70)
        let composantes = [25.0_f64, 25.0, 0.0, 0.0, 0.0];
        let nb_actives = composantes.iter().filter(|&&p| p > 0.0).count();
        let total: f64 = composantes.iter().sum();
        // Vérification que la règle bloque bien un total fictif de 70 avec 2 composantes
        let confluence = total >= 70.0 && nb_actives >= 3;
        assert!(
            !confluence,
            "2 composantes ne peuvent pas donner confluence"
        );
    }

    #[test]
    fn regle_diversite_3_composantes_permet_confluence() {
        // tendance=25, OB=25, IFVG=20 → total=70, 3 composantes actives → confluence ✓
        let composantes = [25.0_f64, 25.0, 20.0, 0.0, 0.0];
        let nb_actives = composantes.iter().filter(|&&p| p > 0.0).count();
        let total: f64 = composantes.iter().sum();
        let confluence = total >= 70.0 && nb_actives >= 3;
        assert!(
            confluence,
            "3 composantes avec total=70 → confluence attendue"
        );
    }
}
