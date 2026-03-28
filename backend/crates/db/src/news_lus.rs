use common::{Result, TradingError};
use sqlx::Row;

use crate::Database;

impl Database {
    /// Marque un article comme lu (insère ou ignore si déjà présent).
    pub async fn marquer_article_lu(&self, url: &str) -> Result<()> {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        sqlx::query("INSERT OR IGNORE INTO news_lus (url, lu_le) VALUES (?, ?)")
            .bind(url)
            .bind(ts)
            .execute(&self.pool)
            .await
            .map_err(|e| TradingError::Database(e.to_string()))?;

        Ok(())
    }

    /// Retourne la liste de toutes les URLs d'articles lus.
    pub async fn lire_articles_lus(&self) -> Result<Vec<String>> {
        let rows = sqlx::query("SELECT url FROM news_lus")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| TradingError::Database(e.to_string()))?;

        Ok(rows.iter().map(|r| r.get::<String, _>("url")).collect())
    }
}
