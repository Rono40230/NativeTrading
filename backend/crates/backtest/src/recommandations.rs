//! Moteur de recommandations post-backtest.
//!
//! Analyse les métriques d'un `BacktestResult` et retourne une liste de
//! `Recommandation` actionnables triées par priorité décroissante.
//! Aucune modification automatique — le trader décide et applique.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::BacktestResult;

/// Une recommandation adressée au trader après analyse des métriques backtest.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../../frontend/src/generated/Recommandation.ts")]
pub struct Recommandation {
    /// Titre court affiché dans l'UI (≤ 80 caractères)
    pub titre: String,
    /// Constat chiffré détaillé
    pub explication: String,
    /// Impact estimé si la recommandation est appliquée (ex: "+8% win rate estimé")
    pub impact_estime: String,
    /// Clé du paramètre à ajuster dans `StrategiesParamsPanel`
    pub param_cible: String,
    /// Valeur actuelle du paramètre (en string pour l'affichage)
    pub valeur_actuelle: String,
    /// Valeur suggérée par l'analyse
    pub valeur_suggeree: String,
    /// "straddle" | "smc" | "rockets" | "global"
    pub strategie: String,
    /// 1 = critique (perte certaine), 2 = important, 3 = mineur
    pub priorite: u8,
}

/// Analyse un `BacktestResult` et produit la liste des recommandations.
/// Retourne une liste vide si aucun problème détecté.
pub fn analyser_recommandations(result: &BacktestResult) -> Vec<Recommandation> {
    let mut recs: Vec<Recommandation> = Vec::new();

    let strategie = format!("{:?}", result.config.strategie).to_lowercase();

    // ── Règle 1 : Taux de double SL Straddle élevé ───────────────────────────
    if let Some(dsl) = result.double_sl_rate {
        if dsl > 0.25 {
            // Détecter les créneaux horaires les plus mauvais
            let heures_perdantes: Vec<String> = result
                .stats_par_heure
                .iter()
                .filter(|s| s.nb_trades >= 2 && s.win_rate < 0.35 && s.pnl_r_moyen < -0.5)
                .map(|s| format!("{:02}h", s.heure))
                .collect();

            let explication = if heures_perdantes.is_empty() {
                format!(
                    "{:.1}% des trades Straddle sont des double SL (les deux jambes stoppées). \
                     Seuil acceptable : <25%.",
                    dsl * 100.0
                )
            } else {
                format!(
                    "{:.1}% des trades Straddle sont des double SL. \
                     Créneaux les plus touchés : {}.",
                    dsl * 100.0,
                    heures_perdantes.join(", ")
                )
            };

            let valeur_suggeree = if heures_perdantes.is_empty() {
                "Augmenter sl_mult de 0.5 → 0.7".to_string()
            } else {
                format!("Exclure les créneaux : {}", heures_perdantes.join(", "))
            };

            recs.push(Recommandation {
                titre: "Taux de double SL Straddle trop élevé".to_string(),
                explication,
                impact_estime: format!(
                    "-{:.1}% de pertes estimées si créneaux exclus",
                    dsl * 100.0 * 0.4
                ),
                param_cible: if heures_perdantes.is_empty() {
                    "sl_mult".to_string()
                } else {
                    "heures_exclues".to_string()
                },
                valeur_actuelle: "0.5".to_string(),
                valeur_suggeree,
                strategie: strategie.clone(),
                priorite: 1,
            });
        }
    }

    // ── Règle 2 : Win rate global trop faible ────────────────────────────────
    if result.nb_trades >= 10 && result.win_rate < 0.45 {
        recs.push(Recommandation {
            titre: "Win rate insuffisant — seuil de score trop permissif".to_string(),
            explication: format!(
                "Win rate = {:.1}% sur {} trades (seuil minimal : 45%). \
                 Relever le score minimum devrait filtrer les signaux de faible qualité.",
                result.win_rate * 100.0,
                result.nb_trades,
            ),
            impact_estime: "+5 à +10% de win rate estimé".to_string(),
            param_cible: "score_min".to_string(),
            valeur_actuelle: "70".to_string(),
            valeur_suggeree: "75".to_string(),
            strategie: strategie.clone(),
            priorite: 1,
        });
    }

    // ── Règle 3 : Drawdown maximum trop élevé ────────────────────────────────
    if result.drawdown_max > 0.15 {
        recs.push(Recommandation {
            titre: "Drawdown maximum trop élevé — réduire le risque par trade".to_string(),
            explication: format!(
                "Drawdown max = {:.1}% (seuil de sécurité : 15%). \
                 Réduire le risque par trade permettra de limiter les pertes en série.",
                result.drawdown_max * 100.0
            ),
            impact_estime: format!(
                "Drawdown max estimé à {:.1}% après réduction",
                result.drawdown_max * 100.0 * 0.65
            ),
            param_cible: "risque_par_trade".to_string(),
            valeur_actuelle: format!("{:.1}%", result.config.risque_par_trade * 100.0),
            valeur_suggeree: format!("{:.1}%", result.config.risque_par_trade * 100.0 * 0.65),
            strategie: strategie.clone(),
            priorite: 1,
        });
    }

    // ── Règle 4 : Sharpe trop bas — sur-trading ──────────────────────────────
    if result.nb_trades > 80 && result.sharpe < 1.0 {
        recs.push(Recommandation {
            titre: "Ratio de Sharpe faible — trop de trades peu qualitatifs".to_string(),
            explication: format!(
                "Sharpe = {:.2} sur {} trades. Un ratio < 1.0 avec un volume élevé \
                 indique un sur-trading : beaucoup de signaux de faible valeur attendue.",
                result.sharpe, result.nb_trades
            ),
            impact_estime: "Sharpe > 1.2 estimé en réduisant la fréquence".to_string(),
            param_cible: "score_min".to_string(),
            valeur_actuelle: "70".to_string(),
            valeur_suggeree: "78".to_string(),
            strategie: strategie.clone(),
            priorite: 2,
        });
    }

    // ── Règle 5 : Profit factor < 1 → stratégie perdante ────────────────────
    if result.nb_trades >= 10 && result.profit_factor < 1.0 {
        recs.push(Recommandation {
            titre: "Stratégie perdante sur cette période — vérifier les paramètres".to_string(),
            explication: format!(
                "Profit factor = {:.2} (< 1.0 = pertes nettes). \
                 Les gains ne couvrent pas les pertes sur cette configuration.",
                result.profit_factor
            ),
            impact_estime: "Revoir les niveaux TP/SL pour rééquilibrer le R:R".to_string(),
            param_cible: "tp_mult_1".to_string(),
            valeur_actuelle: "2.0".to_string(),
            valeur_suggeree: "2.5".to_string(),
            strategie: strategie.clone(),
            priorite: 1,
        });
    }

    // ── Règle 6 : Créneaux horaires systématiquement perdants ────────────────
    let heures_critiques: Vec<&crate::StatHeure> = result
        .stats_par_heure
        .iter()
        .filter(|s| s.nb_trades >= 3 && s.win_rate < 0.3 && s.pnl_r_moyen < -1.0)
        .collect();

    if !heures_critiques.is_empty() {
        let liste: Vec<String> = heures_critiques
            .iter()
            .map(|s| format!("{:02}h (win:{:.0}%)", s.heure, s.win_rate * 100.0))
            .collect();
        recs.push(Recommandation {
            titre: "Créneaux horaires systématiquement perdants détectés".to_string(),
            explication: format!(
                "Créneaux avec win rate < 30% et P&L moyen < -1R : {}. \
                 Les exclure devrait améliorer significativement les résultats.",
                liste.join(", ")
            ),
            impact_estime: "+8 à +15% de P&L net estimé".to_string(),
            param_cible: "heures_exclues".to_string(),
            valeur_actuelle: "[]".to_string(),
            valeur_suggeree: format!(
                "[{}]",
                heures_critiques.iter().map(|s| s.heure.to_string()).collect::<Vec<_>>().join(", ")
            ),
            strategie: strategie.clone(),
            priorite: 2,
        });
    }

    // Trier par priorité croissante (1 = le plus urgent en premier)
    recs.sort_by_key(|r| r.priorite);
    recs
}

#[cfg(test)]
#[path = "tests_reco.rs"]
mod tests;
