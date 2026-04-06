use common::{Result, TradingError};
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};

// ── Types publics ─────────────────────────────────────────────────────────────

/// Pic de volatilité à insérer en DB.
#[derive(Debug)]
pub struct NouveauPic {
    pub asset: String,
    pub timeframe: String,
    pub timestamp_pic: i64,
    pub prix: f64,
    pub atr_actuel: f64,
    pub atr_moyen_14: f64,
    pub ratio_atr: f64,
    pub categorie: String,
    pub evenement_nom: Option<String>,
    pub evenement_devise: Option<String>,
    pub evenement_impact: Option<String>,
    pub minutes_avant_evt: Option<i64>,
    pub session_active: String,
    pub kill_zone_active: bool,
}

/// Pic lu depuis la DB (pour les endpoints API et le prompt LLM).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PicDetecte {
    pub id: i64,
    pub asset: String,
    pub timeframe: String,
    pub timestamp_pic: i64,
    pub prix: f64,
    pub atr_actuel: f64,
    pub atr_moyen_14: f64,
    pub ratio_atr: f64,
    pub categorie: String,
    pub evenement_nom: Option<String>,
    pub evenement_devise: Option<String>,
    pub evenement_impact: Option<String>,
    pub minutes_avant_evt: Option<i64>,
    pub session_active: String,
    pub kill_zone_active: bool,
    pub signal_id: Option<String>,
    pub cree_le: i64,
}

// ── Opérations DB ─────────────────────────────────────────────────────────────

/// Insère un nouveau pic et retourne son ID autoincrémenté.
pub async fn inserer_pic(pool: &SqlitePool, pic: &NouveauPic) -> Result<i64> {
    let row = sqlx::query(
        "INSERT INTO straddle_pics
         (asset, timeframe, timestamp_pic, prix, atr_actuel, atr_moyen_14, ratio_atr,
          categorie, evenement_nom, evenement_devise, evenement_impact, minutes_avant_evt,
          session_active, kill_zone_active)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         RETURNING id",
    )
    .bind(&pic.asset)
    .bind(&pic.timeframe)
    .bind(pic.timestamp_pic)
    .bind(pic.prix)
    .bind(pic.atr_actuel)
    .bind(pic.atr_moyen_14)
    .bind(pic.ratio_atr)
    .bind(&pic.categorie)
    .bind(&pic.evenement_nom)
    .bind(&pic.evenement_devise)
    .bind(&pic.evenement_impact)
    .bind(pic.minutes_avant_evt)
    .bind(&pic.session_active)
    .bind(pic.kill_zone_active as i64)
    .fetch_one(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;

    Ok(row.get::<i64, _>("id"))
}

/// Lie un signal généré à un pic existant.
pub async fn lier_signal(pool: &SqlitePool, pic_id: i64, signal_id: &str) -> Result<()> {
    sqlx::query("UPDATE straddle_pics SET signal_id = ? WHERE id = ?")
        .bind(signal_id)
        .bind(pic_id)
        .execute(pool)
        .await
        .map_err(|e| TradingError::Database(e.to_string()))?;
    Ok(())
}

/// Retourne les pics des N dernières heures (toutes les paires), triés du plus récent.
pub async fn lister_recents(pool: &SqlitePool, heures: u32, limit: i64) -> Result<Vec<PicDetecte>> {
    let seuil = chrono::Utc::now().timestamp() - (heures as i64 * 3600);
    let rows = sqlx::query(
        "SELECT id, asset, timeframe, timestamp_pic, prix, atr_actuel, atr_moyen_14, ratio_atr,
                categorie, evenement_nom, evenement_devise, evenement_impact, minutes_avant_evt,
                session_active, kill_zone_active, signal_id, cree_le
         FROM straddle_pics
         WHERE cree_le >= ?
         ORDER BY cree_le DESC
         LIMIT ?",
    )
    .bind(seuil)
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;

    Ok(rows.iter().map(mapper_pic).collect())
}

/// Retourne les pics récents pour un asset donné.
pub async fn lister_recents_asset(
    pool: &SqlitePool,
    asset: &str,
    heures: u32,
    limit: i64,
) -> Result<Vec<PicDetecte>> {
    let seuil = chrono::Utc::now().timestamp() - (heures as i64 * 3600);
    let rows = sqlx::query(
        "SELECT id, asset, timeframe, timestamp_pic, prix, atr_actuel, atr_moyen_14, ratio_atr,
                categorie, evenement_nom, evenement_devise, evenement_impact, minutes_avant_evt,
                session_active, kill_zone_active, signal_id, cree_le
         FROM straddle_pics
         WHERE asset = ? AND cree_le >= ?
         ORDER BY cree_le DESC
         LIMIT ?",
    )
    .bind(asset)
    .bind(seuil)
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;

    Ok(rows.iter().map(mapper_pic).collect())
}

/// Dernier pic enregistré pour un asset/timeframe (pour le lier au signal généré).
pub async fn dernier_pic_asset(
    pool: &SqlitePool,
    asset: &str,
    timeframe: &str,
    dans_les_dernieres_min: i64,
) -> Result<Option<i64>> {
    let seuil = chrono::Utc::now().timestamp() - dans_les_dernieres_min * 60;
    let row = sqlx::query(
        "SELECT id FROM straddle_pics
         WHERE asset = ? AND timeframe = ? AND cree_le >= ?
         ORDER BY cree_le DESC
         LIMIT 1",
    )
    .bind(asset)
    .bind(timeframe)
    .bind(seuil)
    .fetch_optional(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;

    Ok(row.map(|r| r.get::<i64, _>("id")))
}

/// Charge un pic par son ID — utilisé pour enrichir le feedback au moment du signal.
pub async fn charger_par_id(pool: &SqlitePool, pic_id: i64) -> Result<Option<PicDetecte>> {
    let row = sqlx::query(
        "SELECT id, asset, timeframe, timestamp_pic, prix, atr_actuel, atr_moyen_14, ratio_atr,
                categorie, evenement_nom, evenement_devise, evenement_impact, minutes_avant_evt,
                session_active, kill_zone_active, signal_id, cree_le
         FROM straddle_pics WHERE id = ?",
    )
    .bind(pic_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;

    Ok(row.as_ref().map(mapper_pic))
}

// ── Helpers internes ──────────────────────────────────────────────────────────

fn mapper_pic(r: &sqlx::sqlite::SqliteRow) -> PicDetecte {
    PicDetecte {
        id: r.get("id"),
        asset: r.get("asset"),
        timeframe: r.get("timeframe"),
        timestamp_pic: r.get("timestamp_pic"),
        prix: r.get("prix"),
        atr_actuel: r.get("atr_actuel"),
        atr_moyen_14: r.get("atr_moyen_14"),
        ratio_atr: r.get("ratio_atr"),
        categorie: r.get("categorie"),
        evenement_nom: r.get("evenement_nom"),
        evenement_devise: r.get("evenement_devise"),
        evenement_impact: r.get("evenement_impact"),
        minutes_avant_evt: r.get("minutes_avant_evt"),
        session_active: r.get("session_active"),
        kill_zone_active: r.get::<i64, _>("kill_zone_active") == 1,
        signal_id: r.get("signal_id"),
        cree_le: r.get("cree_le"),
    }
}
