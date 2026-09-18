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
}

impl Database {
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
                    prix_entree, stop_loss, take_profit, prix_verdict, cle_moteur
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
