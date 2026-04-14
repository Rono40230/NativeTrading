//! Snapshot des features ML au moment d'émission d'un signal Rockets.
//! Sert de fondation pour le fine-tuning stratégie-spécifique (P3).

use sqlx::{Row, SqlitePool};

/// Persiste le vecteur de features (52 f64) associé à un signal Rockets.
/// Appelé juste après l'INSERT dans rockets_signaux — signal_id déjà connu.
/// Si un snapshot existe déjà pour ce signal_id, l'opération est ignorée.
pub async fn inserer_snapshot(
    pool: &SqlitePool,
    signal_id: i64,
    ticker: &str,
    features: &[f64],
) -> anyhow::Result<()> {
    let json = serde_json::to_string(features)?;
    sqlx::query(
        "INSERT OR IGNORE INTO rockets_features_snapshot (signal_id, ticker, features_json)
         VALUES (?, ?, ?)",
    )
    .bind(signal_id)
    .bind(ticker)
    .bind(&json)
    .execute(pool)
    .await?;
    Ok(())
}

/// Retourne les features d'un signal donné, si le snapshot existe.
pub async fn lire_snapshot(
    pool: &SqlitePool,
    signal_id: i64,
) -> anyhow::Result<Option<Vec<f64>>> {
    let row = sqlx::query(
        "SELECT features_json FROM rockets_features_snapshot WHERE signal_id = ?",
    )
    .bind(signal_id)
    .fetch_optional(pool)
    .await?;

    match row {
        Some(r) => {
            let json: String = r.get("features_json");
            let features: Vec<f64> = serde_json::from_str(&json)?;
            Ok(Some(features))
        }
        None => Ok(None),
    }
}

/// Retourne tous les snapshots avec leur label (1.0=TP, 0.0=SL/invalide).
/// Utilisé par P3 pour entraîner le modèle sur les trades clôturés.
pub async fn lire_snapshots_avec_labels(
    pool: &SqlitePool,
) -> anyhow::Result<Vec<(Vec<f64>, f64)>> {
    let rows = sqlx::query(
        r#"SELECT s.features_json,
               CASE WHEN rs.verdict IN ('TP1','TP2','TP3') THEN 1.0 ELSE 0.0 END AS label
        FROM rockets_features_snapshot s
        JOIN rockets_signaux rs ON rs.id = s.signal_id
        WHERE rs.statut = 'ferme' AND rs.verdict IS NOT NULL"#,
    )
    .fetch_all(pool)
    .await?;

    let mut result = Vec::with_capacity(rows.len());
    for row in rows {
        let json: String = row.get("features_json");
        let label: f64 = row.get("label");
        let features: Vec<f64> = serde_json::from_str(&json)?;
        result.push((features, label));
    }
    Ok(result)
}
