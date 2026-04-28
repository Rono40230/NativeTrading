//! Snapshot des features ML au moment d'émission d'un signal SMC Directionnel.
//!
//! 59 features = 52 OHLCV standard + 7 contextuelles SMC :
//!   [52] tendance       (0.0–25.0 pts)
//!   [53] order_block    (0.0–25.0 pts)
//!   [54] ifvg           (0.0–20.0 pts)
//!   [55] fibonacci      (0.0–15.0 pts)
//!   [56] imbalance      (0.0–15.0 pts)
//!   [57] kill_zone      (0.0=non, 1.0=oui)
//!   [58] sweep_detecte  (0.0=non, 1.0=oui)
//!
//! Label : `smc_feedback.verdict` = 'TP1' | 'TP2' | 'SL' (via JOIN).

use sqlx::{Row, SqlitePool};

/// 7 features contextuelles SMC regroupées pour éviter un excès d'arguments.
pub struct ContexteSmc {
    pub tendance: f64,
    pub order_block: f64,
    pub ifvg: f64,
    pub fibonacci: f64,
    pub imbalance: f64,
    pub kill_zone_active: bool,
    pub sweep_detecte: bool,
}

/// Construit le vecteur 59 features depuis les 52 OHLCV + 7 contextuelles SMC.
pub fn construire_features_59(features_ohlcv: &[f64], ctx: &ContexteSmc) -> Vec<f64> {
    let mut v = Vec::with_capacity(59);
    v.extend_from_slice(&features_ohlcv[..features_ohlcv.len().min(52)]);
    while v.len() < 52 {
        v.push(0.0);
    }
    v.push(ctx.tendance);
    v.push(ctx.order_block);
    v.push(ctx.ifvg);
    v.push(ctx.fibonacci);
    v.push(ctx.imbalance);
    v.push(if ctx.kill_zone_active { 1.0 } else { 0.0 });
    v.push(if ctx.sweep_detecte { 1.0 } else { 0.0 });
    v
}

// ── Écriture ──────────────────────────────────────────────────────────────────

/// Persiste le vecteur 59 features associé à un signal SMC.
/// Opération ignorée si un snapshot existe déjà pour ce signal (idempotent).
pub async fn inserer_snapshot(
    pool: &SqlitePool,
    signal_id: &str,
    ticker: &str,
    features: &[f64],
) -> anyhow::Result<()> {
    let json = serde_json::to_string(features)?;
    sqlx::query(
        "INSERT OR IGNORE INTO smc_features_snapshot (signal_id, ticker, features_json)
         VALUES (?, ?, ?)",
    )
    .bind(signal_id)
    .bind(ticker)
    .bind(&json)
    .execute(pool)
    .await?;
    Ok(())
}

// ── Lecture ───────────────────────────────────────────────────────────────────

/// Retourne les snapshots avec leur label (1.0=TP1/TP2, 0.0=SL) pour le fine-tuning.
/// Seuls les trades clôturés avec verdict non NULL sont retournés.
pub async fn lire_snapshots_avec_labels(
    pool: &SqlitePool,
) -> anyhow::Result<Vec<(Vec<f64>, f64)>> {
    let rows = sqlx::query(
        "SELECT s.features_json,
                CAST(COALESCE(f.pnl_r, CASE WHEN f.verdict IN ('TP1','TP2','TP3') THEN 1.0 ELSE 0.0 END) AS REAL) AS label
         FROM smc_features_snapshot s
         JOIN smc_feedback f ON f.signal_id = s.signal_id
         WHERE f.verdict IS NOT NULL",
    )
    .fetch_all(pool)
    .await?;

    let mut samples = Vec::with_capacity(rows.len());
    for row in rows {
        let json: String = row.get("features_json");
        let label: f64 = row.get("label");
        let features: Vec<f64> = serde_json::from_str(&json)?;
        samples.push((features, label));
    }
    Ok(samples)
}
