//! État d'historique par (asset, tf) pour l'EA MT5 — que manque-t-il ?
//!
//! L'EA interroge avant de pousser : si la base couvre déjà la profondeur
//! demandée (ou remonte assez loin), le TF est sauté ; sinon l'EA ne pousse
//! que les bougies PLUS ANCIENNES que le min en base (le delta). Un
//! redémarrage de l'EA sur une base déjà pleine devient instantané.

use actix_web::{web, HttpResponse, Responder};
use sqlx::Row;

use crate::state::AppState;

#[derive(serde::Deserialize)]
pub struct QueryEtat {
    pub asset: String,
    pub tf: String,
}

/// GET /api/mt5/historique/etat?asset=XAUUSD&tf=M1
/// → { "count": 720000, "min_ts": 1724000000 } (count 0 si vide)
pub async fn get_etat(
    state: web::Data<AppState>,
    query: web::Query<QueryEtat>,
) -> impl Responder {
    let Ok(row) = sqlx::query(
        "SELECT COUNT(*) as n, COALESCE(MIN(timestamp), 0) as min_ts
         FROM bougies WHERE asset = ? AND timeframe = ?",
    )
    .bind(&query.asset)
    .bind(&query.tf)
    .fetch_one(state.db.pool())
    .await
    else {
        return HttpResponse::Ok().json(serde_json::json!({ "count": 0, "min_ts": 0 }));
    };
    HttpResponse::Ok().json(serde_json::json!({
        "count": row.get::<i64, _>("n"),
        "min_ts": row.get::<i64, _>("min_ts"),
    }))
}
