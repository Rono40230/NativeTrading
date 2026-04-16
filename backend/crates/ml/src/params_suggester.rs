//! Génération de suggestions de paramètres basées sur les métriques de performance.
//! Toutes les fonctions sont pures — aucun accès DB, aucun effet de bord.
//! Seuils de sécurité : confiance minimum 0.70, minimum 30 trades de base.
use serde::{Deserialize, Serialize};

use crate::feedback_analyser::{AnalyseGlobale, RocketsAnalyse, SmcAnalyse, TrancheStat};

const SEUIL_MIN_SAMPLES: i64 = 30;
const SEUIL_MIN_CONFIANCE: f64 = 0.70;

// ── Structure suggestion ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestionParams {
    pub strategie: String,
    pub param_name: String,
    pub valeur_actuelle: f64,
    pub valeur_suggeree: f64,
    pub gain_winrate_estime: f64, // % points WR estimés
    pub confiance: f64,           // 0.0-1.0
    pub justification: String,
    pub nb_samples_base: i64,
}

// ── Point d'entrée principal ──────────────────────────────────────────────────

/// Génère la liste des suggestions de paramètres triée par gain estimé décroissant.
/// Ne génère rien si le minimum de samples ou de confiance n'est pas atteint.
pub fn generer_suggestions(
    analyse: &AnalyseGlobale,
    score_min_smc: i64,
    kill_zone_filtre_smc: bool,
    atr_sl_smc: f64,
) -> Vec<SuggestionParams> {
    let mut suggestions = Vec::new();

    if let Some(ref smc) = analyse.smc {
        if smc.global.nb_trades >= SEUIL_MIN_SAMPLES {
            suggerer_score_min(smc, score_min_smc, &mut suggestions);
            suggerer_kill_zone(analyse, kill_zone_filtre_smc, &mut suggestions);
            suggerer_atr_sl(smc, atr_sl_smc, &mut suggestions);
        }
    }

    if let Some(ref rockets) = analyse.rockets {
        if rockets.global.nb_trades >= SEUIL_MIN_SAMPLES {
            suggerer_conviction_rockets(rockets, &mut suggestions);
            suggerer_pnl_rockets(rockets, &mut suggestions);
            suggerer_winrate_rockets(rockets, &mut suggestions);
        }
    }

    // Trier par gain win rate estimé décroissant
    suggestions.sort_by(|a, b| {
        b.gain_winrate_estime
            .partial_cmp(&a.gain_winrate_estime)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    suggestions
}

// ── Règles de suggestion ──────────────────────────────────────────────────────

fn suggerer_score_min(smc: &SmcAnalyse, score_min_actuel: i64, out: &mut Vec<SuggestionParams>) {
    if score_min_actuel >= 65 {
        return; // Score déjà suffisamment élevé
    }
    let Some((nb_bas, wr_bas)) = tranche(&smc.par_score, "50-65") else {
        return;
    };
    let Some((_, wr_moyen)) = tranche(&smc.par_score, "65-75") else {
        return;
    };

    if nb_bas < SEUIL_MIN_SAMPLES || wr_bas >= 45.0 {
        return;
    }
    let gain = (wr_moyen - wr_bas).max(0.0);
    if gain < 5.0 {
        return;
    }
    let confiance = (0.70 + (nb_bas as f64 / 300.0).min(0.25)).min(0.95);
    if confiance < SEUIL_MIN_CONFIANCE {
        return;
    }

    out.push(SuggestionParams {
        strategie: "SMC".into(),
        param_name: "score_min".into(),
        valeur_actuelle: score_min_actuel as f64,
        valeur_suggeree: 65.0,
        gain_winrate_estime: gain,
        confiance,
        justification: format!(
            "Win rate {:.0}% pour score < 65 ({} trades) vs {:.0}% pour score 65-75",
            wr_bas, nb_bas, wr_moyen
        ),
        nb_samples_base: nb_bas,
    });
}

fn suggerer_kill_zone(
    analyse: &AnalyseGlobale,
    kill_zone_active: bool,
    out: &mut Vec<SuggestionParams>,
) {
    let Some(wr_kz) = analyse.smc_win_rate_kill_zone() else {
        return;
    };
    let Some(wr_hors) = analyse.smc_win_rate_hors_kill_zone() else {
        return;
    };
    let nb = analyse
        .smc
        .as_ref()
        .map(|s| s.global.nb_trades)
        .unwrap_or(0);

    if nb < SEUIL_MIN_SAMPLES {
        return;
    }

    // Kill Zone désactivée mais clairement moins bonne hors Kill Zone → réactiver
    if !kill_zone_active && wr_hors < 40.0 && wr_kz > wr_hors + 10.0 {
        out.push(SuggestionParams {
            strategie: "SMC".into(),
            param_name: "kill_zone_filtre".into(),
            valeur_actuelle: 0.0,
            valeur_suggeree: 1.0,
            gain_winrate_estime: wr_kz - wr_hors,
            confiance: 0.80,
            justification: format!(
                "Hors Kill Zone : {:.0}% WR vs {:.0}% en Kill Zone ({} trades) — filtre recommandé",
                wr_hors, wr_kz, nb
            ),
            nb_samples_base: nb,
        });
    }

    // Kill Zone active mais hors session fait mieux → désactiver
    if kill_zone_active && wr_hors > wr_kz + 10.0 {
        out.push(SuggestionParams {
            strategie:           "SMC".into(),
            param_name:          "kill_zone_filtre".into(),
            valeur_actuelle:     1.0,
            valeur_suggeree:     0.0,
            gain_winrate_estime: wr_hors - wr_kz,
            confiance:           0.75,
            justification: format!(
                "Performance {:.0}% WR hors Kill Zone > {:.0}% en session — filtre contre-productif",
                wr_hors, wr_kz
            ),
            nb_samples_base: nb,
        });
    }
}

fn suggerer_atr_sl(smc: &SmcAnalyse, atr_sl_actuel: f64, out: &mut Vec<SuggestionParams>) {
    // R:R moyen inférieur à 0.5 sur au moins 50 trades → SL probablement trop serré
    if smc.global.pnl_r_moyen >= 0.5 || smc.global.nb_trades < 50 {
        return;
    }
    let valeur_suggeree = (atr_sl_actuel * 1.2).min(2.5);
    if (valeur_suggeree - atr_sl_actuel).abs() < 0.05 {
        return;
    }

    out.push(SuggestionParams {
        strategie: "SMC".into(),
        param_name: "atr_sl".into(),
        valeur_actuelle: atr_sl_actuel,
        valeur_suggeree,
        gain_winrate_estime: 3.0, // Estimation conservatrice
        confiance: 0.65,
        justification: format!(
            "R:R moyen {:.2} sur {} trades — SL trop serré, élargir de {:.1} à {:.1}×ATR",
            smc.global.pnl_r_moyen, smc.global.nb_trades, atr_sl_actuel, valeur_suggeree
        ),
        nb_samples_base: smc.global.nb_trades,
    });
}

// ── Utilitaire ────────────────────────────────────────────────────────────────

fn tranche(tranches: &[TrancheStat], nom: &str) -> Option<(i64, f64)> {
    tranches
        .iter()
        .find(|t| t.tranche == nom)
        .map(|t| (t.nb_trades, t.win_rate))
}

// ── Règles Rockets ────────────────────────────────────────────────────────────

/// Si une tranche de conviction LLM a un WR nettement inférieur aux autres,
/// suggérer de relever le seuil de conviction minimum.
fn suggerer_conviction_rockets(rockets: &RocketsAnalyse, out: &mut Vec<SuggestionParams>) {
    // Tranche basse : conviction < 60
    let Some((nb_bas, wr_bas)) = tranche(&rockets.conviction_llm, "<60") else {
        return;
    };
    // Tranche haute : conviction ≥ 70
    let Some((_, wr_haut)) = tranche(&rockets.conviction_llm, "70-80").or_else(|| tranche(&rockets.conviction_llm, "80+")) else {
        return;
    };

    if nb_bas < 10 || wr_bas >= 45.0 {
        return;
    }
    let gain = (wr_haut - wr_bas).max(0.0);
    if gain < 8.0 {
        return;
    }
    let confiance = (0.70 + (rockets.global.nb_trades as f64 / 200.0).min(0.20)).min(0.90);
    if confiance < SEUIL_MIN_CONFIANCE {
        return;
    }

    out.push(SuggestionParams {
        strategie:           "ROCKETS".into(),
        param_name:          "conviction_llm_min".into(),
        valeur_actuelle:     60.0,
        valeur_suggeree:     70.0,
        gain_winrate_estime: gain,
        confiance,
        justification: format!(
            "Conviction <60 : {:.0}% WR ({} trades) vs {:.0}% à 70+ — relever le seuil LLM",
            wr_bas, nb_bas, wr_haut
        ),
        nb_samples_base: nb_bas,
    });
}

/// Si le WR global est sous l'objectif (55%), suggérer de relever le score_min du scan.
fn suggerer_winrate_rockets(rockets: &RocketsAnalyse, out: &mut Vec<SuggestionParams>) {
    const OBJECTIF_WR: f64 = 55.0;
    const SEUIL_TRADES: i64 = 50;
    if rockets.global.win_rate >= OBJECTIF_WR || rockets.global.nb_trades < SEUIL_TRADES {
        return;
    }
    let ecart = OBJECTIF_WR - rockets.global.win_rate;
    let confiance = (0.70 + (rockets.global.nb_trades as f64 / 300.0).min(0.20)).min(0.90);
    if confiance < SEUIL_MIN_CONFIANCE {
        return;
    }
    out.push(SuggestionParams {
        strategie:           "ROCKETS".into(),
        param_name:          "score_min".into(),
        valeur_actuelle:     60.0,
        valeur_suggeree:     65.0,
        gain_winrate_estime: ecart,
        confiance,
        justification: format!(
            "WR global {:.1}% ({} trades) sous objectif 55% — relever score_min de 60 à 65 pour filtrer les setups faibles",
            rockets.global.win_rate, rockets.global.nb_trades
        ),
        nb_samples_base: rockets.global.nb_trades,
    });
}

/// Si le R:R moyen global est < 0.5, suggérer de revoir les paramètres TP/SL Rockets.
fn suggerer_pnl_rockets(rockets: &RocketsAnalyse, out: &mut Vec<SuggestionParams>) {
    if rockets.global.pnl_r_moyen >= 0.5 || rockets.global.nb_trades < 40 {
        return;
    }
    out.push(SuggestionParams {
        strategie:           "ROCKETS".into(),
        param_name:          "tp_multiplier".into(),
        valeur_actuelle:     2.0,
        valeur_suggeree:     2.5,
        gain_winrate_estime: 3.0,
        confiance:           0.65,
        justification: format!(
            "R:R moyen {:.2}R sur {} trades — TP trop conservateur, élargir de 2.0× à 2.5×ATR",
            rockets.global.pnl_r_moyen, rockets.global.nb_trades
        ),
        nb_samples_base: rockets.global.nb_trades,
    });
}
