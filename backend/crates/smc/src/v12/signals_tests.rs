//! Tests du générateur de signaux — SL fixe (owner 19/10), DoL TP3.

    use super::*;
    fn fixe(base: f64, garde: GardeSlFixe) -> SignalGenerator {
        let mut g = SignalGenerator::new();
        g.definir_sl_fixe(Some(SlFixeEtude {
            base,
            mult_faible: 0.7,
            mult_fort: 1.3,
            garde,
        }));
        g
    }

    #[test]
    fn nearest_liq_bull_plus_proche() {
        let mut out = SmcOutput::default();
        out.liquidite.pdh_active = Some(110.0);
        out.liquidite.pwh_active = Some(120.0);
        // entry=100 → candidats 110, 120 → min = 110.
        assert_eq!(nearest_liq(&out, 100.0, true, true), Some(110.0));
        assert_eq!(nearest_liq(&out, 100.0, true, false), Some(110.0));
    }

    #[test]
    fn nearest_liq_asian_hl_v11_seulement() {
        // Pine : _tAHH3 candidat pour v11 (Pine 3562), PAS pour BS (_bsDolTarget 3294).
        let mut out = SmcOutput::default();
        out.asian_hl.high = Some(105.0);
        out.asian_hl.low = Some(95.0);
        assert_eq!(nearest_liq(&out, 100.0, true, true), Some(105.0));
        assert_eq!(nearest_liq(&out, 100.0, true, false), None);
        assert_eq!(nearest_liq(&out, 100.0, false, true), Some(95.0));
        assert_eq!(nearest_liq(&out, 100.0, false, false), None);
    }

    #[test]
    fn nearest_liq_aucune() {
        let out = SmcOutput::default();
        assert_eq!(nearest_liq(&out, 100.0, true, true), None);
    }

    #[test]
    fn build_levels_fixe_paliers_et_serre() {
        // OB bull top=100 bot=96 (largeur 4), base 5, score moyen → r=5,
        // garde SerreBord → r = min(5, 4) = 4. SL=96, TP1=100+0.6×4.
        let c = AssetCalibration::detect("XAUUSD", "M15");
        let out = SmcOutput::default();
        let gen = fixe(5.0, GardeSlFixe::SerreBord);
        let (entry, sl, tp1, tp2, _tp3, r) =
            gen.build_levels(true, 100.0, 96.0, &out, true, 10).unwrap();
        assert!((entry - 100.0).abs() < 1e-9);
        assert!((r - 4.0).abs() < 1e-9, "SerreBord : r = min(base, largeur)");
        assert!((sl - 96.0).abs() < 1e-9);
        assert!((tp1 - 102.4).abs() < 1e-9, "TP1 = entry + 0.6×r");
        assert!((tp2 - 108.0).abs() < 1e-9);

        // Score faible (<8) : r = 5×0.7 = 3.5 < largeur → r conservé.
        let (_, _, _, _, _, r_faible) =
            gen.build_levels(true, 100.0, 96.0, &out, true, 6).unwrap();
        assert!((r_faible - 3.5).abs() < 1e-9, "palier faible ×0.7");

        // Score fort (≥12) : r = 5×1.3 = 6.5 → serré à la largeur 4.
        let (_, _, _, _, _, r_fort) =
            gen.build_levels(true, 100.0, 96.0, &out, true, 13).unwrap();
        assert!((r_fort - 4.0).abs() < 1e-9, "palier fort borné par la zone");
    }

    #[test]
    fn build_levels_sans_sl_fixe_aucun_signal() {
        // Aucune valeur configurée → pas de signal (échec visible).
        let c = AssetCalibration::detect("XAUUSD", "M15");
        let out = SmcOutput::default();
        let gen = SignalGenerator::new();
        assert!(gen.build_levels(true, 100.0, 96.0, &out, true, 10).is_none());
    }

    #[test]
    fn build_levels_garde_skip_zone_large() {
        let c = AssetCalibration::detect("XAUUSD", "M15");
        let out = SmcOutput::default();
        let gen = fixe(3.0, GardeSlFixe::SkipZoneLarge);
        // Zone [90;100] largeur 10 > base 3 → signal abandonné.
        assert!(gen.build_levels(true, 100.0, 90.0, &out, true, 10).is_none());
        // Zone étroite → r = 3.
        let (_, _, _, _, _, r) =
            gen.build_levels(true, 100.0, 98.0, &out, true, 10).unwrap();
        assert!((r - 3.0).abs() < 1e-9);
    }

    #[test]
    fn build_levels_tp3_dol_plafonne_3r() {
        // Décision DoL≤3R 28/08 : TP3 = min(DoL, 3R). Base 2.3, zone
        // [99.2;100] → SerreBord donne r=0.8… non : largeur 0.8 < 2.3.
        // Ici base 1 → r=1, TP2=102, plafond 3R=103 ; PDH=115 → plafonné.
        let c = AssetCalibration::detect("XAUUSD", "M15");
        let mut out = SmcOutput::default();
        let gen = fixe(1.0, GardeSlFixe::Aucune);
        out.liquidite.pdh_active = Some(115.0);
        let (_, _sl, _tp1, tp2, tp3, r) =
            gen.build_levels(true, 100.0, 99.2, &out, true, 10).unwrap();
        assert!((r - 1.0).abs() < 1e-9);
        assert!((tp2 - 102.0).abs() < 1e-9);
        assert!((tp3 - 103.0).abs() < 1e-9, "TP3 = min(PDH=115, 3R=103)");
        // DoL proche entre TP2 (102) et 3R (103) → conservé tel quel.
        out.liquidite.pdh_active = Some(102.5);
        let (_, _, _, _, tp3_proche, _) =
            gen.build_levels(true, 100.0, 99.2, &out, true, 10).unwrap();
        assert!((tp3_proche - 102.5).abs() < 1e-9, "DoL proche conservé");
    }
