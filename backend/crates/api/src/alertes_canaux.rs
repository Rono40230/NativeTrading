//! Canaux d'alertes par famille (owner 21/10) : source unique App/Telegram/
//! Mobile. La modale « 🔔 Gérer les alertes » du bandeau écrit ici ; tous
//! les émetteurs interrogent `actif` avant d'envoyer ; l'app principale
//! notifie (OS+son) en pollant le journal universel mobile_alertes.

use actix_web::{web, HttpResponse};
use serde::Deserialize;
use sqlx::Row;

use crate::state::AppState;

#[derive(Debug, Clone, serde::Serialize)]
pub struct CanalAlerte {
    pub famille: String,
    pub app: bool,
    pub telegram: bool,
    pub mobile: bool,
}

/// Lit la table entière (une ligne par famille, defaults en migration).
pub async fn lire_tous(pool: &sqlx::SqlitePool) -> Vec<CanalAlerte> {
    sqlx::query("SELECT famille, app, telegram, mobile FROM alertes_canaux ORDER BY rowid")
        .fetch_all(pool)
        .await
        .unwrap_or_default()
        .iter()
        .map(|r| CanalAlerte {
            famille: r.try_get::<String, _>("famille").unwrap_or_default(),
            app: r.try_get::<i64, _>("app").unwrap_or(0) != 0,
            telegram: r.try_get::<i64, _>("telegram").unwrap_or(0) != 0,
            mobile: r.try_get::<i64, _>("mobile").unwrap_or(0) != 0,
        })
        .collect()
}

/// Décision pure : la famille est-elle active sur ce canal ? Famille
/// inconnue = actif (jamais un silence par accident).
pub async fn actif(pool: &sqlx::SqlitePool, famille: &str, canal: &str) -> bool {
    let colonne = match canal {
        "telegram" => "telegram",
        "app" => "app",
        _ => "mobile",
    };
    let r: Option<i64> = sqlx::query_scalar(&format!(
        "SELECT {colonne} FROM alertes_canaux WHERE famille = ?"
    ))
    .bind(famille)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();
    r.map(|v| v != 0).unwrap_or(true)
}

/// Envoi Telegram si la famille l'accepte (matrice complète owner 21/10).
/// Silencieux sans tokens ou canal coupé.
pub async fn telegram_si_actif(
    pool: &sqlx::SqlitePool,
    famille: &str,
    texte: &str,
) {
    if !actif(pool, famille, "telegram").await {
        return;
    }
    let (token, chat) = notifications::telegram::lire_tokens_pool(pool).await;
    if token.is_empty() || chat.is_empty() {
        return;
    }
    let _ = notifications::telegram::post_message(&token, &chat, texte).await;
}

/// GET /api/alertes-canaux — l'état complet (modale du bandeau + mobile).
pub async fn get(state: web::Data<AppState>) -> HttpResponse {
    HttpResponse::Ok().json(lire_tous(state.db.pool()).await)
}

#[derive(Deserialize)]
pub struct CorpsMaj {
    famille: String,
    canal: String,
    actif: bool,
}

/// PUT /api/alertes-canaux — un interrupteur à la fois (idempotent).
pub async fn maj(state: web::Data<AppState>, corps: web::Json<CorpsMaj>) -> HttpResponse {
    let colonne = match corps.canal.as_str() {
        "app" | "telegram" | "mobile" => corps.canal.as_str(),
        _ => return HttpResponse::BadRequest().json(serde_json::json!({ "error": "canal inconnu" })),
    };
    let res = sqlx::query(&format!(
        "UPDATE alertes_canaux SET {colonne} = ? WHERE famille = ?"
    ))
    .bind(if corps.actif { 1 } else { 0 })
    .bind(&corps.famille)
    .execute(state.db.pool())
    .await;
    match res {
        Ok(r) if r.rows_affected() > 0 => HttpResponse::Ok().json(serde_json::json!({ "ok": true })),
        Ok(_) => HttpResponse::NotFound().json(serde_json::json!({ "error": "famille inconnue" })),
        Err(e) => HttpResponse::InternalServerError().body(format!("{e}")),
    }
}
