//! §4 (09/09, décision propriétaire) — **véto unlocks** rockets.
//!
//! Un candidat crypto avec un déverrouillage de tokens daté à moins de N
//! jours est ÉLIMINATOIRE (l'offre libérée par l'équipe/early investors
//! casse la dynamique de cassure — le scanner /10 est aveugle à ce risque).
//!
//! Sources, sans dépendance externe fragile (DefiLlama/CryptoRank = paywall
//! ou Cloudflare) :
//! 1. **Analyste IA** (quotidien) : lit les titres de dépêches DÉJÀ
//!    collectées (`presse_articles` + traduction FR `news_traductions`)
//!    mentionnant un unlock, et extrait symbole + date + montant —
//!    uniquement les unlocks DATÉS, jamais de date inventée.
//! 2. **Saisie propriétaire** (endpoint) : correction/ajout manuel.
//!
//! Constitution respectée : le véto est une RÈGLE MOTEUR aux seuils du
//! propriétaire (kv `rockets_unlock_jours`, `rockets_unlock_veto`) ; l'IA
//! ne fait que lire les dépêches et remplir la table.

use std::sync::Arc;

use actix_web::{web, HttpResponse, Responder};
use sqlx::Row;

use crate::state::AppState;
use db::Database;

const JOURS_DEFAUT: i64 = 30;

/// Ticker base sans suffixe pair (ARBUSDT → ARB).
fn base_symbole(symbole: &str) -> String {
    symbole.trim_end_matches("USDT").trim().to_uppercase()
}

/// Horizon du véto (kv, clampé 1-180).
async fn lire_jours(db: &Database) -> i64 {
    db.lire_config("rockets_unlock_jours")
        .await
        .ok()
        .flatten()
        .and_then(|s| s.parse::<i64>().ok())
        .filter(|n| (1..=180).contains(n))
        .unwrap_or(JOURS_DEFAUT)
}

/// Le véto est-il actif ? (kv `rockets_unlock_veto`, actif par défaut).
async fn veto_actif(db: &Database) -> bool {
    db.lire_config("rockets_unlock_veto")
        .await
        .ok()
        .flatten()
        .and_then(|s| s.parse::<i64>().ok())
        .map(|n| n != 0)
        .unwrap_or(true)
}

/// Veto pour un symbole ? `Some((date_unlock, source))` si un unlock daté
/// tombe dans l'horizon. Appelé par le scanner avant toute ouverture.
pub async fn est_veto(db: &Database, symbole: &str) -> Option<(i64, String)> {
    if !veto_actif(db).await {
        return None;
    }
    let limite = chrono::Utc::now().timestamp() + lire_jours(db).await * 86_400;
    let row: Option<(i64, String)> = sqlx::query_as(
        "SELECT date_unlock, source FROM unlocks_prochains
         WHERE symbole = ? AND date_unlock <= ?
         ORDER BY date_unlock LIMIT 1",
    )
    .bind(base_symbole(symbole))
    .bind(limite)
    .fetch_optional(db.pool())
    .await
    .ok()
    .flatten();
    row
}

/// Détection IA : dépêches des 7 derniers jours mentionnant un unlock,
/// titres FR en priorité. L'analyste n'extrait que des unlocks DATÉS et
/// cite la ligne (i) de la dépêche source.
async fn detecter(db: &Database) {
    let rows = sqlx::query(
        "SELECT a.titre AS titre_src, COALESCE(NULLIF(t.titre_fr, ''), a.titre) AS titre_fr
         FROM presse_articles a
         LEFT JOIN news_traductions t ON t.hash_titre = a.hash_titre
         WHERE a.publie_le >= datetime('now', '-7 days')
           AND (a.titre LIKE '%unlock%' OR t.titre_fr LIKE '%unlock%'
                OR a.titre LIKE '%déverrouil%' OR t.titre_fr LIKE '%déverrouil%')
         ORDER BY a.publie_le DESC LIMIT 60",
    )
    .fetch_all(db.pool())
    .await
    .unwrap_or_default();
    if rows.is_empty() {
        return;
    }
    let titres: Vec<String> = rows
        .iter()
        .map(|r| r.try_get::<String, _>("titre_fr").unwrap_or_default())
        .collect();
    let lignes: Vec<String> = titres
        .iter()
        .enumerate()
        .map(|(i, t)| format!("{}. {}", i + 1, t))
        .collect();
    let prompt = format!(
        "{}\n\nTitres de dépêches :\n{}\n\nRéponds UNIQUEMENT en JSON.",
        llm::prompt_effectif("unlock_detection"),
        lignes.join("\n"),
    );
    let Ok(texte) = llm::ollama::interroger(&prompt).await else {
        tracing::info!("🚫 Véto unlocks : analyste indisponible — réessai demain");
        return;
    };
    let debut = texte.find('[').unwrap_or(0);
    let fin = texte.rfind(']').map(|i| i + 1).unwrap_or(texte.len());
    let Ok(unlocks) = serde_json::from_str::<Vec<serde_json::Value>>(&texte[debut..fin]) else {
        tracing::warn!("🚫 Véto unlocks : réponse non parsable");
        return;
    };

    let maintenant = chrono::Utc::now().timestamp();
    let mut inseres = 0u32;
    for u in &unlocks {
        let Some(symbole) = u.get("symbole").and_then(|v| v.as_str()) else { continue };
        let Some(date) = u.get("date").and_then(|v| v.as_str()) else { continue };
        let Ok(jour) = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d") else { continue };
        let Some(ts) = jour.and_hms_opt(0, 0, 0).map(|d| d.and_utc().timestamp()) else { continue };
        // dépêche source = ligne i citée par l'analyste (1-based), repli 1
        let idx = u.get("i").and_then(|v| v.as_u64()).unwrap_or(1).saturating_sub(1) as usize;
        let source: String = titres
            .get(idx)
            .unwrap_or(&String::new())
            .chars()
            .take(80)
            .collect();
        let usd = u.get("usd").and_then(|v| v.as_f64());
        let res = sqlx::query(
            "INSERT OR IGNORE INTO unlocks_prochains (symbole, date_unlock, usd_estime, source, maj_le)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(base_symbole(symbole))
        .bind(ts)
        .bind(usd)
        .bind(source)
        .bind(maintenant)
        .execute(db.pool())
        .await;
        if res.map(|r| r.rows_affected() > 0).unwrap_or(false) {
            inseres += 1;
        }
    }
    if inseres > 0 {
        tracing::info!("🚫 Véto unlocks : {inseres} unlock(s) daté(s) extrait(s) des dépêches");
    }
}

/// Boucle de fond : détection quotidienne (boot + 24 h).
pub async fn boucle(db: Arc<Database>) {
    tracing::info!("🚫 Véto unlocks actif (quotidien, éliminatoire < N jours)");
    loop {
        detecter(&db).await;
        tokio::time::sleep(std::time::Duration::from_secs(86_400)).await;
    }
}

// ── Endpoints ────────────────────────────────────────────────────────────────

/// GET /api/rockets/unlocks — calendrier complet + réglages.
pub async fn lister(state: web::Data<AppState>) -> impl Responder {
    let jours = lire_jours(&state.db).await;
    let rows = sqlx::query(
        "SELECT symbole, date_unlock, usd_estime, source FROM unlocks_prochains
         WHERE date_unlock >= ? ORDER BY date_unlock ASC LIMIT 200",
    )
    .bind(chrono::Utc::now().timestamp() - 86_400)
    .fetch_all(state.db.pool())
    .await
    .unwrap_or_default();
    let horizon = chrono::Utc::now().timestamp() + jours * 86_400;
    let liste: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            let d = r.get::<i64, _>("date_unlock");
            serde_json::json!({
                "symbole": r.get::<String, _>("symbole"),
                "date_unlock": d,
                "usd_estime": r.try_get::<Option<f64>, _>("usd_estime").ok().flatten(),
                "source": r.get::<String, _>("source"),
                "veto_actif": d <= horizon,
            })
        })
        .collect();
    HttpResponse::Ok().json(serde_json::json!({
        "jours": jours,
        "actif": veto_actif(&state.db).await,
        "unlocks": liste,
    }))
}

#[derive(serde::Deserialize)]
pub struct BodyUnlock {
    pub symbole: String,
    /// YYYY-MM-DD
    pub date: String,
    pub usd: Option<f64>,
}

/// POST /api/rockets/unlocks — saisie propriétaire (ajout/correction).
pub async fn ajouter(state: web::Data<AppState>, body: web::Json<BodyUnlock>) -> impl Responder {
    let Ok(jour) = chrono::NaiveDate::parse_from_str(&body.date, "%Y-%m-%d") else {
        return HttpResponse::BadRequest().json(serde_json::json!({ "error": "Date attendue YYYY-MM-DD" }));
    };
    let Some(ts) = jour.and_hms_opt(0, 0, 0).map(|d| d.and_utc().timestamp()) else {
        return HttpResponse::BadRequest().json(serde_json::json!({ "error": "Date invalide" }));
    };
    let res = sqlx::query(
        "INSERT INTO unlocks_prochains (symbole, date_unlock, usd_estime, source, maj_le)
         VALUES (?, ?, ?, 'saisie propriétaire', strftime('%s','now'))
         ON CONFLICT(symbole, date_unlock) DO UPDATE SET
            usd_estime = excluded.usd_estime, source = excluded.source, maj_le = excluded.maj_le",
    )
    .bind(base_symbole(&body.symbole))
    .bind(ts)
    .bind(body.usd)
    .execute(state.db.pool())
    .await;
    match res {
        Ok(_) => HttpResponse::Ok().json(serde_json::json!({ "ok": true })),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({ "error": e.to_string() })),
    }
}

/// DELETE /api/rockets/unlocks/{symbole}/{date} — retrait propriétaire.
pub async fn retirer(state: web::Data<AppState>, path: web::Path<(String, String)>) -> impl Responder {
    let (symbole, date) = path.into_inner();
    let Ok(jour) = chrono::NaiveDate::parse_from_str(&date, "%Y-%m-%d") else {
        return HttpResponse::BadRequest().json(serde_json::json!({ "error": "Date attendue YYYY-MM-DD" }));
    };
    let Some(ts) = jour.and_hms_opt(0, 0, 0).map(|d| d.and_utc().timestamp()) else {
        return HttpResponse::BadRequest().json(serde_json::json!({ "error": "Date invalide" }));
    };
    let res = sqlx::query("DELETE FROM unlocks_prochains WHERE symbole = ? AND date_unlock = ?")
        .bind(base_symbole(&symbole))
        .bind(ts)
        .execute(state.db.pool())
        .await;
    match res {
        Ok(r) => HttpResponse::Ok().json(serde_json::json!({ "ok": true, "supprimes": r.rows_affected() })),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({ "error": e.to_string() })),
    }
}
