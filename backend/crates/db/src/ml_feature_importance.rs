//! Persistance et lecture des importances de features XGBoost (P4).

use sqlx::{Row, SqlitePool};

/// Une feature et son importance (chute accuracy OOS lors de la permutation).
#[derive(Debug, Clone)]
pub struct FeatureImportance {
    pub feature_idx: i64,
    pub feature_nom: String,
    pub importance: f64,
}

/// Insère un batch d'importances pour une stratégie donnée.
/// Remplace les entrées précédentes en insérant simplement de nouvelles lignes
/// (les requêtes lisent toujours le calcul le plus récent via ORDER BY calcule_le DESC).
pub async fn inserer_importances(
    pool: &SqlitePool,
    strategie: &str,
    importances: &[FeatureImportance],
) -> anyhow::Result<()> {
    for fi in importances {
        sqlx::query(
            "INSERT INTO ml_feature_importance (strategie, feature_idx, feature_nom, importance)
             VALUES (?, ?, ?, ?)",
        )
        .bind(strategie)
        .bind(fi.feature_idx)
        .bind(&fi.feature_nom)
        .bind(fi.importance)
        .execute(pool)
        .await?;
    }
    Ok(())
}

/// Retourne les N features les plus importantes pour une stratégie (calcul le plus récent).
pub async fn lire_top_importances(
    pool: &SqlitePool,
    strategie: &str,
    top_n: i64,
) -> anyhow::Result<Vec<FeatureImportance>> {
    // Sous-requête : récupérer la date du dernier calcul pour cette stratégie
    let rows = sqlx::query(
        r#"SELECT feature_idx, feature_nom, importance
           FROM ml_feature_importance
           WHERE strategie = ?
             AND calcule_le = (
               SELECT MAX(calcule_le) FROM ml_feature_importance WHERE strategie = ?
             )
           ORDER BY importance DESC
           LIMIT ?"#,
    )
    .bind(strategie)
    .bind(strategie)
    .bind(top_n)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .iter()
        .map(|r| FeatureImportance {
            feature_idx: r.get("feature_idx"),
            feature_nom: r.get("feature_nom"),
            importance: r.get("importance"),
        })
        .collect())
}
