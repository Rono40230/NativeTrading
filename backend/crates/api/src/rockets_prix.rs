//! Helpers prix et feedback pour le suivi des signaux Rockets.

use db::rockets_feedback;

// ── Prix Binance ──────────────────────────────────────────────────────────────

#[derive(serde::Deserialize)]
pub struct BinancePrix {
    price: String,
}

pub async fn fetch_prix(client: &reqwest::Client, ticker: &str) -> Option<f64> {
    let url = format!(
        "https://api.binance.com/api/v3/ticker/price?symbol={}USDT",
        ticker
    );
    let resp: BinancePrix = client.get(&url).send().await.ok()?.json().await.ok()?;
    resp.price.parse::<f64>().ok()
}

// ── Feedback ──────────────────────────────────────────────────────────────────

/// Réconcilie le feedback Rockets après une clôture TP/SL.
/// `cree_le_str` est au format SQLite `datetime('now')` → "2026-04-06 14:32:00".
#[allow(clippy::too_many_arguments)]
pub async fn reconcilier_feedback(
    pool: &sqlx::SqlitePool,
    ticker: &str,
    signal_id: i64,
    verdict: &str,
    prix_entree: f64,
    prix_verdict: f64,
    atr14: Option<f64>,
    cree_le_str: &str,
) {
    let timestamp_signal = chrono::NaiveDateTime::parse_from_str(cree_le_str, "%Y-%m-%d %H:%M:%S")
        .map(|dt| dt.and_utc().timestamp())
        .unwrap_or_else(|_| chrono::Utc::now().timestamp());

    let atr = atr14.unwrap_or(1.0).max(1e-9);
    if let Err(e) = rockets_feedback::maj_feedback_verdict(
        pool,
        signal_id,
        verdict,
        prix_entree,
        prix_verdict,
        atr,
        timestamp_signal,
    )
    .await
    {
        tracing::warn!("Feedback Rockets {} id={}: {}", ticker, signal_id, e);
    }
}
