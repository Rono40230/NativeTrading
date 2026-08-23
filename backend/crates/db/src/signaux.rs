use chrono::Utc;
use common::{Asset, Result, Signal, Timeframe, TradingError};
use serde::Serialize;
use sqlx::{Row, SqlitePool};

use crate::Database;

/// Signal actif retourné par le worker de suivi
#[derive(Debug, Serialize)]
pub struct SignalActif {
    pub id: String,
    pub asset: String,
    pub direction: String,
    pub prix_entree: f64,
    pub stop_loss: f64,
    pub take_profit: Vec<f64>,
    pub cree_le: i64,
}

// ── Fonctions libres sur SqlitePool (utilisées par le worker) ────────────────

pub async fn lister_actifs(pool: &SqlitePool) -> Result<Vec<SignalActif>> {
    let rows = sqlx::query(
        "SELECT id, asset, direction, prix_entree, stop_loss, take_profit, cree_le
         FROM signaux WHERE statut = 'Actif' ORDER BY cree_le DESC",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;

    Ok(rows
        .iter()
        .map(|row| {
            let tp_raw = row.get::<String, _>("take_profit");
            let take_profit: Vec<f64> = serde_json::from_str(&tp_raw).unwrap_or_default();
            SignalActif {
                id: row.get("id"),
                asset: row.get("asset"),
                direction: row.get("direction"),
                prix_entree: row.get("prix_entree"),
                stop_loss: row.get("stop_loss"),
                take_profit,
                cree_le: row.get("cree_le"),
            }
        })
        .collect())
}

pub async fn maj_verdict(pool: &SqlitePool, id: &str, verdict: &str, prix: f64) -> Result<()> {
    let now = Utc::now().timestamp();
    sqlx::query(
        "UPDATE signaux SET statut='Fermé', verdict=?, prix_verdict=?, ferme_le=? WHERE id=?",
    )
    .bind(verdict)
    .bind(prix)
    .bind(now)
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;
    Ok(())
}

/// Met à jour l'état de suivi progressif d'un signal SMC (SL courant + TPs atteints).
/// Appelée par le job de réconciliation SMC à chaque transition (TP1 → BE, TP2 → TP1).
/// N'altère pas le statut du signal (reste 'Actif').
pub async fn maj_suivi_progressif_smc(
    pool: &SqlitePool,
    id: &str,
    sl_effectif: f64,
    tps_atteints: &[&str],
) -> Result<()> {
    let tps_json =
        serde_json::to_string(tps_atteints).map_err(|e| TradingError::Database(e.to_string()))?;
    sqlx::query("UPDATE signaux SET sl_effectif = ?, tps_atteints = ? WHERE id = ?")
        .bind(sl_effectif)
        .bind(tps_json)
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| TradingError::Database(e.to_string()))?;
    Ok(())
}

/// Met à jour l'état de suivi progressif d'un signal Straddle, par jambe.
/// Appelée par le job de réconciliation Straddle à chaque transition.
pub async fn maj_suivi_progressif_straddle(
    pool: &SqlitePool,
    id: &str,
    sl_long: f64,
    sl_short: f64,
    tps_long: &[&str],
    tps_short: &[&str],
) -> Result<()> {
    let long_json =
        serde_json::to_string(tps_long).map_err(|e| TradingError::Database(e.to_string()))?;
    let short_json =
        serde_json::to_string(tps_short).map_err(|e| TradingError::Database(e.to_string()))?;
    sqlx::query(
        "UPDATE signaux
         SET sl_long_effectif = ?, sl_short_effectif = ?,
             tps_long_atteints = ?, tps_short_atteints = ?
         WHERE id = ?",
    )
    .bind(sl_long)
    .bind(sl_short)
    .bind(long_json)
    .bind(short_json)
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;
    Ok(())
}

pub async fn expirer_anciens(pool: &SqlitePool) -> Result<i64> {
    let seuil = Utc::now().timestamp() - 24 * 3600;
    let res = sqlx::query(
        "UPDATE signaux SET statut='Fermé', verdict='expire', ferme_le=?
         WHERE statut='Actif' AND cree_le < ?",
    )
    .bind(Utc::now().timestamp())
    .bind(seuil)
    .execute(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;
    Ok(res.rows_affected() as i64)
}

impl Database {
    /// Enregistre un signal en base
    pub async fn inserer_signal(&self, signal: &Signal) -> Result<()> {
        let tp_json = serde_json::to_string(&signal.take_profit)
            .map_err(|e| TradingError::Database(e.to_string()))?;

        sqlx::query(
            "INSERT OR IGNORE INTO signaux
             (id, asset, timeframe, direction, score, prix_entree,
              stop_loss, take_profit, strategie, cree_le)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(signal.id.to_string())
        .bind(signal.asset.as_str())
        .bind(signal.timeframe.as_str())
        .bind(format!("{:?}", signal.direction))
        .bind(signal.score)
        .bind(signal.prix_entree)
        .bind(signal.stop_loss)
        .bind(tp_json)
        .bind(&signal.strategie)
        .bind(signal.cree_le.timestamp())
        .execute(&self.pool)
        .await
        .map_err(|e| TradingError::Database(e.to_string()))?;

        Ok(())
    }

    /// Phase 2.8 — insère un signal OFFICIEL du runtime v12 (avec `cle_moteur`
    /// pour fermer la ligne à l'événement de clôture correspondant).
    pub async fn inserer_signal_officiel(
        &self,
        signal: &Signal,
        cle_moteur: &str,
    ) -> Result<()> {
        let tp_json = serde_json::to_string(&signal.take_profit)
            .map_err(|e| TradingError::Database(e.to_string()))?;
        sqlx::query(
            "INSERT OR IGNORE INTO signaux
             (id, asset, timeframe, direction, score, prix_entree,
              stop_loss, take_profit, strategie, statut, cree_le, cle_moteur)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 'Actif', ?, ?)",
        )
        .bind(signal.id.to_string())
        .bind(signal.asset.as_str())
        .bind(signal.timeframe.as_str())
        .bind(format!("{:?}", signal.direction))
        .bind(signal.score)
        .bind(signal.prix_entree)
        .bind(signal.stop_loss)
        .bind(tp_json)
        .bind(&signal.strategie)
        .bind(signal.cree_le.timestamp())
        .bind(cle_moteur)
        .execute(&self.pool)
        .await
        .map_err(|e| TradingError::Database(e.to_string()))?;
        Ok(())
    }

    /// Étape 2 — marque le signal comme notifié sur Telegram (le writer
    /// officiel envoie directement ; ce drapeau trace l'envoi en base).
    pub async fn marquer_telegram_envoye(&self, id: &str) -> Result<()> {
        sqlx::query("UPDATE signaux SET telegram_envoye = 1 WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| TradingError::Database(e.to_string()))?;
        Ok(())
    }

    /// Phase 2.8 — ferme le signal officiel correspondant à une clé moteur.
    pub async fn fermer_signal_par_cle(&self, cle_moteur: &str, ferme_le: i64) -> Result<u64> {
        let res = sqlx::query(
            "UPDATE signaux SET statut = 'Fermé', ferme_le = ? WHERE cle_moteur = ? AND statut = 'Actif'",
        )
        .bind(ferme_le)
        .bind(cle_moteur)
        .execute(&self.pool)
        .await
        .map_err(|e| TradingError::Database(e.to_string()))?;
        Ok(res.rows_affected())
    }

    /// Enregistre un signal Straddle (Direction::Both) avec les niveaux des deux jambes.
    /// - `stop_loss`         = SL jambe LONG  (< prix_entree)
    /// - `take_profit`       = [tp1, tp2, tp3] jambe LONG  (> prix_entree)
    /// - `sl_short`          = SL jambe SHORT (> prix_entree)
    /// - `take_profit_short` = [tp1, tp2, tp3] jambe SHORT (< prix_entree)
    /// - `heure_entree`      = timestamp Unix UTC cible (None = entrée immédiate)
    pub async fn inserer_signal_straddle_complet(
        &self,
        signal: &Signal,
        sl_short: f64,
        take_profit_short: &[f64],
        heure_entree: Option<i64>,
    ) -> Result<()> {
        let tp_long_json = serde_json::to_string(&signal.take_profit)
            .map_err(|e| TradingError::Database(e.to_string()))?;
        let tp_short_json = serde_json::to_string(take_profit_short)
            .map_err(|e| TradingError::Database(e.to_string()))?;

        sqlx::query(
            "INSERT OR IGNORE INTO signaux
             (id, asset, timeframe, direction, score, prix_entree,
              stop_loss, take_profit, strategie, cree_le,
              sl_short, take_profit_short, heure_entree)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(signal.id.to_string())
        .bind(signal.asset.as_str())
        .bind(signal.timeframe.as_str())
        .bind(format!("{:?}", signal.direction))
        .bind(signal.score)
        .bind(signal.prix_entree)
        .bind(signal.stop_loss)
        .bind(tp_long_json)
        .bind(&signal.strategie)
        .bind(signal.cree_le.timestamp())
        .bind(sl_short)
        .bind(tp_short_json)
        .bind(heure_entree)
        .execute(&self.pool)
        .await
        .map_err(|e| TradingError::Database(e.to_string()))?;

        Ok(())
    }

    /// Liste les signaux encore actifs (pour le worker de suivi).
    pub async fn lister_signaux_actifs(&self) -> Result<Vec<SignalActif>> {
        lister_actifs(&self.pool).await
    }

    /// Expire les signaux actifs depuis plus de 24h sans verdict.
    pub async fn expirer_signaux_anciens(&self) -> Result<i64> {
        expirer_anciens(&self.pool).await
    }

    /// Vérifie si un signal (même asset/timeframe) existe dans la fenêtre anti-doublon.
    pub async fn signal_recent_existe(
        &self,
        asset: &Asset,
        tf: &Timeframe,
        min: i64,
    ) -> Result<bool> {
        let seuil = Utc::now().timestamp() - min * 60;
        let row = sqlx::query(
            "SELECT COUNT(*) as n FROM signaux
             WHERE asset = ? AND timeframe = ? AND cree_le >= ?",
        )
        .bind(asset.as_str())
        .bind(tf.as_str())
        .bind(seuil)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| TradingError::Database(e.to_string()))?;
        Ok(row.get::<i64, _>("n") > 0)
    }

    /// Compte les signaux générés dans les `minutes` dernières minutes.
    pub async fn compter_signaux_recents(&self, minutes: i64) -> Result<i64> {
        let seuil = Utc::now().timestamp() - minutes * 60;
        let row = sqlx::query("SELECT COUNT(*) as n FROM signaux WHERE cree_le >= ?")
            .bind(seuil)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| TradingError::Database(e.to_string()))?;
        Ok(row.get::<i64, _>("n"))
    }

    /// Enregistre un signal avec les métadonnées LLM du filtre pré-sauvegarde.
    pub async fn inserer_signal_avec_llm(
        &self,
        signal: &Signal,
        llm_valide: i64,
        llm_conviction: i64,
        llm_raison: &str,
        llm_sl_suggere: Option<f64>,
        llm_tp1_suggere: Option<f64>,
    ) -> Result<()> {
        let tp_json = serde_json::to_string(&signal.take_profit)
            .map_err(|e| TradingError::Database(e.to_string()))?;

        sqlx::query(
            "INSERT OR IGNORE INTO signaux
             (id, asset, timeframe, direction, score, prix_entree,
              stop_loss, take_profit, strategie, cree_le,
              llm_valide, llm_conviction, llm_raison, llm_sl_suggere, llm_tp1_suggere)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(signal.id.to_string())
        .bind(signal.asset.as_str())
        .bind(signal.timeframe.as_str())
        .bind(format!("{:?}", signal.direction))
        .bind(signal.score)
        .bind(signal.prix_entree)
        .bind(signal.stop_loss)
        .bind(tp_json)
        .bind(&signal.strategie)
        .bind(signal.cree_le.timestamp())
        .bind(llm_valide)
        .bind(llm_conviction)
        .bind(llm_raison)
        .bind(llm_sl_suggere)
        .bind(llm_tp1_suggere)
        .execute(&self.pool)
        .await
        .map_err(|e| TradingError::Database(e.to_string()))?;

        Ok(())
    }
}
