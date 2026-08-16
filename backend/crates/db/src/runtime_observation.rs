//! Journal d'observation du runtime tick (Phase 1.5 ROADMAP).
//!
//! Chaque bougie clôturée par le runtime est journalisée ici, avec son
//! mode de clôture. La concordance avec les bougies officielles (`bougies`,
//! écrites par le worker Bybit) se mesure via [`Database::lire_concordance`]
//! — c'est l'outil de la Gate 1 (100 % de concordance sur 24 h).

use chrono::{DateTime, Utc};
use common::{Asset, Candle, Timeframe};
use serde::Serialize;
use sqlx::Row;

use crate::Database;

/// Ligne de concordance par couple (asset × timeframe).
#[derive(Debug, Serialize)]
pub struct ConcordanceCouple {
    pub asset: String,
    pub timeframe: String,
    /// Bougies présentes des deux côtés (runtime + officielles).
    pub communes: i64,
    /// Strictement identiques (OHLCV bit à bit).
    pub concordantes: i64,
    /// Divergentes sur au moins un champ.
    pub divergentes: i64,
    /// Pourcentage de concordance (0–100), arrondi.
    pub pct: i64,
}

/// Divergence détaillée (échantillon diagnostique).
#[derive(Debug, Serialize)]
pub struct DivergenceDetail {
    pub asset: String,
    pub timeframe: String,
    pub timestamp: i64,
    pub mode_cloture: String,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub off_open: f64,
    pub off_high: f64,
    pub off_low: f64,
    pub off_close: f64,
    pub off_volume: f64,
}

/// Rapport de concordance complet — payload de la Gate 1.
#[derive(Debug, Serialize)]
pub struct RapportConcordance {
    pub depuis_ts: i64,
    pub jusqu_a_ts: i64,
    pub par_couple: Vec<ConcordanceCouple>,
    /// Bougies officielles sans contrepartie runtime (trous de couverture).
    pub officielles_sans_runtime: i64,
    /// Bougies runtime sans contrepartie officielle (uniquement des clôtures
    /// `forcee`/`passage` en théorie).
    pub runtime_sans_officielle: i64,
    /// Échantillon de divergences (max 20, pour diagnostic).
    pub divergences: Vec<DivergenceDetail>,
    /// `true` si toutes les bougies communes concordent.
    pub conforme: bool,
}

impl Database {
    /// Journalise une bougie clôturée par le runtime.
    /// `INSERT OR REPLACE` : une re-clôture post-reconnexion remplace
    /// l'entrée précédente (la dernière fait foi).
    pub async fn inserer_observation_runtime(
        &self,
        asset: &Asset,
        timeframe: &Timeframe,
        bougie: &Candle,
        mode_cloture: &str,
        cloture_le: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        sqlx::query(
            "INSERT OR REPLACE INTO runtime_observation
                (asset, timeframe, timestamp, open, high, low, close, volume,
                 mode_cloture, cloture_le_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        )
        .bind(asset.as_str())
        .bind(timeframe.as_str())
        .bind(bougie.timestamp.timestamp())
        .bind(bougie.open)
        .bind(bougie.high)
        .bind(bougie.low)
        .bind(bougie.close)
        .bind(bougie.volume)
        .bind(mode_cloture)
        .bind(cloture_le.timestamp_millis())
        .execute(self.pool())
        .await?;
        Ok(())
    }

    /// Compare les bougies du runtime aux bougies officielles depuis
    /// `depuis_ts` (epoch sec). L'égalité est stricte (bit à bit) : les
    /// deux chemins parsent les mêmes valeurs poussées par l'échange.
    pub async fn lire_concordance(&self, depuis_ts: i64) -> anyhow::Result<RapportConcordance> {
        // Concordance par couple.
        let lignes = sqlx::query(
            "SELECT o.asset AS asset, o.timeframe AS tf,
                    COUNT(*) AS communes,
                    SUM(CASE WHEN o.open = b.open AND o.high = b.high
                          AND o.low = b.low AND o.close = b.close
                          AND o.volume = b.volume
                        THEN 1 ELSE 0 END) AS concordantes
             FROM runtime_observation o
             JOIN bougies b
               ON b.asset = o.asset AND b.timeframe = o.timeframe
              AND b.timestamp = o.timestamp
             WHERE o.timestamp >= ?1
             GROUP BY o.asset, o.timeframe
             ORDER BY o.asset, o.timeframe",
        )
        .bind(depuis_ts)
        .fetch_all(self.pool())
        .await?;

        let par_couple: Vec<ConcordanceCouple> = lignes
            .iter()
            .map(|r| {
                let communes: i64 = r.try_get("communes").unwrap_or(0);
                let concordantes: i64 = r.try_get("concordantes").unwrap_or(0);
                ConcordanceCouple {
                    asset: r.try_get("asset").unwrap_or_default(),
                    timeframe: r.try_get("tf").unwrap_or_default(),
                    communes,
                    concordantes,
                    divergentes: communes - concordantes,
                    pct: if communes > 0 {
                        concordantes * 100 / communes
                    } else {
                        0
                    },
                }
            })
            .collect();

        // Bougies officielles sans contrepartie runtime (sur les couples
        // effectivement journalisés par le runtime).
        let officielles_sans_runtime: i64 = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM bougies b
             WHERE b.timestamp >= ?1
               AND EXISTS (SELECT 1 FROM runtime_observation o1
                            WHERE o1.asset = b.asset AND o1.timeframe = b.timeframe)
               AND NOT EXISTS (SELECT 1 FROM runtime_observation o2
                                WHERE o2.asset = b.asset AND o2.timeframe = b.timeframe
                                  AND o2.timestamp = b.timestamp)",
        )
        .bind(depuis_ts)
        .fetch_one(self.pool())
        .await?;

        let runtime_sans_officielle: i64 = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM runtime_observation o
             WHERE o.timestamp >= ?1
               AND NOT EXISTS (SELECT 1 FROM bougies b
                                WHERE b.asset = o.asset AND b.timeframe = o.timeframe
                                  AND b.timestamp = o.timestamp)",
        )
        .bind(depuis_ts)
        .fetch_one(self.pool())
        .await?;

        // Échantillon de divergences.
        let divergences_lignes = sqlx::query(
            "SELECT o.asset AS asset, o.timeframe AS tf, o.timestamp AS ts,
                    o.mode_cloture AS mode,
                    o.open AS o_open, o.high AS o_high, o.low AS o_low,
                    o.close AS o_close, o.volume AS o_volume,
                    b.open AS b_open, b.high AS b_high, b.low AS b_low,
                    b.close AS b_close, b.volume AS b_volume
             FROM runtime_observation o
             JOIN bougies b
               ON b.asset = o.asset AND b.timeframe = o.timeframe
              AND b.timestamp = o.timestamp
             WHERE o.timestamp >= ?1
               AND (o.open != b.open OR o.high != b.high OR o.low != b.low
                    OR o.close != b.close OR o.volume != b.volume)
             ORDER BY o.timestamp DESC
             LIMIT 20",
        )
        .bind(depuis_ts)
        .fetch_all(self.pool())
        .await?;

        let divergences: Vec<DivergenceDetail> = divergences_lignes
            .iter()
            .map(|r| DivergenceDetail {
                asset: r.try_get("asset").unwrap_or_default(),
                timeframe: r.try_get("tf").unwrap_or_default(),
                timestamp: r.try_get("ts").unwrap_or(0),
                mode_cloture: r.try_get("mode").unwrap_or_default(),
                open: r.try_get("o_open").unwrap_or(0.0),
                high: r.try_get("o_high").unwrap_or(0.0),
                low: r.try_get("o_low").unwrap_or(0.0),
                close: r.try_get("o_close").unwrap_or(0.0),
                volume: r.try_get("o_volume").unwrap_or(0.0),
                off_open: r.try_get("b_open").unwrap_or(0.0),
                off_high: r.try_get("b_high").unwrap_or(0.0),
                off_low: r.try_get("b_low").unwrap_or(0.0),
                off_close: r.try_get("b_close").unwrap_or(0.0),
                off_volume: r.try_get("b_volume").unwrap_or(0.0),
            })
            .collect();

        let conforme = par_couple.iter().all(|c| c.divergentes == 0);

        Ok(RapportConcordance {
            depuis_ts,
            jusqu_a_ts: Utc::now().timestamp(),
            par_couple,
            officielles_sans_runtime,
            runtime_sans_officielle,
            divergences,
            conforme,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    async fn db_test() -> Database {
        let db = Database::new(":memory:").await.expect("DB mémoire");
        db.run_migrations().await.expect("migrations OK");
        db
    }

    fn bougie_ts(ts: i64, open: f64, high: f64, low: f64, close: f64, volume: f64) -> Candle {
        Candle {
            timestamp: Utc.timestamp_opt(ts, 0).unwrap(),
            open,
            high,
            low,
            close,
            volume,
        }
    }

    #[tokio::test]
    async fn concordance_parfaite_sur_bougies_identiques() {
        let db = db_test().await;
        let bougies = vec![
            bougie_ts(1000, 10.0, 11.0, 9.0, 10.5, 100.0),
            bougie_ts(1300, 10.5, 12.0, 10.0, 11.5, 200.0),
        ];
        // Officielles en DB…
        db.inserer_bougies(&Asset::from("BTC"), &Timeframe::M5, &bougies)
            .await
            .unwrap();
        // …et journal runtime identiques.
        for b in &bougies {
            db.inserer_observation_runtime(&Asset::from("BTC"), &Timeframe::M5, b, "confirmation", Utc::now())
                .await
                .unwrap();
        }

        let rapport = db.lire_concordance(0).await.unwrap();
        assert!(rapport.conforme, "aucune divergence attendue");
        assert_eq!(rapport.par_couple.len(), 1);
        let c = &rapport.par_couple[0];
        assert_eq!(c.communes, 2);
        assert_eq!(c.concordantes, 2);
        assert_eq!(c.pct, 100);
        assert_eq!(rapport.runtime_sans_officielle, 0);
        assert_eq!(rapport.officielles_sans_runtime, 0);
    }

    #[tokio::test]
    async fn divergence_detectee_et_detaillee() {
        let db = db_test().await;
        db.inserer_bougies(
            &Asset::from("BTC"),
            &Timeframe::M5,
            &[bougie_ts(1000, 10.0, 11.0, 9.0, 10.5, 100.0)],
        )
        .await
        .unwrap();
        // Le runtime a clôturé par passage de période avec un close différent
        // (la confirmation officielle n'était pas encore arrivée).
        db.inserer_observation_runtime(
            &Asset::from("BTC"),
            &Timeframe::M5,
            &bougie_ts(1000, 10.0, 11.0, 9.0, 10.2, 100.0),
            "passage",
            Utc::now(),
        )
        .await
        .unwrap();

        let rapport = db.lire_concordance(0).await.unwrap();
        assert!(!rapport.conforme, "divergence attendue");
        assert_eq!(rapport.par_couple[0].divergentes, 1);
        assert_eq!(rapport.par_couple[0].pct, 0);
        assert_eq!(rapport.divergences.len(), 1);
        let d = &rapport.divergences[0];
        assert_eq!(d.mode_cloture, "passage");
        assert!((d.close - 10.2).abs() < 1e-9);
        assert!((d.off_close - 10.5).abs() < 1e-9);
    }

    #[tokio::test]
    async fn trous_de_couverture_comptes() {
        let db = db_test().await;
        // Officielles @1000 et @1300, runtime n'a journalisé que @1000.
        db.inserer_bougies(
            &Asset::from("BTC"),
            &Timeframe::M5,
            &[
                bougie_ts(1000, 10.0, 11.0, 9.0, 10.5, 100.0),
                bougie_ts(1300, 10.5, 12.0, 10.0, 11.5, 200.0),
            ],
        )
        .await
        .unwrap();
        db.inserer_observation_runtime(
            &Asset::from("BTC"),
            &Timeframe::M5,
            &bougie_ts(1000, 10.0, 11.0, 9.0, 10.5, 100.0),
            "confirmation",
            Utc::now(),
        )
        .await
        .unwrap();

        let rapport = db.lire_concordance(0).await.unwrap();
        assert_eq!(rapport.officielles_sans_runtime, 1, "la bougie @1300 manque au runtime");
        assert!(rapport.conforme, "les bougies communes concordent");
    }

    #[tokio::test]
    async fn clôture_remplace_l_entree_precedente() {
        let db = db_test().await;
        db.inserer_bougies(
            &Asset::from("BTC"),
            &Timeframe::M5,
            &[bougie_ts(1000, 10.0, 11.0, 9.0, 10.5, 100.0)],
        )
        .await
        .unwrap();
        // Clôture 'passage' approximative puis re-clôture 'confirmation'
        // officielle post-reconnexion : la dernière fait foi.
        db.inserer_observation_runtime(
            &Asset::from("BTC"),
            &Timeframe::M5,
            &bougie_ts(1000, 10.0, 11.0, 9.0, 10.2, 95.0),
            "passage",
            Utc::now(),
        )
        .await
        .unwrap();
        db.inserer_observation_runtime(
            &Asset::from("BTC"),
            &Timeframe::M5,
            &bougie_ts(1000, 10.0, 11.0, 9.0, 10.5, 100.0),
            "confirmation",
            Utc::now(),
        )
        .await
        .unwrap();

        let rapport = db.lire_concordance(0).await.unwrap();
        assert!(rapport.conforme, "la re-clôture officielle remplace l'approximation");
        assert_eq!(rapport.par_couple[0].communes, 1, "une seule ligne (REPLACE)");
    }
}
