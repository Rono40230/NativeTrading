use chrono::Utc;
use common::{Result, TradingError};
use sqlx::Row;

use crate::Database;

/// Données d'un entraînement à persister
pub struct EntrainementRecord {
    pub asset: String,
    pub timeframe: String,
    pub nb_bougies: i64,
    pub accuracy_xgb: f64,
    pub accuracy_lstm: f64,
    pub accuracy_finale: f64,
    pub accuracy_train: f64,
    pub accuracy_val: f64,
    pub duree_ms: i64,
    pub derive_detectee: bool,
}

impl Database {
    /// Insère un enregistrement d'entraînement dans l'historique.
    pub async fn inserer_historique_entrainement(&self, rec: &EntrainementRecord) -> Result<()> {
        let maintenant = Utc::now().timestamp();
        let derive = i64::from(rec.derive_detectee);
        sqlx::query(
            "INSERT INTO historique_entrainements
             (cree_le, asset, timeframe, nb_bougies, accuracy_rf, accuracy_lstm, accuracy_finale, accuracy_train, accuracy_val, duree_ms, derive_detectee)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(maintenant)
        .bind(&rec.asset)
        .bind(&rec.timeframe)
        .bind(rec.nb_bougies)
        .bind(rec.accuracy_xgb)
        .bind(rec.accuracy_lstm)
        .bind(rec.accuracy_finale)
        .bind(rec.accuracy_train)
        .bind(rec.accuracy_val)
        .bind(rec.duree_ms)
        .bind(derive)
        .execute(&self.pool)
        .await
        .map_err(|e| TradingError::Database(e.to_string()))?;
        Ok(())
    }

    /// Retourne les N derniers entraînements (plus récent en premier).
    pub async fn obtenir_historique_entrainements(
        &self,
        limit: i64,
    ) -> Result<Vec<serde_json::Value>> {
        let rows = sqlx::query(
            "SELECT id, cree_le, asset, timeframe, nb_bougies,
                    accuracy_rf, accuracy_lstm, accuracy_finale, accuracy_train, accuracy_val, duree_ms, derive_detectee
             FROM historique_entrainements
             ORDER BY cree_le DESC
             LIMIT ?",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| TradingError::Database(e.to_string()))?;

        Ok(rows
            .iter()
            .map(|r| {
                let derive_val: i64 = r.get("derive_detectee");
                serde_json::json!({
                    "id": r.get::<i64, _>("id"),
                    "cree_le": r.get::<i64, _>("cree_le"),
                    "asset": r.get::<String, _>("asset"),
                    "timeframe": r.get::<String, _>("timeframe"),
                    "nb_bougies": r.get::<i64, _>("nb_bougies"),
                    "accuracy_rf": r.get::<f64, _>("accuracy_rf"),
                    "accuracy_xgb": r.get::<f64, _>("accuracy_rf"),
                    "accuracy_lstm": r.get::<f64, _>("accuracy_lstm"),
                    "accuracy_finale": r.get::<f64, _>("accuracy_finale"),
                    "accuracy_train": r.get::<f64, _>("accuracy_train"),
                    "accuracy_val": r.get::<f64, _>("accuracy_val"),
                    "duree_ms": r.get::<i64, _>("duree_ms"),
                    "derive_detectee": derive_val != 0,
                })
            })
            .collect())
    }

    /// Retourne la moyenne d'accuracy_val sur les `nb` derniers entraînements.
    /// Utilisé par la surveillance 6h pour détecter une dégradation rapide.
    pub async fn accuracy_val_recente(&self, nb: i64) -> Result<Option<f64>> {
        let row = sqlx::query(
            "SELECT AVG(accuracy_val) as moy FROM \
             (SELECT accuracy_val FROM historique_entrainements ORDER BY cree_le DESC LIMIT ?)",
        )
        .bind(nb)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| TradingError::Database(e.to_string()))?;
        Ok(row.try_get::<Option<f64>, _>("moy").unwrap_or(None))
    }

    /// Détecte une dérive : accuracy_finale moyenne < seuil sur les 7 derniers jours.
    /// Retourne Ok(false) s'il n'y a pas encore de données.
    pub async fn detecter_derive_ml(&self, seuil: f64) -> Result<bool> {
        let depuis = Utc::now().timestamp() - 7 * 86400;
        let row = sqlx::query(
            "SELECT COALESCE(AVG(accuracy_finale), -1.0) as moy
             FROM historique_entrainements
             WHERE cree_le >= ?",
        )
        .bind(depuis)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| TradingError::Database(e.to_string()))?;

        let moy: Option<f64> = row.try_get("moy").ok();
        Ok(moy.is_some_and(|m| m < seuil))
    }

    /// Retourne (accuracy_train, accuracy_val_oos) du dernier entraînement.
    /// Utilisé pour calculer le gap train/OOS (détection d'overfitting).
    pub async fn dernier_gap_train_val(&self) -> Result<Option<(f64, f64)>> {
        let row = sqlx::query(
            "SELECT accuracy_train, accuracy_val FROM historique_entrainements
             ORDER BY cree_le DESC LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| TradingError::Database(e.to_string()))?;

        Ok(row.map(|r| (r.get("accuracy_train"), r.get("accuracy_val"))))
    }
}
