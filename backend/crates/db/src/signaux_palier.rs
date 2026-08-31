//! R de référence par palier (spéc propriétaire 31/08).
//!
//! La vérité qui juge une entrée est l'EXTRÊME atteint — SL ou TP max touché —
//! pas la sortie (trailing, BE, time-stop). Ce module dérive ce R de référence
//! depuis les champs déjà stockés (`verdict`, niveaux), pour la courbe R cumulé
//! et les stats des blocs stratégies du dashboard. Miroir exact de la logique
//! front `useSignalFormat.palierMax` — toute évolution se fait des deux côtés.
//!
//! Chirurgie : `r_realise` reste la trace factuelle en base ; le R de
//! référence est un calcul dérivé, jamais réécrit (leçon migrations 0082/0083).

/// R de référence d'un trade clôturé, déduit du verdict (palier max touché)
/// et des niveaux stockés :
/// - SMC      : dist(tp_n)/risque (TP1 ≈ 0.6R post-étape 4, TP2 = 2R…)
/// - Straddle : idem MOINS 1R (la jambe perdante a payé le SL à ±1R)
/// - SL → −1R · BE → 0R
/// None si le verdict ne dit rien d'exploitable (expire…) → l'appelant
/// repliera sur `r_realise`.
pub fn r_reference_palier(
    verdict: &str,
    strategie: &str,
    entree: f64,
    sl: f64,
    tps: &[f64],
) -> Option<f64> {
    let v = verdict.to_lowercase();
    let penalite_straddle = if strategie.to_lowercase().contains("straddle") {
        1.0
    } else {
        0.0
    };

    let r_niveau = |tp: Option<&f64>| -> Option<f64> {
        let tp = tp?;
        let risque = (entree - sl).abs();
        if risque <= f64::EPSILON {
            return None;
        }
        Some(((tp - entree).abs() / risque) - penalite_straddle)
    };

    match v.as_str() {
        "sl" | "sl+be" => Some(-1.0),
        "tp1" | "tp1+be" => r_niveau(tps.first()),
        "tp2" | "tp2+be" => r_niveau(tps.get(1).or_else(|| tps.first())),
        "tp3" => r_niveau(tps.get(2).or_else(|| tps.first())),
        "be" => Some(0.0),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::r_reference_palier;

    const TPS_SMC: [f64; 3] = [2006.0, 2020.0, 2030.0]; // entry 2000, SL 1990 → 0.6R/2R/3R
    const TPS_STRADDLE: [f64; 2] = [2010.0, 2020.0]; // entry 2000, SL 1990 → 1R/2R

    #[test]
    fn smc_tp2_vaut_2r() {
        let r = r_reference_palier("TP2", "SMC", 2000.0, 1990.0, &TPS_SMC).unwrap();
        assert!((r - 2.0).abs() < 1e-9, "{r}");
    }

    #[test]
    fn smc_tp1_post_etape4_vaut_06r() {
        let r = r_reference_palier("TP1", "SMC", 2000.0, 1990.0, &TPS_SMC).unwrap();
        assert!((r - 0.6).abs() < 1e-9, "{r}");
    }

    #[test]
    fn straddle_tp2_net_1r() {
        // 2R du palier − 1R de la jambe perdante
        let r = r_reference_palier("tp2", "straddle", 2000.0, 1990.0, &TPS_STRADDLE).unwrap();
        assert!((r - 1.0).abs() < 1e-9, "{r}");
    }

    #[test]
    fn straddle_tp1_net_0r() {
        let r = r_reference_palier("tp1", "straddle", 2000.0, 1990.0, &TPS_STRADDLE).unwrap();
        assert!((r - 0.0).abs() < 1e-9, "{r}");
    }

    #[test]
    fn sl_toujours_moins_1r() {
        for (v, s) in [("SL", "SMC"), ("sl", "straddle"), ("sl+be", "straddle")] {
            assert_eq!(r_reference_palier(v, s, 2000.0, 1990.0, &TPS_SMC), Some(-1.0));
        }
    }

    #[test]
    fn be_0r_et_expire_inconnu() {
        assert_eq!(r_reference_palier("be", "SMC", 2000.0, 1990.0, &TPS_SMC), Some(0.0));
        assert_eq!(r_reference_palier("expire", "SMC", 2000.0, 1990.0, &TPS_SMC), None);
    }

    #[test]
    fn variante_historique_straddle_capitale() {
        // le writer v1 écrivait 'Straddle' — insensible à la casse
        let r = r_reference_palier("TP2+BE", "Straddle", 2000.0, 1990.0, &TPS_STRADDLE).unwrap();
        assert!((r - 1.0).abs() < 1e-9, "{r}");
    }
}
