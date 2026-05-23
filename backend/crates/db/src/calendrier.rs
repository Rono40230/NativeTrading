use chrono::Utc;
use common::{Result, TradingError};
use sqlx::Row;

use crate::Database;

impl Database {
    /// Lit les annonces du cache si fetched_at >= now - ttl_sec
    pub async fn lire_calendrier_cache(&self, ttl_sec: i64) -> Result<Vec<serde_json::Value>> {
        let seuil = Utc::now().timestamp() - ttl_sec;
        let rows = sqlx::query(
            "SELECT id, date_heure, devise, titre, impact, precedent, prevision
             FROM calendrier_cache
             WHERE fetched_at >= ?
             ORDER BY date_heure ASC",
        )
        .bind(seuil)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| TradingError::Database(e.to_string()))?;

        Ok(rows
            .iter()
            .map(|r| {
                serde_json::json!({
                    "id":         r.get::<String, _>("id"),
                    "date_heure": r.get::<String, _>("date_heure"),
                    "devise":     r.get::<String, _>("devise"),
                    "titre":      r.get::<String, _>("titre"),
                    "impact":     r.get::<String, _>("impact"),
                    "precedent":  r.get::<Option<String>, _>("precedent"),
                    "prevision":  r.get::<Option<String>, _>("prevision"),
                })
            })
            .collect())
    }

    /// Retourne le prochain événement macro High-impact USD/EUR dans les 30 min.
    /// Résultat : Some((titre, minutes_restantes)) ou None.
    pub async fn fenetre_macro_smc_dans_minutes(&self) -> Result<Option<(String, i64)>> {
        self.prochain_evenement_macro_high(0, 30).await
    }

    /// Retourne le prochain événement High-impact dans la fenêtre [min_avant, max_avant] minutes.
    /// Utilisé par les pré-alertes avec un horizon configurable (ex : 0..90 min pour Straddle).
    /// Résultat : Some((titre, devise, minutes_restantes)) ou None.
    pub async fn prochain_evenement_macro_high(
        &self,
        min_avant: i64,
        max_avant: i64,
    ) -> Result<Option<(String, i64)>> {
        let events = self.lire_calendrier_cache(4 * 3600).await?;
        let maintenant = chrono::Utc::now();
        for ev in &events {
            if ev["impact"].as_str().unwrap_or("") != "High" {
                continue;
            }
            let date_str = ev["date_heure"].as_str().unwrap_or("");
            let dt = match chrono::DateTime::parse_from_rfc3339(date_str) {
                Ok(d) => d.with_timezone(&chrono::Utc),
                Err(_) => continue,
            };
            let diff_min = (dt - maintenant).num_minutes();
            if diff_min >= min_avant && diff_min <= max_avant {
                let titre = ev["titre"]
                    .as_str()
                    .unwrap_or("Événement macro")
                    .to_string();
                return Ok(Some((titre, diff_min)));
            }
        }
        Ok(None)
    }

    /// Efface et ré-insère toutes les annonces économiques (mise à jour du cache)
    pub async fn ecrire_calendrier_cache(&self, annonces: &[serde_json::Value]) -> Result<()> {
        let now = Utc::now().timestamp();
        sqlx::query("DELETE FROM calendrier_cache")
            .execute(&self.pool)
            .await
            .map_err(|e| TradingError::Database(e.to_string()))?;

        for a in annonces {
            sqlx::query(
                "INSERT INTO calendrier_cache
                 (id, date_heure, devise, titre, impact, precedent, prevision, fetched_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(a["id"].as_str().unwrap_or(""))
            .bind(a["date_heure"].as_str().unwrap_or(""))
            .bind(a["devise"].as_str().unwrap_or(""))
            .bind(a["titre"].as_str().unwrap_or(""))
            .bind(a["impact"].as_str().unwrap_or(""))
            .bind(a["precedent"].as_str())
            .bind(a["prevision"].as_str())
            .bind(now)
            .execute(&self.pool)
            .await
            .map_err(|e| TradingError::Database(e.to_string()))?;
        }
        Ok(())
    }
}
