//! Statistiques agrégées et courbe equity pour la stratégie Rockets.
//! Fonctions séparées de rockets_feedback.rs pour respecter la limite de taille.

use common::{Result, TradingError};
use sqlx::{Row, SqlitePool};

// ── Statistiques temporelles ──────────────────────────────────────────────────

/// Taux de réussite et pnl moyen sur les N dernières heures (tous tickers+phases).
/// Retourne (nb_trades, win_rate 0.0–1.0, pnl_moyen_r).
/// Utilisé pour injecter le contexte de marché global dans le prompt LLM.
pub async fn taux_reussite_recent(pool: &SqlitePool, heures: i64) -> (i64, f64, f64) {
    let seuil = chrono::Utc::now().timestamp() - heures * 3600;
    let row = sqlx::query(
        "SELECT COUNT(*) as nb,
                AVG(CAST(gagnant AS REAL)) as wr,
                AVG(pnl_r) as pnl
         FROM rockets_feedback
         WHERE verdict IS NOT NULL AND cree_le >= ?",
    )
    .bind(seuil)
    .fetch_one(pool)
    .await;

    match row {
        Ok(r) => {
            let nb: i64 = r.get::<i64, _>("nb");
            let wr: f64 = r.get::<Option<f64>, _>("wr").unwrap_or(0.0);
            let pnl: f64 = r.get::<Option<f64>, _>("pnl").unwrap_or(0.0);
            (nb, wr, pnl)
        }
        Err(_) => (0, 0.0, 0.0),
    }
}

// ── Statistiques globales et par phase ───────────────────────────────────────

/// Statistiques globales — pour le endpoint `/api/rockets/monitoring`.
pub async fn stats_globales(pool: &SqlitePool) -> Result<serde_json::Value> {
    let row = sqlx::query(
        "SELECT COUNT(*) as nb_total,
                SUM(CASE WHEN verdict IS NOT NULL THEN 1 ELSE 0 END) as nb_clos,
                AVG(CASE WHEN verdict IS NOT NULL THEN gagnant END) as win_rate,
                AVG(CASE WHEN verdict IS NOT NULL THEN pnl_r END) as pnl_moyen_r
         FROM rockets_feedback",
    )
    .fetch_one(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;

    Ok(serde_json::json!({
        "nb_signals_total":      row.get::<i64, _>("nb_total"),
        "nb_feedbacks_clotures": row.get::<i64, _>("nb_clos"),
        "win_rate_global":       row.get::<Option<f64>, _>("win_rate").unwrap_or(0.0),
        "pnl_moyen_r":           row.get::<Option<f64>, _>("pnl_moyen_r").unwrap_or(0.0),
        "derniere_maj":          chrono::Utc::now().timestamp(),
    }))
}

/// Statistiques par phase — pour le monitoring ML.
pub async fn stats_par_phase(pool: &SqlitePool) -> Result<Vec<serde_json::Value>> {
    let rows = sqlx::query(
        "SELECT phase,
                COUNT(*) as nb_trades,
                SUM(gagnant) as nb_gagnants,
                AVG(CASE WHEN gagnant = 1 THEN conviction_llm END) as conviction_win,
                AVG(CASE WHEN gagnant = 0 THEN conviction_llm END) as conviction_lose,
                AVG(pnl_r) as pnl_r_moyen
         FROM rockets_feedback
         WHERE verdict IS NOT NULL
         GROUP BY phase
         ORDER BY nb_trades DESC",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;

    Ok(rows
        .iter()
        .map(|r| {
            let nb: i64 = r.get("nb_trades");
            let wins: i64 = r.get::<Option<i64>, _>("nb_gagnants").unwrap_or(0);
            serde_json::json!({
                "phase":           r.get::<String, _>("phase"),
                "nb_trades":       nb,
                "win_rate":        if nb > 0 { wins as f64 / nb as f64 } else { 0.0 },
                "conviction_win":  r.get::<Option<f64>, _>("conviction_win"),
                "conviction_lose": r.get::<Option<f64>, _>("conviction_lose"),
                "pnl_r_moyen":     r.get::<Option<f64>, _>("pnl_r_moyen"),
            })
        })
        .collect())
}

// ── Courbe equity ─────────────────────────────────────────────────────────────

#[derive(Debug, serde::Serialize)]
pub struct EquityPoint {
    pub ticker: String,
    pub verdict: String,
    pub pnl_r: f64,
    pub equity_cumulee: f64,
    pub ferme_le: i64,
}

/// Retourne la série equity simulée cumulée depuis `rockets_signaux` (source de vérité).
/// `capital_initial` et `risk_montant` (ex: capital × 0.015) permettent de convertir R → €.
pub async fn courbe_equity(
    pool: &SqlitePool,
    capital_initial: f64,
    risk_montant: f64,
) -> Result<Vec<EquityPoint>> {
    let rows = sqlx::query(
        "SELECT ticker, LOWER(verdict) as verdict, prix_entree, prix_verdict, atr14,
                CAST(strftime('%s', maj_le) AS INTEGER) as ferme_le
         FROM rockets_signaux
         WHERE statut = 'ferme'
           AND UPPER(verdict) IN ('SL', 'TP1', 'TP2', 'TP3', 'INVALIDE')
           AND prix_verdict IS NOT NULL
           AND (UPPER(verdict) = 'INVALIDE' OR (atr14 IS NOT NULL AND atr14 > 0))
         ORDER BY maj_le ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;

    let mut equity = capital_initial;
    let mut points = Vec::with_capacity(rows.len());

    for r in &rows {
        let verdict: String = r.get("verdict");
        let pnl_r = if verdict == "invalide" {
            -1.0
        } else {
            let prix_entree: f64 = r.get("prix_entree");
            let prix_verdict: f64 = r.get("prix_verdict");
            let atr14: f64 = r.get("atr14");
            (prix_verdict - prix_entree) / atr14
        };
        equity += pnl_r * risk_montant;
        points.push(EquityPoint {
            ticker: r.get("ticker"),
            verdict,
            pnl_r,
            equity_cumulee: equity,
            ferme_le: r.get("ferme_le"),
        });
    }

    Ok(points)
}
