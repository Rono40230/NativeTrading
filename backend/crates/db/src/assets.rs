use chrono::Utc;
use common::{Result, TradingError};
use serde::{Deserialize, Serialize};
use sqlx::Row;

use crate::Database;

/// Asset vu par les workers d'ingestion : identité + colonnes de routing.
/// `symbol_bybit` détermine quel worker ingère l'asset
/// (`None` → non couvert par ce worker).
#[derive(Debug, Clone, Serialize)]
pub struct AssetWorker {
    pub id: String,
    pub source: String,
    pub symbol_bybit: Option<String>,
    pub actif: bool,
    /// Instrument Dukascopy pour le backfill historique (`None` → non
    /// disponible via le datafeed public, migration 0066).
    pub datafeed_dukascopy: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetDb {
    pub id: String,
    pub nom: String,
    #[serde(rename = "type")]
    pub type_asset: String,
    pub source: String,
    pub actif: bool,
    /// Si true, cet asset est inclus dans le réentraînement ML.
    /// Distinct du soft-delete `actif` : un asset peut être actif mais exclu du ML.
    pub ml_actif: bool,
    pub cree_le: i64,
}

impl Database {
    /// Retourne tous les assets actifs.
    pub async fn lister_assets(&self) -> Result<Vec<AssetDb>> {
        self.lister_assets_filtre(true).await
    }

    /// Retourne tous les assets (actifs + inactifs).
    pub async fn lister_tous_assets(&self) -> Result<Vec<AssetDb>> {
        self.lister_assets_filtre(false).await
    }

    async fn lister_assets_filtre(&self, actifs_seulement: bool) -> Result<Vec<AssetDb>> {
        let sql = if actifs_seulement {
            "SELECT id, nom, type, source, actif, COALESCE(ml_actif, actif) as ml_actif, cree_le FROM assets WHERE actif = 1 ORDER BY type, id"
        } else {
            "SELECT id, nom, type, source, actif, COALESCE(ml_actif, actif) as ml_actif, cree_le FROM assets ORDER BY type, id"
        };
        let rows = sqlx::query(sql)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| TradingError::Database(e.to_string()))?;

        Ok(rows
            .iter()
            .map(|r| AssetDb {
                id: r.get("id"),
                nom: r.get("nom"),
                type_asset: r.get("type"),
                source: r.get("source"),
                actif: r.get::<i64, _>("actif") == 1,
                ml_actif: r.get::<i64, _>("ml_actif") == 1,
                cree_le: r.get("cree_le"),
            })
            .collect())
    }

    /// Liste tous les assets (actifs + inactifs) avec leurs colonnes de
    /// routing worker (`symbol_bybit`). Les workers filtrent ensuite
    /// selon `source`, `actif` et la présence du mapping.
    pub async fn lister_assets_worker(&self) -> Result<Vec<AssetWorker>> {
        let rows = sqlx::query(
            "SELECT id, source, symbol_bybit, actif, datafeed_dukascopy FROM assets ORDER BY type, id",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| TradingError::Database(e.to_string()))?;

        Ok(rows
            .iter()
            .map(|r| AssetWorker {
                id: r.get("id"),
                source: r.get("source"),
                symbol_bybit: r.get("symbol_bybit"),
                actif: r.get::<i64, _>("actif") == 1,
                datafeed_dukascopy: r.get("datafeed_dukascopy"),
            })
            .collect())
    }

    /// Instrument Dukascopy configuré pour un asset (backfill historique).
    /// `Ok(None)` si l'asset n'a pas de mapping (migration 0066).
    pub async fn instrument_dukascopy(&self, id_asset: &str) -> Result<Option<String>> {
        let row = sqlx::query("SELECT datafeed_dukascopy FROM assets WHERE id = ?")
            .bind(id_asset)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| TradingError::Database(e.to_string()))?;
        Ok(row.and_then(|r| r.get::<Option<String>, _>("datafeed_dukascopy")))
    }

    /// Ajoute un asset (INSERT OR IGNORE pour éviter les doublons).
    /// Si l'asset existait avec actif=0, le réactive.
    pub async fn ajouter_asset(
        &self,
        id: &str,
        nom: &str,
        type_asset: &str,
        source: &str,
    ) -> Result<()> {
        let now = Utc::now().timestamp();
        // Tenter réactivation si existait (soft-deleted)
        let nb = sqlx::query(
            "UPDATE assets SET actif = 1, nom = ?, type = ?, source = ?
             WHERE id = ? AND actif = 0",
        )
        .bind(nom)
        .bind(type_asset)
        .bind(source)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| TradingError::Database(e.to_string()))?
        .rows_affected();

        if nb == 0 {
            // Vérifier si déjà actif
            let existe: i64 =
                sqlx::query("SELECT COUNT(*) as n FROM assets WHERE id = ? AND actif = 1")
                    .bind(id)
                    .fetch_one(&self.pool)
                    .await
                    .map_err(|e| TradingError::Database(e.to_string()))?
                    .get("n");

            if existe > 0 {
                return Err(TradingError::Data(format!("L'asset '{}' existe déjà.", id)));
            }

            sqlx::query(
                "INSERT INTO assets (id, nom, type, source, actif, cree_le)
                 VALUES (?, ?, ?, ?, 1, ?)",
            )
            .bind(id)
            .bind(nom)
            .bind(type_asset)
            .bind(source)
            .bind(now)
            .execute(&self.pool)
            .await
            .map_err(|e| TradingError::Database(e.to_string()))?;
        }
        Ok(())
    }

    /// Soft-delete d'un asset (actif → 0). Les données historiques sont conservées.
    pub async fn supprimer_asset(&self, id: &str) -> Result<()> {
        let nb = sqlx::query("UPDATE assets SET actif = 0 WHERE id = ? AND actif = 1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| TradingError::Database(e.to_string()))?
            .rows_affected();

        if nb == 0 {
            return Err(TradingError::Data(format!(
                "Asset '{}' introuvable ou déjà supprimé.",
                id
            )));
        }
        Ok(())
    }

    /// Active ou désactive l'inclusion d'un asset dans le réentraînement ML.
    /// Indépendant du soft-delete `actif` : un asset peut être affiché mais exclu du ML.
    pub async fn set_ml_actif(&self, id: &str, valeur: bool) -> Result<()> {
        sqlx::query("UPDATE assets SET ml_actif = ? WHERE id = ?")
            .bind(if valeur { 1i64 } else { 0i64 })
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| TradingError::Database(e.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn db_test() -> Database {
        let db = Database::new(":memory:").await.expect("DB mémoire");
        db.run_migrations().await.expect("migrations OK");
        db
    }

    #[tokio::test]
    async fn instruments_dukascopy_mappes_par_la_migration() {
        let db = db_test().await;
        // Mapping vérifié empiriquement sur le datafeed (migration 0066).
        for (id, instrument) in [
            ("XAUUSD", "XAUUSD"),
            ("XAGUSD", "XAGUSD"),
            ("EURUSD", "EURUSD"),
            ("BTC", "BTCUSD"),
            ("NAS100", "USATECHIDXUSD"),
            ("DAX", "DEUIDXEUR"),
            ("SP500", "USA500IDXUSD"),
        ] {
            let obtenu = db
                .instrument_dukascopy(id)
                .await
                .unwrap_or_else(|e| panic!("lecture {} : {}", id, e));
            assert_eq!(obtenu, Some(instrument.to_string()), "asset {}", id);
        }
        // Asset sans mapping (crypto Binance hors BTC) → None, pas d'erreur.
        assert_eq!(db.instrument_dukascopy("ETH").await.unwrap(), None);
        // Asset inexistant → None (aucune panic).
        assert_eq!(db.instrument_dukascopy("INCONNU").await.unwrap(), None);
    }

    #[tokio::test]
    async fn lister_assets_worker_expose_le_mapping() {
        let db = db_test().await;
        let assets = db.lister_assets_worker().await.expect("liste workers");
        assert!(assets.len() > 10, "catalogue pré-peuplé");
        let nas = assets
            .iter()
            .find(|a| a.id == "NAS100")
            .expect("NAS100 présent");
        assert_eq!(nas.datafeed_dukascopy.as_deref(), Some("USATECHIDXUSD"));
        let eth = assets.iter().find(|a| a.id == "ETH").expect("ETH présent");
        assert_eq!(eth.datafeed_dukascopy, None);
    }
}
