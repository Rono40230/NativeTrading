//! Blacklist des tickers Rockets problématiques (prix figé, doublons permanents).
//! Un ticker blacklisté est ignoré par le scan AVANT toute inférence ML.

use crate::Database;
use common::{Result, TradingError};

impl Database {
    /// Retourne true si le ticker est blacklisté.
    pub async fn est_blackliste_rockets(&self, ticker: &str) -> bool {
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM rockets_blacklist WHERE ticker = ?",
        )
        .bind(ticker)
        .fetch_one(&self.pool)
        .await
        .map(|n| n > 0)
        .unwrap_or(false)
    }

    /// Ajoute un ticker à la blacklist (idempotent — INSERT OR IGNORE).
    pub async fn blacklister_ticker_rockets(&self, ticker: &str, raison: &str) -> Result<()> {
        sqlx::query(
            "INSERT OR IGNORE INTO rockets_blacklist (ticker, raison) VALUES (?, ?)",
        )
        .bind(ticker)
        .bind(raison)
        .execute(&self.pool)
        .await
        .map_err(|e| TradingError::Database(e.to_string()))?;
        Ok(())
    }

    /// Auto-blacklist : si un ticker apparaît ≥ seuil fois avec le même prix arrondi
    /// dans les dernières `heures` heures, il est blacklisté automatiquement.
    /// Retourne true si le ticker vient d'être blacklisté.
    pub async fn auto_blacklist_si_doublon(
        &self,
        ticker: &str,
        heures: i64,
        seuil: i64,
    ) -> Result<bool> {
        // Compter les signaux récents avec le même prix (arrondi à 4 décimales)
        let n: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM rockets_signaux
             WHERE ticker = ?
               AND cree_le >= datetime('now', ? || ' hours')
               AND ROUND(prix_entree, 4) = (
                   SELECT ROUND(prix_entree, 4) FROM rockets_signaux
                   WHERE ticker = ?
                   ORDER BY cree_le DESC LIMIT 1
               )",
        )
        .bind(ticker)
        .bind(-heures)
        .bind(ticker)
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0);

        if n >= seuil {
            let raison = format!(
                "Auto-ban : {} signaux avec le même prix en {}h (seuil={})",
                n, heures, seuil
            );
            self.blacklister_ticker_rockets(ticker, &raison).await?;
            tracing::warn!(
                "🚫 Ticker '{}' auto-blacklisté : {} doublons prix en {}h",
                ticker, n, heures
            );
            return Ok(true);
        }
        Ok(false)
    }
}
