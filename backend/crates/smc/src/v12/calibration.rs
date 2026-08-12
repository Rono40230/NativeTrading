//! Module 0 du Pine `smc_indicateur_v12.pine` (lignes 12-111) :
//! détection de l'actif + tables de calibration EXACTES (swing length, pip, SL mode,
//! seuils de scoring, pondérations).

/// Mode de stop-loss automatique (Pine `_autoSlMode`, lignes 78-81).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlMode {
    /// "Bas OB" — défaut pour actifs non reconnus.
    BasOb,
    /// "1× ATR" — XAU / NAS / DAX.
    Atr1x,
    /// "1.5× ATR" — XAG.
    Atr15x,
    /// "2× ATR" — BTC.
    Atr2x,
}

/// Calibration par actif — tables exactes du v12 Pine (Module 0).
///
/// Toutes les valeurs proviennent du Pine ; ne pas inventer.
#[derive(Debug, Clone)]
pub struct AssetCalibration {
    pub is_xau: bool,
    pub is_xag: bool,
    pub is_nas: bool,
    pub is_btc: bool,
    pub is_dax: bool,
    /// Vrai si l'actif correspond à un profil connu (XAU/XAG/NAS/BTC/DAX).
    /// Si faux → le scoring v12 doit retourner 0 (Pine `_assetReconnu`).
    pub asset_reconnu: bool,

    /// `_autoSwing` (Pine lignes 42-47) : longueur des pivots ta.pivothigh/low.
    pub swing_length: usize,
    /// `_pipValue` (Pine lignes 33-34).
    pub pip_value: f64,
    /// `_pvLot` (Pine lignes 35-36).
    pub pv_lot: f64,
    /// `_autoSlMode` (Pine lignes 78-81).
    pub sl_mode: SlMode,

    /// `_autoRocSeuil` (Pine ligne 51) — seuil ROC en bps pour la détection d'impulsion
    /// OB (MODULE 7). Constante 5 bps tous actifs.
    pub roc_seuil: f64,
    /// `_autoSeuilIB` (Pine lignes 53-55) — seuil d'imbalance en × ATR14 (MODULE 13b).
    pub seuil_ib: f64,

    // --- Seuils scoring (Pine lignes 986-991) — utilisés en phase 2.5 ---
    pub seuil_moyen: i32,
    pub seuil_fort: i32,
    pub seuil_instit: i32,
    pub score_max: i32,
    // --- Pondérations scoring ---
    pub w_fvg: i32,
    pub w_sweep: i32,
    pub w_atr: i32,
    pub w_ote: i32,
    pub w_kz: i32,
}

impl AssetCalibration {
    /// Détecte l'actif et calcule la calibration. `asset` insensible à la casse.
    ///
    /// Les tables ci-dessous sont des traductions LITTÉRALES du Pine (Module 0) :
    /// certains actifs partagent volontairement les mêmes valeurs (ex. NAS/DAX
    /// ont des poids scoring identiques). On lève donc le lint clippy correspondant.
    #[allow(clippy::if_same_then_else)]
    pub fn detect(asset: &str, timeframe: &str) -> Self {
        let a = asset.to_uppercase();
        let is_xau = a.contains("XAU");
        let is_xag = a.contains("XAG");
        let is_nas = a.contains("NAS") || a.contains("NDX") || a.contains("US100");
        let is_btc = a.contains("BTC");
        let is_dax = a.contains("DAX") || a.contains("GER40") || a.contains("DE30");
        let asset_reconnu = is_xau || is_xag || is_nas || is_btc || is_dax;

        // _autoSwing (Pine lignes 42-47) : TF ≤ M15 → 3, sinon 5 (spécifique asset inchangé).
        let tf_mins = tf_minutes(timeframe);
        let tf_m15 = tf_mins <= 15;
        let swing_length = if tf_m15 {
            3
        } else {
            match (is_xau, is_xag, is_nas, is_btc, is_dax) {
                (true, _, _, _, _) => 5,
                (_, true, _, _, _) => 4,
                _ => 5,
            }
        };

        // _pipValue / _pvLot (Pine lignes 33-36)
        let (pip_value, pv_lot) = if is_xau {
            (0.1, 10.0)
        } else if is_xag {
            (0.01, 50.0)
        } else if is_nas {
            (1.0, 20.0)
        } else if is_btc {
            (1.0, 1.0)
        } else if is_dax {
            (1.0, 25.0)
        } else {
            (1.0, 1.0)
        };

        // _autoSlMode (Pine lignes 78-81)
        let sl_mode = if is_btc {
            SlMode::Atr2x
        } else if is_xag {
            SlMode::Atr15x
        } else if is_xau || is_nas || is_dax {
            SlMode::Atr1x
        } else {
            SlMode::BasOb
        };

        // _autoRocSeuil (Pine ligne 51) : constante 5 bps tous actifs.
        let roc_seuil = 5.0_f64;
        // _autoSeuilIB (Pine lignes 53-55) : profil par asset (× ATR14).
        let seuil_ib = if is_xau {
            1.5
        } else if is_xag {
            1.2
        } else if is_btc {
            2.0
        } else {
            // NAS / DAX / défaut = 1.5
            1.5
        };

        // Seuils scoring (Pine lignes 986-991 ; PseudoCode PARTIE 1 MODULE 11)
        let (seuil_moyen, seuil_fort, seuil_instit, score_max) = if is_xau {
            (7, 10, 12, 13)
        } else if is_xag {
            (7, 99, 99, 14)
        } else if is_nas {
            (10, 15, 17, 19)
        } else if is_dax {
            (11, 16, 19, 21)
        } else if is_btc {
            (8, 99, 99, 15)
        } else {
            (7, 99, 99, 13)
        };

        // Pondérations (Pine MODULE 11)
        let (w_fvg, w_sweep, w_atr, w_ote, w_kz) = if is_xau {
            (5, 4, 2, 5, 3)
        } else if is_xag {
            (5, 2, 2, 2, 3)
        } else if is_nas {
            (4, 4, 2, 5, 3)
        } else if is_dax {
            (4, 4, 2, 5, 3)
        } else if is_btc {
            (3, 1, 2, 2, 2)
        } else {
            (4, 4, 2, 5, 3)
        };

        Self {
            is_xau,
            is_xag,
            is_nas,
            is_btc,
            is_dax,
            asset_reconnu,
            swing_length,
            pip_value,
            pv_lot,
            sl_mode,
            roc_seuil,
            seuil_ib,
            seuil_moyen,
            seuil_fort,
            seuil_instit,
            score_max,
            w_fvg,
            w_sweep,
            w_atr,
            w_ote,
            w_kz,
        }
    }
}

/// Parse un timeframe Pine en minutes. Reconnait M1/M5/M15/M30/H1/H4/D1/W1 ;
/// sinon tente d'extraire le préfixe numérique (ex. "45" → 45), défaut 15.
pub fn tf_minutes(tf: &str) -> u32 {
    match tf.trim().to_uppercase().as_str() {
        "M1" => 1,
        "M5" => 5,
        "M15" => 15,
        "M30" => 30,
        "H1" => 60,
        "H4" => 240,
        "D1" => 1440,
        "W1" => 10080,
        other => other
            .trim_end_matches(|c: char| !c.is_ascii_digit())
            .parse()
            .unwrap_or(15),
    }
}

/// Équivalent Pine `timeframe.in_seconds()` pour un timeframe texte.
pub fn tf_seconds(tf: &str) -> i64 {
    tf_minutes(tf) as i64 * 60
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xau_m15_swlength_3_pip_0_1() {
        let c = AssetCalibration::detect("XAUUSD", "M15");
        assert!(c.is_xau && c.asset_reconnu);
        assert_eq!(c.swing_length, 3, "M15 ⇒ swing 3");
        assert!((c.pip_value - 0.1).abs() < 1e-9);
        assert!((c.pv_lot - 10.0).abs() < 1e-9);
        assert_eq!(c.sl_mode, SlMode::Atr1x);
        assert_eq!(c.score_max, 13);
        assert_eq!(c.roc_seuil, 5.0, "_autoRocSeuil = 5 bps");
        assert_eq!(c.seuil_ib, 1.5, "XAU _autoSeuilIB = 1.5");
    }

    #[test]
    fn xau_h1_swlength_5() {
        let c = AssetCalibration::detect("XAUUSD", "H1");
        assert_eq!(c.swing_length, 5, "H1 ⇒ swing 5");
    }

    #[test]
    fn btc_h1_atr2x_et_seuils() {
        let c = AssetCalibration::detect("BTCUSD", "H1");
        assert_eq!(c.sl_mode, SlMode::Atr2x);
        assert_eq!(c.seuil_moyen, 8);
        assert_eq!(c.score_max, 15);
        assert_eq!(c.w_sweep, 1);
        assert_eq!(c.seuil_ib, 2.0, "BTC _autoSeuilIB = 2.0");
        assert_eq!(c.roc_seuil, 5.0);
    }

    #[test]
    fn nas_us100_reconnu() {
        assert!(AssetCalibration::detect("US100", "M15").is_nas);
    }

    #[test]
    fn actif_inconnu_non_reconnu_et_defaut() {
        let c = AssetCalibration::detect("EURUSD", "M15");
        assert!(!c.asset_reconnu);
        assert_eq!(c.sl_mode, SlMode::BasOb);
    }
}
