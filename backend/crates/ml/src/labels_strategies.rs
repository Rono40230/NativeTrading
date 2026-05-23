//! Fonctions de labellisation spécialisées par stratégie (Phase 3.3 ROADMAP).
//!
//! Contrairement à `features::labelliser` (label directionnel générique), ces fonctions
//! capturent la sémantique propre à chaque stratégie :
//! - **Straddle** : amplitude max dans l'horizon (direction agnostique)
//! - **Rockets** : breakout haussier de X% dans l'horizon
//! - **SMC** : tenue jusqu'au TP1 dans la direction prévue
use common::Candle;

/// Label Straddle : 1.0 si l'amplitude max dans l'horizon >= `seuil_atr_mult` × ATR14.
///
/// Le Straddle bénéficie des mouvements amples QUELLE QUE SOIT la direction.
/// L'amplitude est mesurée comme `max(|high_i - prix_ref|, |prix_ref - low_i|)`
/// sur toutes les bougies de l'horizon.
///
/// Retourne `None` si l'horizon dépasse la série ou si ATR14 est nul.
pub fn labelliser_straddle(
    bougies: &[Candle],
    index: usize,
    horizon: usize,
    seuil_atr_mult: f64,
) -> Option<f64> {
    let futur_fin = index + horizon;
    if futur_fin >= bougies.len() || index < 14 {
        return None;
    }
    let atr_series = indicators::calculer_atr(&bougies[..=index], 14);
    let atr = atr_series.last().copied().unwrap_or(0.0);
    if atr <= 0.0 {
        return None;
    }
    let prix_ref = bougies[index].close;
    let amplitude_max = bougies[index + 1..=futur_fin]
        .iter()
        .map(|b| (b.high - prix_ref).abs().max((prix_ref - b.low).abs()))
        .fold(0.0_f64, f64::max);
    Some(if amplitude_max >= seuil_atr_mult * atr { 1.0 } else { 0.0 })
}

/// Label Rockets : 1.0 si le close max dans l'horizon marque un breakout haussier >= `seuil_pct`.
///
/// Le breakout est validé si `max(close_i) / close_ref - 1.0 >= seuil_pct`.
/// Exemple : seuil_pct=0.08 → breakout de +8%.
///
/// Retourne `None` si l'horizon dépasse la série ou si le prix de référence est nul.
pub fn labelliser_rockets(
    bougies: &[Candle],
    index: usize,
    horizon: usize,
    seuil_pct: f64,
) -> Option<f64> {
    let futur_fin = index + horizon;
    if futur_fin >= bougies.len() {
        return None;
    }
    let prix_ref = bougies[index].close;
    if prix_ref <= 0.0 {
        return None;
    }
    let breakout_max = bougies[index + 1..=futur_fin]
        .iter()
        .map(|b| b.close / prix_ref - 1.0)
        .fold(f64::NEG_INFINITY, f64::max);
    Some(if breakout_max >= seuil_pct { 1.0 } else { 0.0 })
}

/// Label SMC Directionnel : 1.0 si le signal tient jusqu'au TP1 (mouvement >= `seuil_pct`
/// dans la direction prévue à l'intérieur de l'horizon).
///
/// - `direction_haussiere = true` : on attend une hausse de `seuil_pct`
/// - `direction_haussiere = false` : on attend une baisse de `seuil_pct`
///
/// Retourne `None` si l'horizon dépasse la série ou si le prix de référence est nul.
pub fn labelliser_smc(
    bougies: &[Candle],
    index: usize,
    horizon: usize,
    seuil_pct: f64,
    direction_haussiere: bool,
) -> Option<f64> {
    let futur_fin = index + horizon;
    if futur_fin >= bougies.len() {
        return None;
    }
    let prix_ref = bougies[index].close;
    if prix_ref <= 0.0 {
        return None;
    }
    let tenu = if direction_haussiere {
        bougies[index + 1..=futur_fin]
            .iter()
            .any(|b| b.close / prix_ref - 1.0 >= seuil_pct)
    } else {
        bougies[index + 1..=futur_fin]
            .iter()
            .any(|b| prix_ref / b.close - 1.0 >= seuil_pct)
    };
    Some(if tenu { 1.0 } else { 0.0 })
}
