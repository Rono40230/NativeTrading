//! Univers actions US de la veille Rockets (étape A, 31/08).
//! Alimenté par le répertoire officiel NASDAQ Trader puis noté par le
//! pré-screen trend template. Le cure propriétaire vit dans `etat`
//! ('exclu' survit aux ré-énumérations).

use common::{Result, TradingError};
use sqlx::Row;

use crate::Database;

/// Ligne d'univers renvoyée aux endpoints/scan.
#[derive(Debug, serde::Serialize)]
pub struct LigneUniversAction {
    pub ticker: String,
    pub nom: String,
    pub exchange: String,
    pub etat: String,
    pub maj_le: i64,
}

/// Une ligne brute du répertoire NASDAQ Trader, après filtrage.
#[derive(Debug, Clone)]
pub struct TickerFiltre {
    pub ticker: String,
    pub nom: String,
    pub exchange: String,
}

impl Database {
    /// Insère/met à jour l'énumération. Les lignes marquées 'exclu' par le
    /// propriétaire ne sont jamais réactivées (le cure prime).
    pub async fn maj_univers_actions(&self, lignes: &[TickerFiltre]) -> Result<usize> {
        let maintenant = chrono::Utc::now().timestamp();
        for l in lignes {
            let sql = "INSERT INTO univers_actions (ticker, nom, exchange, maj_le, cree_le)
                       VALUES (?, ?, ?, ?, ?)
                       ON CONFLICT(ticker) DO UPDATE SET
                           nom = excluded.nom,
                           exchange = excluded.exchange,
                           maj_le = excluded.maj_le";
            sqlx::query(sql)
                .bind(&l.ticker)
                .bind(&l.nom)
                .bind(&l.exchange)
                .bind(maintenant)
                .bind(maintenant)
                .execute(&self.pool)
                .await
                .map_err(|e| TradingError::Database(e.to_string()))?;
        }
        Ok(lignes.len())
    }

    /// Tickers actifs de l'univers (périmètre de scan).
    pub async fn univers_actions_actives(&self) -> Result<Vec<LigneUniversAction>> {
        let rows = sqlx::query(
            "SELECT ticker, nom, exchange, etat, maj_le FROM univers_actions
             WHERE etat = 'actif' ORDER BY ticker",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| TradingError::Database(e.to_string()))?;

        Ok(rows
            .iter()
            .map(|r| LigneUniversAction {
                ticker: r.get("ticker"),
                nom: r.get("nom"),
                exchange: r.get("exchange"),
                etat: r.get("etat"),
                maj_le: r.get("maj_le"),
            })
            .collect())
    }

    /// Exclure/réactiver un ticker (cure propriétaire).
    pub async fn maj_etat_ticker(&self, ticker: &str, etat: &str) -> Result<()> {
        sqlx::query("UPDATE univers_actions SET etat = ? WHERE ticker = ?")
            .bind(etat)
            .bind(ticker)
            .execute(&self.pool)
            .await
            .map_err(|e| TradingError::Database(e.to_string()))?;
        Ok(())
    }

    /// Insère des bougies D1 actions (upsert idempotent — le backfill peut
    /// être rejoué sans doublon).
    pub async fn inserer_bougies_actions(
        &self,
        ticker: &str,
        bougies: &[(i64, f64, f64, f64, f64, f64)],
    ) -> Result<u64> {
        let mut n = 0u64;
        for (ts, o, h, l, c, v) in bougies {
            let res = sqlx::query(
                "INSERT INTO bougies_actions (ticker, ts, open, high, low, close, volume)
                 VALUES (?, ?, ?, ?, ?, ?, ?)
                 ON CONFLICT(ticker, ts) DO UPDATE SET
                     open = excluded.open, high = excluded.high, low = excluded.low,
                     close = excluded.close, volume = excluded.volume",
            )
            .bind(ticker)
            .bind(ts)
            .bind(o)
            .bind(h)
            .bind(l)
            .bind(c)
            .bind(v)
            .execute(&self.pool)
            .await
            .map_err(|e| TradingError::Database(e.to_string()))?;
            n += res.rows_affected();
        }
        Ok(n)
    }

    /// Bougies D1 d'un ticker (ASC) pour le pré-screen et le classement.
    pub async fn bougies_actions(&self, ticker: &str) -> Result<Vec<(i64, f64, f64, f64, f64, f64)>> {
        let rows = sqlx::query(
            "SELECT ts, open, high, low, close, volume FROM bougies_actions
             WHERE ticker = ? ORDER BY ts ASC",
        )
        .bind(ticker)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| TradingError::Database(e.to_string()))?;

        Ok(rows
            .iter()
            .map(|r| {
                (
                    r.get::<i64, _>("ts"),
                    r.get::<f64, _>("open"),
                    r.get::<f64, _>("high"),
                    r.get::<f64, _>("low"),
                    r.get::<f64, _>("close"),
                    r.get::<f64, _>("volume"),
                )
            })
            .collect())
    }

    // ── Sélection backfill (étape A2) ──────────────────────────────────────

    /// Avancement du backfill : (total univers actif, avec bougies).
    pub async fn avancement_backfill(&self) -> Result<(usize, usize)> {
        let total: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM univers_actions WHERE etat = 'actif'",
        )
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0);
        let avec: i64 = sqlx::query_scalar(
            "SELECT COUNT(DISTINCT ticker) FROM bougies_actions",
        )
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0);
        Ok((total as usize, avec as usize))
    }

    /// Prochains tickers SANS bougies à backfiller : les prioritaires d'abord
    /// (liste de liquidité fournie), puis le reste par ordre alphabétique.
    pub async fn tickers_sans_bougies(
        &self,
        prioritaires: &[&str],
        limite: usize,
    ) -> Result<Vec<String>> {
        let mut out = Vec::new();
        // Prioritaires présents dans l'univers actif et sans bougies.
        for t in prioritaires {
            if out.len() >= limite {
                break;
            }
            let n: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM univers_actions u
                 WHERE u.ticker = ? AND u.etat = 'actif'
                   AND NOT EXISTS (SELECT 1 FROM bougies_actions b WHERE b.ticker = u.ticker)",
            )
            .bind(t)
            .fetch_one(&self.pool)
            .await
            .unwrap_or(0);
            if n > 0 {
                out.push(t.to_string());
            }
        }
        if out.len() < limite {
            let bornes = out.iter().map(|t| format!("'{}'", t.replace('\'', "''"))).collect::<Vec<_>>().join(",");
            let sql = format!(
                "SELECT u.ticker FROM univers_actions u
                 WHERE u.etat = 'actif'
                   AND NOT EXISTS (SELECT 1 FROM bougies_actions b WHERE b.ticker = u.ticker)
                   {} ORDER BY u.ticker LIMIT ?",
                if out.is_empty() { String::new() } else { format!("AND u.ticker NOT IN ({bornes})") }
            );
            let rows = sqlx::query(&sql)
                .bind((limite - out.len()) as i64)
                .fetch_all(&self.pool)
                .await
                .map_err(|e| TradingError::Database(e.to_string()))?;
            for r in rows {
                out.push(r.get::<String, _>("ticker"));
            }
        }
        Ok(out)
    }

    /// Tickers DÉJÀ backfillés, les moins récemment rafraîchis d'abord
    /// (MAX(ts) le plus ancien en tête) — tournante de rafraîchissement.
    pub async fn tickers_a_rafraichir(&self, limite: usize) -> Result<Vec<String>> {
        let rows = sqlx::query(
            "SELECT ticker FROM bougies_actions GROUP BY ticker
             ORDER BY MAX(ts) ASC LIMIT ?",
        )
        .bind(limite as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| TradingError::Database(e.to_string()))?;
        Ok(rows.iter().map(|r| r.get::<String, _>("ticker")).collect())
    }
}
