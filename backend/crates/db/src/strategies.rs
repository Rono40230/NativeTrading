//! Registre des stratégies (étape 2 — socle stratégique).
//!
//! La partie structurelle du manifeste vit dans le code ; cette table porte
//! le pilotable : état de vie (Officielle / Observation / Construction),
//! son Telegram, allocation (capital + risque).

use anyhow::Result;

use sqlx::Row;

use crate::Database;

#[derive(Debug, Clone, PartialEq)]
pub struct StrategieRegistre {
    pub id: String,
    pub etat: String,
    pub notifications: bool,
    pub capital: f64,
    pub risque_pct: f64,
}

impl Default for StrategieRegistre {
    fn default() -> Self {
        Self {
            id: String::new(),
            etat: "Construction".into(),
            notifications: false,
            capital: 0.0,
            risque_pct: 1.0,
        }
    }
}

impl Database {
    /// Toutes les stratégies enregistrées.
    pub async fn lire_strategies(&self) -> Result<Vec<StrategieRegistre>> {
        let rows = sqlx::query(
            "SELECT id, etat, notifications, capital, risque_pct FROM strategies ORDER BY id",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .iter()
            .map(|r| StrategieRegistre {
                id: r.get("id"),
                etat: r.get("etat"),
                notifications: r.get::<i64, _>("notifications") != 0,
                capital: r.get("capital"),
                risque_pct: r.get("risque_pct"),
            })
            .collect())
    }

    /// Une stratégie par id.
    pub async fn lire_strategie(&self, id: &str) -> Result<Option<StrategieRegistre>> {
        let row = sqlx::query(
            "SELECT id, etat, notifications, capital, risque_pct FROM strategies WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| StrategieRegistre {
            id: r.get("id"),
            etat: r.get("etat"),
            notifications: r.get::<i64, _>("notifications") != 0,
            capital: r.get("capital"),
            risque_pct: r.get("risque_pct"),
        }))
    }

    /// Met à jour les champs pilotables (état, son, allocation).
    pub async fn maj_strategie(
        &self,
        id: &str,
        etat: &str,
        notifications: bool,
        capital: f64,
        risque_pct: f64,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE strategies SET etat = ?, notifications = ?, capital = ?, risque_pct = ? WHERE id = ?",
        )
        .bind(etat)
        .bind(notifications as i64)
        .bind(capital)
        .bind(risque_pct.clamp(0.1, 10.0))
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Enregistre une stratégie absente du registre (appelé au boot par le
    /// registre code — INSERT OR IGNORE, la DB reste maîtresse des réglages).
    pub async fn enregistrer_si_absente(&self, s: &StrategieRegistre) -> Result<()> {
        sqlx::query(
            "INSERT OR IGNORE INTO strategies (id, etat, notifications, capital, risque_pct)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&s.id)
        .bind(&s.etat)
        .bind(s.notifications as i64)
        .bind(s.capital)
        .bind(s.risque_pct)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
