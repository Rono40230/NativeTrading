//! Clôtures pour la simulation de capital (api::capital_simule).
//! Extrait de signaux.rs — limite de 600 lignes par fichier (pre-commit).

use crate::{Database, TradingError};
use sqlx::Row;

/// Clôture remplie pour la simulation de capital (ordre chronologique).
#[derive(Debug)]
pub struct ClotureCapital {
    pub id: String,
    pub ferme_le: i64,
    /// R réalisé (sortie réelle) — 0 si absent.
    pub r: f64,
}

impl Database {
    /// Clôtures REMPLIES d'une stratégie — matière première de la simulation
    /// de capital composée. Un ordre jamais touché n'engage pas de capital
    /// (heure_entree IS NOT NULL), les expirés/BE participent avec leur R réel.
    pub async fn clotures_pour_capital(&self, id: &str) -> crate::Result<Vec<ClotureCapital>> {
        let rows = sqlx::query(
            "SELECT id, ferme_le, r_realise FROM signaux
             WHERE strategie = ? AND statut = 'Fermé' AND verdict IS NOT NULL
               AND heure_entree IS NOT NULL AND ferme_le IS NOT NULL
             ORDER BY ferme_le ASC, id ASC",
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| TradingError::Database(e.to_string()))?;
        Ok(rows
            .iter()
            .map(|r| ClotureCapital {
                id: r.get("id"),
                ferme_le: r.get("ferme_le"),
                r: r.try_get::<f64, _>("r_realise").ok().unwrap_or(0.0),
            })
            .collect())
    }
}
