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
}

impl Default for RocketsConfig {
    fn default() -> Self {
        Self {
            score_min: 40,
            phases_actives: vec!["breakout".into(), "prelancement".into()],
            rsi_max: 85.0,
            rsi_min: 0.0,
            ratio_volume_min: 1.5,
            vol_marche_min: 500_000.0,
        }
    }
}

pub async fn lire_config(pool: &SqlitePool) -> RocketsConfig {
    let row = sqlx::query(
        "SELECT score_min, phases_actives, rsi_max, rsi_min, ratio_volume_min, vol_marche_min
         FROM rockets_config WHERE id = 1",
    )
    .fetch_optional(pool)
    .await;

    match row {
        Ok(Some(r)) => {
            let phases_json: String = r.get("phases_actives");
            let phases = serde_json::from_str(&phases_json).unwrap_or_else(|_| {
                vec!["breakout".into(), "prelancement".into()]
            });
            RocketsConfig {
                score_min: r.get("score_min"),
                phases_actives: phases,
                rsi_max: r.get("rsi_max"),
                rsi_min: r.get("rsi_min"),
                ratio_volume_min: r.get("ratio_volume_min"),
                vol_marche_min: r.get("vol_marche_min"),
            }
        }
        _ => RocketsConfig::default(),
    }
}

pub async fn sauvegarder_config(pool: &SqlitePool, cfg: &RocketsConfig) -> Result<()> {
    let phases_json = serde_json::to_string(&cfg.phases_actives)
        .unwrap_or_else(|_| r#"["breakout","prelancement"]"#.into());
    sqlx::query(
        "INSERT INTO rockets_config (id, score_min, phases_actives, rsi_max, rsi_min, ratio_volume_min, vol_marche_min, maj_le)
         VALUES (1, ?, ?, ?, ?, ?, ?, datetime('now'))
         ON CONFLICT(id) DO UPDATE SET
           score_min = excluded.score_min,
           phases_actives = excluded.phases_actives,
           rsi_max = excluded.rsi_max,
           rsi_min = excluded.rsi_min,
           ratio_volume_min = excluded.ratio_volume_min,
           vol_marche_min = excluded.vol_marche_min,
           maj_le = excluded.maj_le",
    )
    .bind(cfg.score_min)
    .bind(&phases_json)
    .bind(cfg.rsi_max)
    .bind(cfg.rsi_min)
    .bind(cfg.ratio_volume_min)
    .bind(cfg.vol_marche_min)
    .execute(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;
    Ok(())
}
