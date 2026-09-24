//! Clôtures pour la simulation de capital (api::capital_simule) + le funnel
//! de clôture officiel (fermer_signal_par_cle, extrait de signaux.rs —
//! limite 600 lignes par fichier, pre-commit).

use common::Result;
use crate::{Database, TradingError};
use sqlx::Row;

/// Clôture remplie pour la simulation de capital (ordre chronologique).
#[derive(Debug)]
pub struct ClotureCapital {
    pub id: String,
    pub ferme_le: i64,
    /// R réalisé (sortie réelle) — 0 si absent.
    pub r: f64,
    /// Asset / timeframe / verdict — pour les camemberts $ du dashboard et
    /// les agrégats du centre d'analyse (contribution par catégorie).
    pub asset: String,
    pub tf: String,
    pub verdict: String,
    /// Niveaux du signal (conversion R-distance, refonte 15/09).
    pub prix_entree: f64,
    pub stop_loss: f64,
    pub take_profit: String,
    /// Prix de sortie réel — R du SOLDE des TP2+BE (correctif 15/09 nuit :
    /// le stop suiveur post-TP2 est à TP1, le solde s'y encaisse, pas à 0).
    pub prix_verdict: Option<f64>,
    /// Clé moteur du signal (straddle : straddle-{asset}-{annonce_ts}-B) —
    /// catégorisation « par événement » de l'analyse vécue (17/09).
    pub cle_moteur: Option<String>,
    /// Fractions SMC figées À LA CLÔTURE (6.7, 24/09) — JSON
    /// {"tp1":..,"tp2":..,"tp3":..}. NULL = clôture historique → repli
    /// défaut 0,5/0,3/0,2 à la lecture ; autres stratégies : NULL.
    pub fractions_json: Option<String>,
}

impl Database {
    /// Phase 2.8 — ferme le signal officiel correspondant à une clé moteur,
    /// avec son verdict (TP1/TP2/TP3/SL/BE/Expire), son prix de sortie et
    /// son R réel. 6.7 (24/09) : une clôture SMC FIGE les fractions en
    /// vigueur (fractions_json) — la courbe $ du vécu compose les fractions
    /// du trade, jamais celles du jour de la lecture. Les autres stratégies
    /// n'ont pas de ventes partielles (NULL).
    pub async fn fermer_signal_par_cle(
        &self,
        cle_moteur: &str,
        asset: &str,
        verdict: &str,
        prix_verdict: f64,
        r_realise: f64,
        ferme_le: i64,
    ) -> Result<u64> {
        // Filtre ASSET obligatoire : des stratégies (straddle notamment)
        // partagent la même clé entre assets pour une même annonce — sans
        // ce filtre, la première clôture fermait toutes les lignes (bug
        // 27/08 : +31R de PCE écrasés par la clôture XAU).
        let strategie: Option<String> = sqlx::query(
            "SELECT strategie FROM signaux
             WHERE cle_moteur = ? AND asset = ? AND statut = 'Actif' LIMIT 1",
        )
        .bind(cle_moteur)
        .bind(asset)
        .fetch_optional(&self.pool)
        .await
        .ok()
        .flatten()
        .and_then(|r| r.try_get::<String, _>("strategie").ok());

        let fractions_json = if strategie.as_deref() == Some("SMC") {
            Some(self.fractions_smc_json().await)
        } else {
            None
        };

        let res = sqlx::query(
            "UPDATE signaux SET statut = 'Fermé', verdict = ?, prix_verdict = ?, r_realise = ?, ferme_le = ?,
                fractions_json = ?
             WHERE cle_moteur = ? AND asset = ? AND statut = 'Actif'",
        )
        .bind(verdict)
        .bind(prix_verdict)
        .bind(r_realise)
        .bind(ferme_le)
        .bind(&fractions_json)
        .bind(cle_moteur)
        .bind(asset)
        .execute(&self.pool)
        .await
        .map_err(|e| TradingError::Database(e.to_string()))?;

        if res.rows_affected() > 0 {
            crate::ml_samples::collecter_a_la_cloture(&self.pool, cle_moteur, asset, verdict, prix_verdict, r_realise).await;
        }
        Ok(res.rows_affected())
    }

    /// Fractions SMC lues de la config (smc_frac_tp*, défauts 0,5/0,3/0,2),
    /// sérialisées pour le gel à la clôture. Mêmes clés/défauts que
    /// api::reglages_smc::lire_fractions — la db ne dépend pas de l'api.
    async fn fractions_smc_json(&self) -> String {
        async fn fraction_config(db: &Database, cle: &str, defaut: f64) -> f64 {
            db.lire_config(cle)
                .await
                .ok()
                .flatten()
                .and_then(|v| v.trim().parse::<f64>().ok())
                .unwrap_or(defaut)
        }
        serde_json::json!({
            "tp1": fraction_config(self, "smc_frac_tp1", 0.5).await,
            "tp2": fraction_config(self, "smc_frac_tp2", 0.3).await,
            "tp3": fraction_config(self, "smc_frac_tp3", 0.2).await,
        })
        .to_string()
    }

    /// Epoch (sec) de la première émission d'une stratégie — borne la fenêtre
    /// de re-jeu paramétrique à la période réellement vécue par l'app.
    pub async fn debut_historique_epoch(&self, strategie: &str) -> Option<i64> {
        sqlx::query_scalar::<_, i64>(
            "SELECT MIN(cree_le) FROM signaux WHERE strategie = ? AND cree_le IS NOT NULL",
        )
        .bind(strategie)
        .fetch_optional(&self.pool)
        .await
        .ok()
        .flatten()
    }

    /// Clôtures REMPLIES d'une stratégie — matière première de la simulation
    /// de capital composée. Un ordre jamais touché n'engage pas de capital
    /// (heure_entree IS NOT NULL), les expirés/BE participent avec leur R réel.
    pub async fn clotures_pour_capital(&self, id: &str) -> crate::Result<Vec<ClotureCapital>> {
        let rows = sqlx::query(
            "SELECT id, ferme_le, r_realise, asset, timeframe,
                    COALESCE(verdict, '') AS verdict,
                    prix_entree, stop_loss, take_profit, prix_verdict, cle_moteur, fractions_json
             FROM signaux
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
                asset: r.get("asset"),
                tf: r.get("timeframe"),
                prix_entree: r.try_get::<f64, _>("prix_entree").ok().unwrap_or(0.0),
                stop_loss: r.try_get::<f64, _>("stop_loss").ok().unwrap_or(0.0),
                take_profit: r.try_get::<String, _>("take_profit").ok().unwrap_or_default(),
                verdict: r.get("verdict"),
                prix_verdict: r.try_get::<f64, _>("prix_verdict").ok(),
                cle_moteur: r.try_get::<Option<String>, _>("cle_moteur").ok().flatten(),
                fractions_json: r.try_get::<Option<String>, _>("fractions_json").ok().flatten(),
            })
            .collect())
    }
}

impl Database {
    /// Sources des passes straddle clôturées pour le re-jeu paramétrique :
    /// (asset, annonce_ts parsé de la clé, entrée, R). La clé moteur est
    /// « straddle-{asset}-{annonce_ts}-B » (ou « straddle-{ts}-B » legacy BTC).
    pub async fn clotures_pour_capital_straddle(
        &self,
    ) -> crate::Result<Vec<(String, i64, f64, f64)>> {
        let rows = sqlx::query(
            "SELECT asset, cle_moteur, prix_entree, stop_loss, heure_entree
             FROM signaux
             WHERE strategie = 'straddle' AND statut = 'Fermé'
               AND heure_entree IS NOT NULL AND cle_moteur IS NOT NULL",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| TradingError::Database(e.to_string()))?;
        Ok(rows
            .iter()
            .filter_map(|r| {
                let asset: String = r.try_get("asset").ok()?;
                let cle: String = r.try_get("cle_moteur").ok()?;
                let entree: f64 = r.try_get("prix_entree").ok()?;
                let sl: f64 = r.try_get("stop_loss").ok()?;
                let risque = (entree - sl).abs();
                let heure: i64 = r.try_get("heure_entree").ok()?;
                let parts: Vec<&str> = cle.split('-').collect();
                let annonce_ts = if parts.len() >= 4 {
                    parts[2].parse::<i64>().unwrap_or(heure)
                } else {
                    heure
                };
                if risque > 0.0 && annonce_ts > 0 {
                    Some((asset, annonce_ts, entree, risque))
                } else {
                    None
                }
            })
            .collect())
    }
}
