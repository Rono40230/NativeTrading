cat << 'INNER_EOF' >> crates/api/src/ml_retrain_job.rs

async fn inserer_importances_defaut(
    pool: &sqlx::SqlitePool,
    strategie: &str,
    noms: &[&str],
) {
    let fis: Vec<db::ml_feature_importance::FeatureImportance> = noms
        .iter()
        .enumerate()
        .map(|(i, &nom)| db::ml_feature_importance::FeatureImportance {
            feature_idx: i as i64,
            feature_nom: nom.to_string(),
            importance: 0.0,
        })
        .collect();
    let _ = db::ml_feature_importance::inserer_importances(pool, strategie, &fis).await;
}
INNER_EOF
