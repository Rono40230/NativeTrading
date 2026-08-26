//! Alertes de prix — CRUD + lecture pour le watcher du runtime.

use serde::{Deserialize, Serialize};
use sqlx::Row;

use crate::Database;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertePrix {
    pub id: i64,
    pub asset: String,
    pub prix: f64,
    /// 'au_dessus' | 'en_dessous'.
    pub sens: String,
    pub note: Option<String>,
    pub active: bool,
    pub cree_le: i64,
    pub declenchee_le: Option<i64>,
}

impl Database {
    /// Toutes les alertes (actives et déclenchées), les plus récentes d'abord.
    pub async fn lister_alertes_prix(&self) -> anyhow::Result<Vec<AlertePrix>> {
        let rows = sqlx::query(
            "SELECT id, asset, prix, sens, note, active, cree_le, declenchee_le
             FROM alertes_prix
             ORDER BY active DESC, cree_le DESC",
        )
        .fetch_all(self.pool())
        .await?;
        Ok(rows.into_iter().map(depuis_ligne).collect())
    }

    /// Alertes actives uniquement — vue du watcher runtime.
    pub async fn lister_alertes_actives(&self) -> anyhow::Result<Vec<AlertePrix>> {
        let rows = sqlx::query(
            "SELECT id, asset, prix, sens, note, active, cree_le, declenchee_le
             FROM alertes_prix
             WHERE active = 1",
        )
        .fetch_all(self.pool())
        .await?;
        Ok(rows.into_iter().map(depuis_ligne).collect())
    }

    pub async fn creer_alerte_prix(
        &self,
        asset: &str,
        prix: f64,
        sens: &str,
        note: Option<&str>,
    ) -> anyhow::Result<i64> {
        let res = sqlx::query(
            "INSERT INTO alertes_prix (asset, prix, sens, note, active, cree_le)
             VALUES (?, ?, ?, ?, 1, ?)",
        )
        .bind(asset)
        .bind(prix)
        .bind(sens)
        .bind(note)
        .bind(chrono::Utc::now().timestamp())
        .execute(self.pool())
        .await?;
        Ok(res.last_insert_rowid())
    }

    pub async fn supprimer_alerte_prix(&self, id: i64) -> anyhow::Result<u64> {
        let res = sqlx::query("DELETE FROM alertes_prix WHERE id = ?")
            .bind(id)
            .execute(self.pool())
            .await?;
        Ok(res.rows_affected())
    }

    /// Marque déclenchée (désarmée) — retourne l'alerte pour la notification.
    pub async fn declencher_alerte_prix(&self, id: i64) -> anyhow::Result<Option<AlertePrix>> {
        sqlx::query("UPDATE alertes_prix SET active = 0, declenchee_le = ? WHERE id = ? AND active = 1")
            .bind(chrono::Utc::now().timestamp())
            .bind(id)
            .execute(self.pool())
            .await?;
        let row = sqlx::query(
            "SELECT id, asset, prix, sens, note, active, cree_le, declenchee_le
             FROM alertes_prix WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(self.pool())
        .await?;
        Ok(row.map(depuis_ligne))
    }

    /// Réarme une alerte déclenchée.
    pub async fn rearmee_alerte_prix(&self, id: i64) -> anyhow::Result<u64> {
        let res = sqlx::query(
            "UPDATE alertes_prix SET active = 1, declenchee_le = NULL WHERE id = ? AND active = 0",
        )
        .bind(id)
        .execute(self.pool())
        .await?;
        Ok(res.rows_affected())
    }
}

fn depuis_ligne(r: sqlx::sqlite::SqliteRow) -> AlertePrix {
    AlertePrix {
        id: r.get("id"),
        asset: r.get("asset"),
        prix: r.get("prix"),
        sens: r.get("sens"),
        note: r.get("note"),
        active: r.get::<i64, _>("active") != 0,
        cree_le: r.get("cree_le"),
        declenchee_le: r.get("declenchee_le"),
    }
}
