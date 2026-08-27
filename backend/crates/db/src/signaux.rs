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

    /// Étape 6 (préalable stats) — marque le REMPLISSAGE du trade (l'ordre
    /// au bord de la zone a été touché : le trade existe au marché). Un
    /// signal jamais rempli puis expiré n'est pas un trade — les stats de
    /// réussite ne portent que sur les remplis.
    pub async fn marquer_remplie_par_cle(&self, cle_moteur: &str, asset: &str, ts: i64) -> Result<u64> {
        let res = sqlx::query(
            "UPDATE signaux SET heure_entree = ? WHERE cle_moteur = ? AND asset = ? AND statut = 'Actif' AND heure_entree IS NULL",
        )
        .bind(ts)
        .bind(cle_moteur)
        .bind(asset)
        .execute(&self.pool)
        .await
        .map_err(|e| TradingError::Database(e.to_string()))?;
        Ok(res.rows_affected())
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
    /// Ferme le signal officiel correspondant à une clé moteur, avec son
    /// verdict (TP1/TP2/TP3/SL/BE/Expire), son prix de sortie et son R réel.
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
        let res = sqlx::query(
            "UPDATE signaux SET statut = 'Fermé', verdict = ?, prix_verdict = ?, r_realise = ?, ferme_le = ?
             WHERE cle_moteur = ? AND asset = ? AND statut = 'Actif'",
        )
        .bind(verdict)
        .bind(prix_verdict)
        .bind(r_realise)
        .bind(ferme_le)
        .bind(cle_moteur)
        .bind(asset)
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
        cle_moteur: &str,
    ) -> Result<()> {
        let tp_long_json = serde_json::to_string(&signal.take_profit)
            .map_err(|e| TradingError::Database(e.to_string()))?;
        let tp_short_json = serde_json::to_string(take_profit_short)
            .map_err(|e| TradingError::Database(e.to_string()))?;

        sqlx::query(
            "INSERT OR IGNORE INTO signaux
             (id, asset, timeframe, direction, score, prix_entree,
              stop_loss, take_profit, strategie, cree_le,
              sl_short, take_profit_short, heure_entree, cle_moteur)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
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
        .bind(cle_moteur)
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

// ── Performance par stratégie (étape 3 — blocs dashboard) ────────────────────

/// Point de la courbe des trades : une clôture horodatée avec son R.
#[derive(Debug, Serialize)]
pub struct PointCloture {
    pub ferme_le: i64,
    pub r: f64,
    pub r_cumule: f64,
    pub verdict: String,
    pub asset: String,
    pub timeframe: String,
    pub direction: String,
}

/// Signal en cours (résumé compact pour le bloc).
#[derive(Debug, Serialize)]
pub struct SignalEnCours {
    pub asset: String,
    pub timeframe: String,
    pub direction: String,
    pub force: i32,
    pub prix_entree: f64,
    pub cree_le: i64,
}

/// Performance complète d'une stratégie : courbe R cumulé + stats + en-cours.
#[derive(Debug, Serialize)]
pub struct PerformanceStrategie {
    pub clotures: Vec<PointCloture>,
    pub en_cours: Vec<SignalEnCours>,
    /// Trades REMPLIS clôturés (la base des stats).
    pub total: usize,
    pub gagnants: usize,
    /// Clôturés sans jamais avoir rempli (ordres non touchés) — à part.
    pub non_remplis: usize,
    pub taux_reussite: f64,
    pub r_total: f64,
}

impl Database {
    /// Étape 3 — performance d'une stratégie (courbe des trades clôturés
    /// en R cumulé + signaux en cours). L'état passe par la table, pas le
    /// manifeste : les stratégies en Observation ont un historique aussi.
    pub async fn performance_strategie(&self, id: &str) -> Result<PerformanceStrategie> {
        // STATS = TRADES REMPLIS uniquement (heure_entree non null) : un
        // ordre jamais touché puis expiré n'a jamais engagé de capital.
        let rows = sqlx::query(
            "SELECT ferme_le, verdict, r_realise, asset, timeframe, direction
             FROM signaux
             WHERE strategie = ? AND statut = 'Fermé' AND verdict IS NOT NULL
               AND heure_entree IS NOT NULL
             ORDER BY ferme_le ASC",
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| TradingError::Database(e.to_string()))?;

        let mut cumul = 0.0;
        let mut clotures = Vec::with_capacity(rows.len());
        let mut gagnants = 0usize;
        for r in &rows {
            let verdict: String = r.get("verdict");
            let r_val = r.try_get::<f64, _>("r_realise").ok().unwrap_or(0.0);
            // Gagnant = R réalisé > 0 — englobe TP* (SMC) et TS/TimeStop
            // positifs (straddle), indépendamment du vocabulaire de verdict.
            if r_val > 0.0 {
                gagnants += 1;
            }
            cumul += r_val;
            clotures.push(PointCloture {
                ferme_le: r.try_get::<i64, _>("ferme_le").ok().unwrap_or(0),
                r: r_val,
                r_cumule: cumul,
                verdict,
                asset: r.get("asset"),
                timeframe: r.get("timeframe"),
                direction: r.get("direction"),
            });
        }
        let total = clotures.len();

        let actifs = sqlx::query(
            "SELECT asset, timeframe, direction, score, prix_entree, cree_le
             FROM signaux WHERE strategie = ? AND statut = 'Actif'
             ORDER BY cree_le DESC LIMIT 8",
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| TradingError::Database(e.to_string()))?;

        // Jamais remplis (clôturés sans toucher l'entrée) — comptés à part.
        let non_remplis: usize = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM signaux
             WHERE strategie = ? AND statut = 'Fermé' AND heure_entree IS NULL",
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0) as usize;

        let en_cours = actifs
            .iter()
            .map(|r| SignalEnCours {
                asset: r.get("asset"),
                timeframe: r.get("timeframe"),
                direction: r.get("direction"),
                force: (r.get::<f64, _>("score") as i64).clamp(1, 10) as i32,
                prix_entree: r.get("prix_entree"),
                cree_le: r.get("cree_le"),
            })
            .collect();

        Ok(PerformanceStrategie {
            taux_reussite: if total > 0 {
                gagnants as f64 / total as f64
            } else {
                0.0
            },
            total,
            gagnants,
            non_remplis,
            r_total: cumul,
            clotures,
            en_cours,
        })
    }
}
