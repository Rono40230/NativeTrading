//! Snapshot des features ML au moment d'émission d'un signal Straddle.
//!
//! 56 features = 52 OHLCV standard + 4 contextuelles Straddle :
//!   [52] ratio_atr  (f64 direct)
//!   [53] categorie  (choc_isole=0.0, serie_chocs=1.0, calendrier_eco=2.0, kill_zone=3.0, autres=4.0)
//!   [54] session    (London=0.0, London+NY=1.0, New York=2.0, Asia/Off=3.0, autres=4.0)
//!   [55] score_llm  (0.0–10.0)
//!
//! Label : `straddle_feedback.gagnant = 1` (au moins une jambe TP1+).

use sqlx::{Row, SqlitePool};

// ── Encodages contextuels ─────────────────────────────────────────────────────

pub fn encoder_categorie(cat: &str) -> f64 {
    match cat {
        "choc_isole"     => 0.0,
        "serie_chocs"    => 1.0,
        "calendrier_eco" => 2.0,
        "kill_zone"      => 3.0,
        _                => 4.0,
    }
}

pub fn encoder_session(session: &str) -> f64 {
    match session {
        "London"    => 0.0,
        "London+NY" => 1.0,
        "New York"  => 2.0,
        "Asia/Off"  => 3.0,
        _           => 4.0,
    }
}

/// Construit le vecteur 56 features depuis les 52 OHLCV + contexte Straddle.
pub fn construire_features_56(
    features_ohlcv: &[f64],
    ratio_atr: f64,
    categorie: &str,
    session: &str,
    score_llm: f64,
) -> Vec<f64> {
    let mut v = Vec::with_capacity(56);
    v.extend_from_slice(&features_ohlcv[..features_ohlcv.len().min(52)]);
    // Compléter à 52 si le vecteur est plus court (sécurité)
    while v.len() < 52 {
        v.push(0.0);
    }
    v.push(ratio_atr);
    v.push(encoder_categorie(categorie));
    v.push(encoder_session(session));
    v.push(score_llm);
    v
}

// ── Écriture ──────────────────────────────────────────────────────────────────

/// Persiste le vecteur 56 features associé à un signal Straddle.
/// `signal_id` = UUID TEXT de `signaux.id`.
/// Opération ignorée si un snapshot existe déjà pour ce signal.
pub async fn inserer_snapshot(
    pool: &SqlitePool,
    signal_id: &str,
    ticker: &str,
    features: &[f64],
) -> anyhow::Result<()> {
    let json = serde_json::to_string(features)?;
    sqlx::query(
        "INSERT OR IGNORE INTO straddle_features_snapshot (signal_id, ticker, features_json)
         VALUES (?, ?, ?)",
    )
    .bind(signal_id)
    .bind(ticker)
    .bind(&json)
    .execute(pool)
    .await?;
    Ok(())
}

// ── Lecture ───────────────────────────────────────────────────────────────────

/// Retourne les features d'un signal donné, si le snapshot existe.
pub async fn lire_snapshot(
    pool: &SqlitePool,
    signal_id: &str,
) -> anyhow::Result<Option<Vec<f64>>> {
    let row = sqlx::query(
        "SELECT features_json FROM straddle_features_snapshot WHERE signal_id = ?",
    )
    .bind(signal_id)
    .fetch_optional(pool)
    .await?;

    match row {
        Some(r) => {
            let json: String = r.get("features_json");
            Ok(Some(serde_json::from_str(&json)?))
        }
        None => Ok(None),
    }
}

/// Retourne tous les snapshots avec leur label (1.0 = gagnant, 0.0 = perdant/expire).
/// Utilisé par `straddle_trainer` pour entraîner le modèle fine-tuné.
/// Ne retourne que les signaux clôturés avec un verdict et un snapshot.
pub async fn lire_snapshots_avec_labels(
    pool: &SqlitePool,
) -> anyhow::Result<Vec<(Vec<f64>, f64)>> {
    let rows = sqlx::query(
        r#"SELECT s.features_json,
               CAST(COALESCE(sf.pnl_r, COALESCE(sf.gagnant, 0)) AS REAL) AS label
         FROM straddle_features_snapshot s
         JOIN straddle_feedback sf ON sf.signal_id = s.signal_id
         WHERE sf.verdict IS NOT NULL AND sf.gagnant IS NOT NULL"#,
    )
    .fetch_all(pool)
    .await?;

    let mut result = Vec::with_capacity(rows.len());
    for row in rows {
        let json: String = row.get("features_json");
        let label: f64 = row.get("label");
        let features: Vec<f64> = serde_json::from_str(&json)?;
        result.push((features, label));
    }
    Ok(result)
}
