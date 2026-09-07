//! §8 (07/09) — journalisation du détail de qualification d'un setup SMC.
//!
//! Au moment où un trade est créé (qualification v11), le moteur connaît
//! TOUT ce qui a fait passer le setup : score de la zone, force, qualité de
//! zone, sweep frais, premium/discount, ET les composantes actives du score
//! live (`live_score_detaille`). Ce module sérialise cet instantané en JSON
//! compact — la matière première de l'analyse des setups qui meurent (§8 :
//! « pourquoi 35 SL ? ») et des 7 features contextuelles ML.
//!
//! Lecture seule pour le moteur : rien ne consomme ce détail dans la
//! décision — il est journalisé, point.



use super::scoring_v11::ScoringV11;
use super::calibration::AssetCalibration;
use super::types::{BarInput, SmcOutput};

/// Instantané JSON de la qualification (posé dans `Trade.detail`, transporté
/// jusqu'à `smc_scoring_detail` par le writer officiel).
#[allow(clippy::too_many_arguments)]
pub fn detail_qualification(
    source: &str,
    score_zone: i32,
    zone_qualifiee: bool,
    sweep_ok: bool,
    pd_ok: bool,
    is_bull: bool,
    out: &SmcOutput,
    bar: &BarInput,
    cal: &AssetCalibration,
) -> String {
    // Composantes actives du score live au moment de l'émission — le
    // miroir du diagFlags MQL5.
    let (score_live, flags) =
        ScoringV11::live_score_detaille(is_bull, out, bar, cal, None, None, false);
    let composantes: Vec<String> =
        flags.iter().map(|f| format!("\"{}\"", f)).collect();
    format!(
        "{{\"source\":\"{}\",\"score_zone\":{},\"force\":{},\"zone_qualifiee\":{},\"sweep_frais\":{},\"premium_discount_ok\":{},\"score_live\":{},\"composantes\":[{}]}}",
        source,
        score_zone,
        ScoringV11::force(score_zone, cal),
        zone_qualifiee,
        sweep_ok,
        pd_ok,
        score_live,
        composantes.join(",")
    )
}
