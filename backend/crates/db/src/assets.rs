use chrono::Utc;
use common::{Result, TradingError};
use serde::{Deserialize, Serialize};
use sqlx::Row;

use crate::Database;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetDb {
    pub id: String,
    pub nom: String,
    #[serde(rename = "type")]
    pub type_asset: String,
    pub source: String,
    pub actif: bool,
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
            "SELECT id, nom, type, source, actif, cree_le FROM assets WHERE actif = 1 ORDER BY type, id"
        } else {
            "SELECT id, nom, type, source, actif, cree_le FROM assets ORDER BY type, id"
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
                cree_le: r.get("cree_le"),
            })
            .collect())
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
}
