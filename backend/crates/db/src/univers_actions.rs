//! Univers actions US de la veille Rockets (étape A, 31/08).
//! Importé depuis le répertoire officiel NASDAQ Trader (31/08) puis géré par
//! états : 'actif' (périmètre backfill/scan), 'ecarte' (hors budget Tiingo),
//! 'sans_donnees' (delisting — jamais retenté). Le recalcul quotidien
//! (`recalculer_univers`) fait vivre le périmètre.

use common::{Result, TradingError};
use sqlx::Row;

use crate::Database;

/// Bornes de sélection de l'univers liquide. Réglables par le propriétaire
/// (table `configuration`) — défauts calibrés le 05/09 : plafond 450 =
/// 500 symboles uniques/mois (quota Tiingo gratuit) moins QQQ, les
/// entrants narratifs du mois et la marge d'aléa.
#[derive(Debug, Clone, Copy)]
pub struct BornesUnivers {
    pub taille_max: usize,
    pub dv_min: f64,
    pub prix_min: f64,
    pub seances_min: i64,
}

impl Default for BornesUnivers {
    fn default() -> Self {
        Self { taille_max: 450, dv_min: 2_000_000.0, prix_min: 5.0, seances_min: 40 }
    }
}

impl Database {

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

    /// Avancement du backfill : (total univers actif, actifs avec bougies).
    /// Ne compte que le périmètre actif — les 'ecarte'/'sans_donnees' sont
    /// hors budget (décisions 05/09) et ne doivent pas polluer le compteur.
    pub async fn avancement_backfill(&self) -> Result<(usize, usize)> {
        let total: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM univers_actions WHERE etat = 'actif'",
        )
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0);
        let avec: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM univers_actions u
             WHERE u.etat = 'actif'
               AND EXISTS (SELECT 1 FROM bougies_actions b WHERE b.ticker = u.ticker)",
        )
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0);
        Ok((total as usize, avec as usize))
    }

    /// Prochains tickers SANS bougies à backfiller : les prioritaires d'abord
    /// (liste de liquidité fournie), puis les pionniers NARRATIFS (table
    /// `narratifs` — le quota mensuel Tiingo va d'abord aux zones à
    /// décollages, décision 05/09), puis le reste par ordre alphabétique.
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
                   {} ORDER BY EXISTS (SELECT 1 FROM narratifs n WHERE n.ticker = u.ticker) DESC, u.ticker LIMIT ?",
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

    /// Tickers DÉJÀ backfillés du périmètre ACTIF, les moins récemment
    /// rafraîchis d'abord (MAX(ts) le plus ancien en tête) — tournante de
    /// rafraîchissement. Les 'ecarte' ne consomment plus de quota.
    pub async fn tickers_a_rafraichir(&self, limite: usize) -> Result<Vec<String>> {
        let rows = sqlx::query(
            "SELECT b.ticker FROM bougies_actions b
             JOIN univers_actions u ON u.ticker = b.ticker AND u.etat = 'actif'
             GROUP BY b.ticker
             ORDER BY MAX(b.ts) ASC LIMIT ?",
        )
        .bind(limite as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| TradingError::Database(e.to_string()))?;
        Ok(rows.iter().map(|r| r.get::<String, _>("ticker")).collect())
    }

    // ── Recalcul du périmètre actif (décisions 05/09) ────────────────────────

    /// Recalcule le périmètre 'actif' de l'univers :
    /// 1. parmi les COUVERTS, classement par dollar-volume moyen (63
    ///    dernières séances) ; les `taille_max` meilleurs passant les bornes
    ///    de liquidité deviennent 'actif', les autres couverts 'ecarte' ;
    /// 2. parmi les NON couverts, restent 'actif' les prioritaires et les
    ///    pionniers narratifs (la file de backfill) — le reste passe
    ///    'ecarte' : fin de la couverture alphabétique du marché entier,
    ///    le quota mensuel va d'abord aux zones à décollages ;
    /// 3. les 'sans_donnees' ne sont jamais touchés (delistings — plus
    ///    jamais retentés).
    /// Retour : (actifs, écartés) après recalcul.
    pub async fn recalculer_univers(
        &self,
        prioritaires: &[&str],
        bornes: &BornesUnivers,
    ) -> Result<(usize, usize)> {
        // 1. Métriques de liquidité des couverts (fenêtre 63 séances).
        let rows = sqlx::query(
            "WITH recent AS (
                 SELECT ticker, close, volume,
                        ROW_NUMBER() OVER (PARTITION BY ticker ORDER BY ts DESC) AS rn
                 FROM bougies_actions
             ),
             dv AS (
                 SELECT ticker, AVG(close * volume) AS dv, AVG(close) AS px, COUNT(*) AS n
                 FROM recent WHERE rn <= 63 GROUP BY ticker
             )
             SELECT ticker, dv, px, n FROM dv",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| TradingError::Database(e.to_string()))?;

        struct Candidat {
            ticker: String,
            dv: f64,
            px: f64,
            n: i64,
        }
        let mut eligibles: Vec<Candidat> = rows
            .iter()
            .filter_map(|r| {
                let c = Candidat {
                    ticker: r.get::<String, _>("ticker"),
                    dv: r.get::<f64, _>("dv"),
                    px: r.get::<f64, _>("px"),
                    n: r.get::<i64, _>("n"),
                };
                (c.n >= bornes.seances_min
                    && c.px >= bornes.prix_min
                    && c.dv >= bornes.dv_min)
                .then_some(c)
            })
            .collect();
        eligibles.sort_by(|a, b| b.dv.total_cmp(&a.dv));
        eligibles.truncate(bornes.taille_max);

        // 2. File : non couverts prioritaires ou narratifs (le IN des
        //    prioritaires est échappé — même patron que tickers_sans_bougies).
        let bornes_prio = prioritaires
            .iter()
            .map(|t| format!("'{}'", t.replace('\'', "''")))
            .collect::<Vec<_>>()
            .join(",");
        let sql_file = format!(
            "SELECT u.ticker FROM univers_actions u
             WHERE NOT EXISTS (SELECT 1 FROM bougies_actions b WHERE b.ticker = u.ticker)
               AND (EXISTS (SELECT 1 FROM narratifs n WHERE n.ticker = u.ticker)
                    OR u.ticker IN ({bornes_prio}))"
        );
        let file_rows = sqlx::query(&sql_file)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| TradingError::Database(e.to_string()))?;
        let file: Vec<String> = file_rows.iter().map(|r| r.get::<String, _>("ticker")).collect();

        // 3. Reset massif puis réactivation (jamais les 'sans_donnees').
        sqlx::query("UPDATE univers_actions SET etat = 'ecarte' WHERE etat != 'sans_donnees'")
            .execute(&self.pool)
            .await
            .map_err(|e| TradingError::Database(e.to_string()))?;

        let mut actifs: Vec<String> = eligibles.iter().map(|c| c.ticker.clone()).collect();
        actifs.extend(file);
        self.maj_etat_tickers(&actifs, "actif").await?;

        let (n_actifs, n_ecartes) = sqlx::query("SELECT
                SUM(CASE WHEN etat = 'actif' THEN 1 ELSE 0 END),
                SUM(CASE WHEN etat = 'ecarte' THEN 1 ELSE 0 END)
             FROM univers_actions")
            .fetch_one(&self.pool)
            .await
            .map(|r| (r.get::<i64, _>(0) as usize, r.get::<i64, _>(1) as usize))
            .map_err(|e| TradingError::Database(e.to_string()))?;
        Ok((n_actifs, n_ecartes))
    }

    /// Change l'état d'un lot de tickers (par tranches — borné aux limites
    /// de paramètres SQL quel que soit le SQLite).
    pub async fn maj_etat_tickers(&self, tickers: &[String], etat: &str) -> Result<()> {
        for tranche in tickers.chunks(100) {
            let bornes = tranche
                .iter()
                .map(|t| format!("'{}'", t.replace('\'', "''")))
                .collect::<Vec<_>>()
                .join(",");
            let sql = format!("UPDATE univers_actions SET etat = ? WHERE ticker IN ({bornes})");
            sqlx::query(&sql)
                .bind(etat)
                .execute(&self.pool)
                .await
                .map_err(|e| TradingError::Database(e.to_string()))?;
        }
        Ok(())
    }
    // ── Scanner actions (étape C) ───────────────────────────────────────────

    /// Tickers de l'univers actif disposant d'assez de bougies pour le
    /// pré-screen (≥ 261 séances = MM200+1 mois + fenêtre 52 semaines).
    pub async fn tickers_evaluables(&self) -> Result<Vec<String>> {
        let rows = sqlx::query(
            "SELECT b.ticker FROM bougies_actions b
             JOIN univers_actions u ON u.ticker = b.ticker AND u.etat = 'actif'
             GROUP BY b.ticker HAVING COUNT(*) >= 261
             ORDER BY b.ticker",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| TradingError::Database(e.to_string()))?;
        Ok(rows.iter().map(|r| r.get::<String, _>("ticker")).collect())
    }

    // (Ex-entonnoir prescreen_actions supprimé le 05/09 — §7-3a : journal
    // write-only sans lecteur depuis la suppression de GET prescreen ;
    // table DROP par la migration 0101.)
}
