//! Journal de bord du propriétaire — notes horodatées attachées aux trades.

use crate::{Database, TradingError};
use serde::Serialize;
use sqlx::Row;

#[derive(Debug, Serialize)]
pub struct EntreeJournal {
    pub id: i64,
    pub signal_id: String,
    pub note: String,
    pub cree_le: i64,
}

impl Database {
    /// Fil de notes d'un trade (chrono croissant).
    pub async fn journal_du_signal(&self, signal_id: &str) -> Result<Vec<EntreeJournal>, TradingError> {
        let rows = sqlx::query(
            "SELECT id, signal_id, note, cree_le FROM journal_bord
             WHERE signal_id = ? ORDER BY cree_le ASC, id ASC",
        )
        .bind(signal_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| TradingError::Database(e.to_string()))?;
        Ok(rows
            .iter()
            .map(|r| EntreeJournal {
                id: r.get("id"),
                signal_id: r.get("signal_id"),
                note: r.get("note"),
                cree_le: r.get("cree_le"),
            })
            .collect())
    }

    /// Nombre de notes par signal (badge 📝 de l'historique) — tous les
    /// signaux confondus, une requête.
    pub async fn journal_comptes(&self) -> Result<Vec<(String, i64)>, TradingError> {
        let rows = sqlx::query(
            "SELECT signal_id, COUNT(*) AS n FROM journal_bord GROUP BY signal_id",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| TradingError::Database(e.to_string()))?;
        Ok(rows
            .iter()
            .map(|r| (r.get::<String, _>("signal_id"), r.get::<i64, _>("n")))
            .collect())
    }

    /// Ajoute une entrée (append-only). Retourne l'entrée créée.
    pub async fn ajouter_note_journal(
        &self,
        signal_id: &str,
        note: &str,
        cree_le: i64,
    ) -> Result<EntreeJournal, TradingError> {
        let res = sqlx::query("INSERT INTO journal_bord (signal_id, note, cree_le) VALUES (?, ?, ?)")
            .bind(signal_id)
            .bind(note)
            .bind(cree_le)
            .execute(&self.pool)
            .await
            .map_err(|e| TradingError::Database(e.to_string()))?;
        Ok(EntreeJournal {
            id: res.last_insert_rowid(),
            signal_id: signal_id.to_string(),
            note: note.to_string(),
            cree_le,
        })
    }

    /// Supprime une entrée (faute de frappe) — le reste du fil est intact.
    pub async fn supprimer_note_journal(&self, id: i64) -> Result<bool, TradingError> {
        let res = sqlx::query("DELETE FROM journal_bord WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| TradingError::Database(e.to_string()))?;
        Ok(res.rows_affected() > 0)
    }
}
