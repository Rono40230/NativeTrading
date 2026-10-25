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
/// → { "count": 720000, "min_ts": 1724000000, "max_ts": 1787814000 }
/// (count 0 si vide). `max_ts` = dernière bougie — rattrapage des trous
/// RÉCENTS par l'EA (nuit PC éteint : l'ancien delta min_ts ne voyait que
/// l'historique ancien). Champ ADDITIF : l'EA v1.30 l'ignore sans erreur.
pub async fn get_etat(
    state: web::Data<AppState>,
    query: web::Query<QueryEtat>,
) -> impl Responder {
    // Source mt5 SEULE (bascule totale 24/10) : l'EA juge la profondeur AXI,
    // pas celle des anciennes séries Binance/Bybit — sinon il croit la base
    // complète et ne pousse jamais l'historique Axi de M5→W1 (cause des
    // H4/D1 crypto restés vides après la bascule).
    let Ok(row) = sqlx::query(
        "SELECT COUNT(*) as n, COALESCE(MIN(timestamp), 0) as min_ts,
                COALESCE(MAX(timestamp), 0) as max_ts
         FROM bougies WHERE asset = ? AND timeframe = ? AND source = 'mt5'",
    )
    .bind(&query.asset)
    .bind(&query.tf)
    .fetch_one(state.db.pool())
    .await
    else {
        return HttpResponse::Ok().json(serde_json::json!({ "count": 0, "min_ts": 0, "max_ts": 0 }));
    };
    HttpResponse::Ok().json(serde_json::json!({
        "count": row.get::<i64, _>("n"),
        "min_ts": row.get::<i64, _>("min_ts"),
        "max_ts": row.get::<i64, _>("max_ts"),
    }))
}
