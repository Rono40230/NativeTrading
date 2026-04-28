use common::{Result, TradingError};
use sqlx::{Row, SqlitePool};

// ── Config scan ──────────────────────────────────────────────────────────────

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct RocketsConfig {
    pub score_min: i64,
    pub phases_actives: Vec<String>,
    pub rsi_max: f64,
    pub rsi_min: f64,
    pub ratio_volume_min: f64,
    pub vol_marche_min: f64,
    /// true = vente partielle pyramidale (pct_tp1 à TP1, pct_tp2 à TP2, reste en trailing)
    pub vente_partielle: bool,
    /// Multiplicateur ATR pour le Stop-Loss — définit 1 unité R
    pub sl_mult: f64,
    /// Coefficient trailing stop minimum (signal faible, stop serré)
    pub trailing_coeff_min: f64,
    /// Coefficient trailing stop maximum (signal explosif, stop large)
    pub trailing_coeff_max: f64,
    /// Score < seuil_score_faible : split 40/35/25%
    pub seuil_score_faible: i64,
    /// seuil_score_faible ≤ score < seuil_score_fort : split 25/25/50%
    /// score ≥ seuil_score_fort : split 15/20/65%
    pub seuil_score_fort: i64,
    pub pct_cloture_tp1: f64,
    pub pct_cloture_tp2: f64,
}

impl RocketsConfig {
    /// TP1 = entrée + ATR × (sl_mult + 0.5) → 1.5R minimum
    pub fn tp1_mult(&self) -> f64 { self.sl_mult + 0.5 }
    /// TP2 = entrée + ATR × (sl_mult + 1.5) → 2.5R
    pub fn tp2_mult(&self) -> f64 { self.sl_mult + 1.5 }
    /// Seuil R+3.5 : déplace le SL à TP2 quand atteint
    pub fn trailing_trigger_mult(&self) -> f64 { self.sl_mult + 2.5 }
}

impl Default for RocketsConfig {
    fn default() -> Self {
        Self {
            score_min: 65,
            phases_actives: vec!["breakout".into(), "prelancement".into()],
            rsi_max: 85.0,
            rsi_min: 0.0,
            ratio_volume_min: 1.0,
            vol_marche_min: 500_000.0,
            vente_partielle: true,
            sl_mult: 1.0,
            trailing_coeff_min: 1.5,
            trailing_coeff_max: 5.0,
            seuil_score_faible: 65,
            seuil_score_fort: 80,
            pct_cloture_tp1: 0.33,
            pct_cloture_tp2: 0.33,
        }
    }
}

pub async fn lire_config(pool: &SqlitePool) -> RocketsConfig {
    let row = sqlx::query(
        "SELECT score_min, phases_actives, rsi_max, rsi_min, ratio_volume_min,
                vol_marche_min, vente_partielle, sl_mult,
                trailing_coeff_min, trailing_coeff_max, seuil_score_faible, seuil_score_fort, pct_cloture_tp1, pct_cloture_tp2
         FROM rockets_config WHERE id = 1",
    )
    .fetch_optional(pool)
    .await;

    match row {
        Ok(Some(r)) => {
            let phases_json: String = r.get("phases_actives");
            let phases = serde_json::from_str(&phases_json)
                .unwrap_or_else(|_| vec!["breakout".into(), "prelancement".into()]);
            let def = RocketsConfig::default();
            RocketsConfig {
                score_min: r.get("score_min"),
                phases_actives: phases,
                rsi_max: r.get("rsi_max"),
                rsi_min: r.get("rsi_min"),
                ratio_volume_min: r.get("ratio_volume_min"),
                vol_marche_min: r.get("vol_marche_min"),
                vente_partielle: r.get::<i64, _>("vente_partielle") != 0,
                sl_mult: r.try_get("sl_mult").unwrap_or(def.sl_mult),
                trailing_coeff_min: r.try_get("trailing_coeff_min").unwrap_or(def.trailing_coeff_min),
                trailing_coeff_max: r.try_get("trailing_coeff_max").unwrap_or(def.trailing_coeff_max),
                seuil_score_faible: r.try_get("seuil_score_faible").unwrap_or(def.seuil_score_faible),
                seuil_score_fort: r.try_get("seuil_score_fort").unwrap_or(def.seuil_score_fort),
                pct_cloture_tp1: r.try_get("pct_cloture_tp1").unwrap_or(def.pct_cloture_tp1),
                pct_cloture_tp2: r.try_get("pct_cloture_tp2").unwrap_or(def.pct_cloture_tp2),
            }
        }
        _ => RocketsConfig::default(),
    }
}

pub async fn sauvegarder_config(pool: &SqlitePool, cfg: &RocketsConfig) -> Result<()> {
    let phases_json = serde_json::to_string(&cfg.phases_actives)
        .unwrap_or_else(|_| r#"["breakout","prelancement"]"#.into());
    sqlx::query(
        "INSERT INTO rockets_config
             (id, score_min, phases_actives, rsi_max, rsi_min,
              ratio_volume_min, vol_marche_min, vente_partielle,
              sl_mult, trailing_coeff_min, trailing_coeff_max,
              seuil_score_faible, seuil_score_fort, maj_le)
         VALUES (1, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, datetime('now'))
         ON CONFLICT(id) DO UPDATE SET
           score_min          = excluded.score_min,
           phases_actives     = excluded.phases_actives,
           rsi_max            = excluded.rsi_max,
           rsi_min            = excluded.rsi_min,
           ratio_volume_min   = excluded.ratio_volume_min,
           vol_marche_min     = excluded.vol_marche_min,
           vente_partielle    = excluded.vente_partielle,
           sl_mult            = excluded.sl_mult,
           trailing_coeff_min = excluded.trailing_coeff_min,
           trailing_coeff_max = excluded.trailing_coeff_max,
           seuil_score_faible = excluded.seuil_score_faible,
           seuil_score_fort   = excluded.seuil_score_fort,
           maj_le             = excluded.maj_le",
    )
    .bind(cfg.score_min)
    .bind(&phases_json)
    .bind(cfg.rsi_max)
    .bind(cfg.rsi_min)
    .bind(cfg.ratio_volume_min)
    .bind(cfg.vol_marche_min)
    .bind(if cfg.vente_partielle { 1i64 } else { 0i64 })
    .bind(cfg.sl_mult)
    .bind(cfg.trailing_coeff_min)
    .bind(cfg.trailing_coeff_max)
    .bind(cfg.seuil_score_faible)
    .bind(cfg.seuil_score_fort)
    .execute(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;
    Ok(())
}
