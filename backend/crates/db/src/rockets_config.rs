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
    /// true = Option 1 (vente ⅓ à chaque TP) ; false = Option 2 (SL progresse, pas de vente partielle)
    pub vente_partielle: bool,
    /// Multiplicateur ATR pour le Stop-Loss (R:R base = 1R)
    pub sl_mult: f64,
    /// TP1 = entrée + ATR × tp_mult_1 (doit être = sl_mult + 1.0)
    pub tp_mult_1: f64,
    /// TP2 = entrée + ATR × tp_mult_2 (doit être = sl_mult + 2.0)
    pub tp_mult_2: f64,
    /// TP3 = trailing stop basé sur ATR × tp_mult_3
    pub tp_mult_3: f64,
    /// Trailing stop distance en ATR
    pub trailing_atr: f64,
}

impl Default for RocketsConfig {
    fn default() -> Self {
        Self {
            score_min: 55,
            phases_actives: vec!["breakout".into(), "prelancement".into()],
            rsi_max: 85.0,
            rsi_min: 0.0,
            ratio_volume_min: 1.8,
            vol_marche_min: 500_000.0,
            vente_partielle: true,
            sl_mult: 1.0,
            tp_mult_1: 2.0,
            tp_mult_2: 3.0,
            tp_mult_3: 4.0,
            trailing_atr: 2.0,
        }
    }
}

pub async fn lire_config(pool: &SqlitePool) -> RocketsConfig {
    let row = sqlx::query(
        "SELECT score_min, phases_actives, rsi_max, rsi_min, ratio_volume_min,
                vol_marche_min, vente_partielle, sl_mult, tp_mult_1, tp_mult_2,
                tp_mult_3, trailing_atr
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
                tp_mult_1: r.try_get("tp_mult_1").unwrap_or(def.tp_mult_1),
                tp_mult_2: r.try_get("tp_mult_2").unwrap_or(def.tp_mult_2),
                tp_mult_3: r.try_get("tp_mult_3").unwrap_or(def.tp_mult_3),
                trailing_atr: r.try_get("trailing_atr").unwrap_or(def.trailing_atr),
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
              sl_mult, tp_mult_1, tp_mult_2, tp_mult_3, trailing_atr, maj_le)
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
           tp_mult_1          = excluded.tp_mult_1,
           tp_mult_2          = excluded.tp_mult_2,
           tp_mult_3          = excluded.tp_mult_3,
           trailing_atr       = excluded.trailing_atr,
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
    .bind(cfg.tp_mult_1)
    .bind(cfg.tp_mult_2)
    .bind(cfg.tp_mult_3)
    .bind(cfg.trailing_atr)
    .execute(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;
    Ok(())
}
