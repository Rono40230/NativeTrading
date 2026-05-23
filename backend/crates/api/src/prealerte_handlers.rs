use actix_web::{web, HttpResponse, Responder};
use serde::Deserialize;
use sqlx::Row;

use crate::state::AppState;

#[derive(Deserialize)]
pub struct QueryPreAlertes {
    pub limit: Option<i64>,
}

/// GET /api/pre_alertes?limit=N — pré-alertes récentes (setups en formation)
pub async fn get_pre_alertes(
    state: web::Data<AppState>,
    query: web::Query<QueryPreAlertes>,
) -> impl Responder {
    let limit = query.limit.unwrap_or(20);
    let rows = sqlx::query(
        "SELECT id, asset, strategie, raison, score_actuel, evenement, minutes_avant, cree_le
         FROM pre_alertes
         ORDER BY cree_le DESC
         LIMIT ?",
    )
    .bind(limit)
    .fetch_all(state.db.pool())
    .await;

    match rows {
        Ok(liste) => {
            let json: Vec<serde_json::Value> = liste
                .iter()
                .map(|r| {
                    serde_json::json!({
                        "id":           r.get::<String, _>("id"),
                        "asset":        r.get::<String, _>("asset"),
                        "strategie":    r.get::<String, _>("strategie"),
                        "raison":       r.get::<String, _>("raison"),
                        "score_actuel": r.get::<Option<f64>, _>("score_actuel"),
                        "evenement":    r.get::<Option<String>, _>("evenement"),
                        "minutes_avant":r.get::<Option<i64>, _>("minutes_avant"),
                        "cree_le":      r.get::<String, _>("cree_le"),
                    })
                })
                .collect();
            HttpResponse::Ok().json(json)
        }
        Err(e) => {
            tracing::error!("GET pre_alertes: {}", e);
            HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": e.to_string() }))
        }
    }
}
