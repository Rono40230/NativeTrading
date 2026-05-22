use serde::{Deserialize, Serialize};

// ─── Query params ─────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct IndicatorsQuery {
    pub asset: String,
    pub tf: Option<String>,
    pub ema_periode: Option<usize>,
    pub ema_ma_type: Option<String>,
    pub rsi_periode: Option<usize>,
    pub macd_rapide: Option<usize>,
    pub macd_lente: Option<usize>,
    pub macd_signal: Option<usize>,
    pub bollinger_periode: Option<usize>,
    pub bollinger_stddev: Option<f64>,
    pub bollinger_ma_type: Option<String>,
    pub atr_periode: Option<usize>,
    pub ema: Option<bool>,
    pub rsi: Option<bool>,
    pub macd: Option<bool>,
    pub bollinger: Option<bool>,
    pub atr: Option<bool>,
    pub smc_ob: Option<bool>,
    pub smc_ob_sensitivity: Option<f64>,
    pub smc_ob_mitigation: Option<String>,
    pub smc_ifvg: Option<bool>,
    pub smc_ifvg_show_last: Option<u32>,
    pub smc_ifvg_signal_pref: Option<String>,
    pub smc_ifvg_atr_mult: Option<f64>,
    pub smc_bpr: Option<bool>,
    pub smc_bpr_show_last: Option<u32>,
    pub smc_bpr_atr_mult: Option<f64>,
    pub smc_bpr_fenetre: Option<u32>,
    pub smc_bpr_mitigation: Option<String>,
    pub smc_imbalance: Option<bool>,
    pub smc_imb_show_last: Option<u32>,
    pub smc_imb_show_fvg: Option<bool>,
    pub smc_imb_show_og: Option<bool>,
    pub smc_imb_mitigation: Option<String>,
    pub smc_fib: Option<bool>,
    pub smc_tendance: Option<bool>,
    pub smc_liquidites: Option<bool>,
    pub smc_liq_swing_lookback: Option<u32>,
    pub smc_liq_swings: Option<bool>,
    pub smc_liq_sessions: Option<bool>,
    pub smc_liq_session_asie: Option<bool>,

    pub smc_liq_dwm: Option<bool>,
    pub smc_liq_dwm_nb: Option<u32>,
    /// Range session Asie
    pub smc_liq_asie_range: Option<bool>,
    pub smc_liq_asie_heure_debut: Option<u32>,
    pub smc_liq_asie_heure_fin: Option<u32>,
    pub smc_liq_asie_deviations_nb: Option<u32>,
    pub smc_liq_asie_nb_sessions: Option<u32>,
    /// BOS (Break of Structure) overlay
    pub smc_bos: Option<bool>,
    /// CHoCH (Change of Character) overlay
    pub smc_choch: Option<bool>,
    /// Si `true`, calcule et retourne les signaux pour tous les indicateurs actifs
    pub signaux: Option<bool>,
    pub limit: Option<u32>,
}

// ─── Types de réponse ─────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct PointSerie {
    pub time: i64,
    pub value: f64,
}

#[derive(Serialize)]
pub struct SeriesMacd {
    pub macd: Vec<PointSerie>,
    pub signal: Vec<PointSerie>,
    pub histogramme: Vec<PointSerie>,
}

#[derive(Serialize)]
pub struct SeriesBollinger {
    pub haute: Vec<PointSerie>,
    pub milieu: Vec<PointSerie>,
    pub basse: Vec<PointSerie>,
}

#[derive(Serialize)]
pub struct ReponseIndicators {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ema: Option<Vec<PointSerie>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rsi: Option<Vec<PointSerie>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub atr: Option<Vec<PointSerie>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub macd: Option<SeriesMacd>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bollinger: Option<SeriesBollinger>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_blocks: Option<Vec<smc::OrderBlock>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ifvg: Option<Vec<smc::Ifvg>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bpr: Option<Vec<smc::Bpr>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fibonacci: Option<smc::NiveauxFibonacci>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tendance: Option<smc::ResultatTendance>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub imbalance: Option<Vec<smc::ZoneImbalance>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub liquidites: Option<Vec<smc::NiveauLiquidite>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range_asie: Option<Vec<smc::RangeAsie>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bos: Option<smc::ResultatBos>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub choch: Option<smc::ResultatChoch>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signaux: Option<Vec<indicators::signaux::SignalIndicateur>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub atr_valeurs: Option<Vec<PointSerie>>,
}
