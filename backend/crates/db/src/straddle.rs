use common::{Result, TradingError};
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};

// ── Type public ──────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StraddleCreneau {
    pub id: i64,
    pub asset: String,
    pub jour_semaine: Option<i64>, // 0=Lundi...6=Dimanche, NULL=tous les jours
    pub heure_debut: String,       // "14:00" UTC
    pub heure_fin: String,         // "16:00" UTC
    pub atr_moyen: Option<f64>,
    pub frequence: Option<f64>,      // 0.0–1.0
    pub llm_raison: Option<String>,
    pub llm_conviction: Option<i64>, // 0–100
    pub statut: String,              // 'a_tester' | 'valide' | 'invalide'
    pub cree_le: String,
    pub backtest_winrate: Option<f64>,
    pub backtest_profit_factor: Option<f64>,
}

// ── Lecture ──────────────────────────────────────────────────────────────────

pub async fn lister_creneaux(pool: &SqlitePool) -> Result<Vec<StraddleCreneau>> {
    let rows = sqlx::query(
        "SELECT id, asset, jour_semaine, heure_debut, heure_fin, atr_moyen,
                frequence, llm_raison, llm_conviction, statut, cree_le,
                backtest_winrate, backtest_profit_factor
         FROM straddle_creneaux
         ORDER BY llm_conviction DESC, cree_le DESC",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;

    Ok(rows
        .iter()
        .map(|r| StraddleCreneau {
            id: r.get("id"),
            asset: r.get("asset"),
            jour_semaine: r.get("jour_semaine"),
            heure_debut: r.get("heure_debut"),
            heure_fin: r.get("heure_fin"),
            atr_moyen: r.get("atr_moyen"),
            frequence: r.get("frequence"),
            llm_raison: r.get("llm_raison"),
            llm_conviction: r.get("llm_conviction"),
            statut: r.get("statut"),
            cree_le: r.get("cree_le"),
            backtest_winrate: r.get("backtest_winrate"),
            backtest_profit_factor: r.get("backtest_profit_factor"),
        })
        .collect())
}

pub async fn lister_creneaux_asset(
    pool: &SqlitePool,
    asset: &str,
) -> Result<Vec<StraddleCreneau>> {
    let rows = sqlx::query(
        "SELECT id, asset, jour_semaine, heure_debut, heure_fin, atr_moyen,
                frequence, llm_raison, llm_conviction, statut, cree_le,
                backtest_winrate, backtest_profit_factor
         FROM straddle_creneaux
         WHERE asset = ?
         ORDER BY llm_conviction DESC",
    )
    .bind(asset)
    .fetch_all(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;

    Ok(rows
        .iter()
        .map(|r| StraddleCreneau {
            id: r.get("id"),
            asset: r.get("asset"),
            jour_semaine: r.get("jour_semaine"),
            heure_debut: r.get("heure_debut"),
            heure_fin: r.get("heure_fin"),
            atr_moyen: r.get("atr_moyen"),
            frequence: r.get("frequence"),
            llm_raison: r.get("llm_raison"),
            llm_conviction: r.get("llm_conviction"),
            statut: r.get("statut"),
            cree_le: r.get("cree_le"),
            backtest_winrate: r.get("backtest_winrate"),
            backtest_profit_factor: r.get("backtest_profit_factor"),
        })
        .collect())
}

// ── Insertion ─────────────────────────────────────────────────────────────────

pub struct NouveauCreneau {
    pub asset: String,
    pub jour_semaine: Option<i64>,
    pub heure_debut: String,
    pub heure_fin: String,
    pub atr_moyen: Option<f64>,
    pub frequence: Option<f64>,
    pub llm_raison: Option<String>,
    pub llm_conviction: Option<i64>,
}

pub async fn inserer_creneaux(pool: &SqlitePool, creneaux: &[NouveauCreneau]) -> Result<()> {
    for c in creneaux {
        sqlx::query(
            "INSERT INTO straddle_creneaux
             (asset, jour_semaine, heure_debut, heure_fin, atr_moyen, frequence,
              llm_raison, llm_conviction)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&c.asset)
        .bind(c.jour_semaine)
        .bind(&c.heure_debut)
        .bind(&c.heure_fin)
        .bind(c.atr_moyen)
        .bind(c.frequence)
        .bind(&c.llm_raison)
        .bind(c.llm_conviction)
        .execute(pool)
        .await
        .map_err(|e| TradingError::Database(e.to_string()))?;
    }
    Ok(())
}

// ── Mise à jour ───────────────────────────────────────────────────────────────

pub async fn mettre_a_jour_creneau(
    pool: &SqlitePool,
    id: i64,
    statut: Option<String>,
    backtest_winrate: Option<f64>,
    backtest_profit_factor: Option<f64>,
) -> Result<()> {
    sqlx::query(
        "UPDATE straddle_creneaux
         SET statut                 = COALESCE(?, statut),
             backtest_winrate       = COALESCE(?, backtest_winrate),
             backtest_profit_factor = COALESCE(?, backtest_profit_factor)
         WHERE id = ?",
    )
    .bind(statut)
    .bind(backtest_winrate)
    .bind(backtest_profit_factor)
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;
    Ok(())
}

pub async fn supprimer_creneaux_asset(pool: &SqlitePool, asset: &str) -> Result<()> {
    sqlx::query("DELETE FROM straddle_creneaux WHERE asset = ? AND statut = 'a_tester'")
        .bind(asset)
        .execute(pool)
        .await
        .map_err(|e| TradingError::Database(e.to_string()))?;
    Ok(())
}
