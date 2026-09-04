//! Snapshots quotidiens des rapports d'activité (§14 roadmap) — suivi de
//! l'évolution des métriques ET des avis IA jour après jour.

use crate::{Database, TradingError};
use chrono::{Datelike, TimeZone};
use serde::Serialize;
use sqlx::Row;

#[derive(Debug, Serialize)]
pub struct SnapshotAnalyse {
    pub strategie: String,
    pub jour: String,
    pub capital_depart: f64,
    pub capital_actuel: f64,
    pub r_total: f64,
    /// 0-1.
    pub taux_reussite: f64,
    pub nb_trades: i64,
    pub hier_dollars: Option<f64>,
    pub calcule_le: i64,
    pub avis_ia: Option<String>,
    pub avis_le: Option<i64>,
}

/// Clé du jour en heure locale (YYYY-MM-DD) — mêmes frontières que les
/// agrégats du rapport (jour calendaire local).
pub fn cle_du_jour(ts: i64) -> String {
    let d = chrono::Local
        .timestamp_opt(ts, 0)
        .single()
        .unwrap_or_else(chrono::Local::now);
    format!("{:04}-{:02}-{:02}", d.year(), d.month(), d.day())
}

impl Database {
    /// Écrit/met à jour le snapshot du jour (INSERT OR REPLACE — le snapshot
    /// du jour reflète toujours le dernier calcul, avis préservé).
    pub async fn enregistrer_analyse_snapshot(
        &self,
        strategie: &str,
        jour: &str,
        capital_depart: f64,
        capital_actuel: f64,
        r_total: f64,
        taux_reussite: f64,
        nb_trades: i64,
        hier_dollars: Option<f64>,
        calcule_le: i64,
    ) -> Result<(), TradingError> {
        sqlx::query(
            "INSERT INTO analyses_snapshots
                (strategie, jour, capital_depart, capital_actuel, r_total,
                 taux_reussite, nb_trades, hier_dollars, calcule_le)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(strategie, jour) DO UPDATE SET
                capital_depart = excluded.capital_depart,
                capital_actuel = excluded.capital_actuel,
                r_total = excluded.r_total,
                taux_reussite = excluded.taux_reussite,
                nb_trades = excluded.nb_trades,
                hier_dollars = excluded.hier_dollars,
                calcule_le = excluded.calcule_le",
        )
        .bind(strategie)
        .bind(jour)
        .bind(capital_depart)
        .bind(capital_actuel)
        .bind(r_total)
        .bind(taux_reussite)
        .bind(nb_trades)
        .bind(hier_dollars)
        .bind(calcule_le)
        .execute(&self.pool)
        .await
        .map_err(|e| TradingError::Database(e.to_string()))?;
        Ok(())
    }

    /// Rattache l'avis IA du jour à son snapshot (crée la ligne si le
    /// snapshot n'existe pas encore — l'avis peut précéder le calcul).
    pub async fn enregistrer_avis_snapshot(
        &self,
        strategie: &str,
        jour: &str,
        avis_json: &str,
        avis_le: i64,
    ) -> Result<(), TradingError> {
        sqlx::query(
            "INSERT INTO analyses_snapshots (strategie, jour, avis_ia, avis_le, calcule_le)
             VALUES (?, ?, ?, ?, ?)
             ON CONFLICT(strategie, jour) DO UPDATE SET
                avis_ia = excluded.avis_ia,
                avis_le = excluded.avis_le",
        )
        .bind(strategie)
        .bind(jour)
        .bind(avis_json)
        .bind(avis_le)
        .bind(avis_le)
        .execute(&self.pool)
        .await
        .map_err(|e| TradingError::Database(e.to_string()))?;
        Ok(())
    }

    /// Historique d'une stratégie, du plus récent au plus ancien.
    pub async fn lister_analyses_snapshots(
        &self,
        strategie: &str,
        limite: i64,
    ) -> Result<Vec<SnapshotAnalyse>, TradingError> {
        let rows = sqlx::query(
            "SELECT strategie, jour, capital_depart, capital_actuel, r_total,
                    taux_reussite, nb_trades, hier_dollars, calcule_le, avis_ia, avis_le
             FROM analyses_snapshots WHERE strategie = ?
             ORDER BY jour DESC LIMIT ?",
        )
        .bind(strategie)
        .bind(limite)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| TradingError::Database(e.to_string()))?;
        Ok(rows
            .iter()
            .map(|r| SnapshotAnalyse {
                strategie: r.get("strategie"),
                jour: r.get("jour"),
                capital_depart: r.get("capital_depart"),
                capital_actuel: r.get("capital_actuel"),
                r_total: r.get("r_total"),
                taux_reussite: r.get("taux_reussite"),
                nb_trades: r.get("nb_trades"),
                hier_dollars: r.try_get("hier_dollars").ok(),
                calcule_le: r.get("calcule_le"),
                avis_ia: r.try_get("avis_ia").ok(),
                avis_le: r.try_get("avis_le").ok(),
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::cle_du_jour;

    #[test]
    fn cle_du_jour_format_iso() {
        // 2026-09-04 00:30 UTC = 2026-09-03 en heure US (Local honoré)
        let cle = cle_du_jour(1_788_515_200);
        assert!(cle.starts_with("2026-09-0"), "préfixe date locale : {cle}");
        assert_eq!(cle.len(), 10);
    }
}
