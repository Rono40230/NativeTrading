use common::{Result, TradingError};
use sqlx::{Row, SqlitePool};

// ── Paramètres Straddle (BiDi) ───────────────────────────────────────────────

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct StraddleParams {
    pub atr_periode: i64,
    pub atr_seuil: f64,
    pub tp_mult_1: f64,
    pub tp_mult_2: f64,
    pub tp_mult_3: f64,
    pub sl_mult: f64,
    pub horizon_bougies: i64,
    pub trailing_atr: f64,
}

impl Default for StraddleParams {
    fn default() -> Self {
        Self {
            atr_periode: 14,
            atr_seuil: 1.5,
            tp_mult_1: 2.0,
            tp_mult_2: 3.5,
            tp_mult_3: 5.0,
            sl_mult: 0.5,
            horizon_bougies: 48,
            trailing_atr: 1.5,
        }
    }
}

pub async fn lire_straddle_params(pool: &SqlitePool) -> StraddleParams {
    let row = sqlx::query(
        "SELECT atr_periode, atr_seuil, tp_mult_1, tp_mult_2, tp_mult_3,
                sl_mult, horizon_bougies, trailing_atr
         FROM straddle_params WHERE id = 1",
    )
    .fetch_optional(pool)
    .await;

    match row {
        Ok(Some(r)) => StraddleParams {
            atr_periode: r.get("atr_periode"),
            atr_seuil: r.get("atr_seuil"),
            tp_mult_1: r.get("tp_mult_1"),
            tp_mult_2: r.get("tp_mult_2"),
            tp_mult_3: r.get("tp_mult_3"),
            sl_mult: r.get("sl_mult"),
            horizon_bougies: r.get("horizon_bougies"),
            trailing_atr: r.get("trailing_atr"),
        },
        _ => StraddleParams::default(),
    }
}

pub async fn sauvegarder_straddle_params(pool: &SqlitePool, p: &StraddleParams) -> Result<()> {
    sqlx::query(
        "INSERT INTO straddle_params
             (id, atr_periode, atr_seuil, tp_mult_1, tp_mult_2, tp_mult_3,
              sl_mult, horizon_bougies, trailing_atr, maj_le)
         VALUES (1, ?, ?, ?, ?, ?, ?, ?, ?, datetime('now'))
         ON CONFLICT(id) DO UPDATE SET
             atr_periode      = excluded.atr_periode,
             atr_seuil        = excluded.atr_seuil,
             tp_mult_1        = excluded.tp_mult_1,
             tp_mult_2        = excluded.tp_mult_2,
             tp_mult_3        = excluded.tp_mult_3,
             sl_mult          = excluded.sl_mult,
             horizon_bougies  = excluded.horizon_bougies,
             trailing_atr     = excluded.trailing_atr,
             maj_le           = excluded.maj_le",
    )
    .bind(p.atr_periode)
    .bind(p.atr_seuil)
    .bind(p.tp_mult_1)
    .bind(p.tp_mult_2)
    .bind(p.tp_mult_3)
    .bind(p.sl_mult)
    .bind(p.horizon_bougies)
    .bind(p.trailing_atr)
    .execute(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;
    Ok(())
}

// ── Paramètres SMC Directionnel (Uni) ────────────────────────────────────────

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct SmcParams {
    pub atr_periode: i64,
    pub score_min: i64,
    pub atr_tp1: f64,
    pub atr_tp2: f64,
    pub atr_tp3: f64,
    pub atr_sl: f64,
    pub horizon_bougies: i64,
}

impl Default for SmcParams {
    fn default() -> Self {
        Self {
            atr_periode: 14,
            score_min: 70,
            atr_tp1: 1.5,
            atr_tp2: 3.0,
            atr_tp3: 5.0,
            atr_sl: 1.0,
            horizon_bougies: 24,
        }
    }
}

pub async fn lire_smc_params(pool: &SqlitePool) -> SmcParams {
    let row = sqlx::query(
        "SELECT atr_periode, score_min, atr_tp1, atr_tp2, atr_tp3, atr_sl, horizon_bougies
         FROM smc_params WHERE id = 1",
    )
    .fetch_optional(pool)
    .await;

    match row {
        Ok(Some(r)) => SmcParams {
            atr_periode: r.get("atr_periode"),
            score_min: r.get("score_min"),
            atr_tp1: r.get("atr_tp1"),
            atr_tp2: r.get("atr_tp2"),
            atr_tp3: r.get("atr_tp3"),
            atr_sl: r.get("atr_sl"),
            horizon_bougies: r.get("horizon_bougies"),
        },
        _ => SmcParams::default(),
    }
}

pub async fn sauvegarder_smc_params(pool: &SqlitePool, p: &SmcParams) -> Result<()> {
    sqlx::query(
        "INSERT INTO smc_params
             (id, atr_periode, score_min, atr_tp1, atr_tp2, atr_tp3, atr_sl,
              horizon_bougies, maj_le)
         VALUES (1, ?, ?, ?, ?, ?, ?, ?, datetime('now'))
         ON CONFLICT(id) DO UPDATE SET
             atr_periode     = excluded.atr_periode,
             score_min       = excluded.score_min,
             atr_tp1         = excluded.atr_tp1,
             atr_tp2         = excluded.atr_tp2,
             atr_tp3         = excluded.atr_tp3,
             atr_sl          = excluded.atr_sl,
             horizon_bougies = excluded.horizon_bougies,
             maj_le          = excluded.maj_le",
    )
    .bind(p.atr_periode)
    .bind(p.score_min)
    .bind(p.atr_tp1)
    .bind(p.atr_tp2)
    .bind(p.atr_tp3)
    .bind(p.atr_sl)
    .bind(p.horizon_bougies)
    .execute(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;
    Ok(())
}
