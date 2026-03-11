use actix_web::{web, HttpResponse, Responder};
use serde::Deserialize;

use crate::state::AppState;

#[derive(Deserialize)]
pub struct ExportQuery {
    pub limit: Option<i64>,
}

/// GET /api/signaux/export — télécharge l'historique des signaux au format CSV
pub async fn exporter_signaux_csv(
    state: web::Data<AppState>,
    query: web::Query<ExportQuery>,
) -> impl Responder {
    let limit = query.limit.unwrap_or(500).min(5000);
    let signaux = match state.db.obtenir_signaux(limit).await {
        Ok(s) => s,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": format!("{}", e) }));
        }
    };

    let mut csv = String::from(
        "id,asset,timeframe,direction,score,prix_entree,stop_loss,take_profit,strategie,cree_le\n",
    );
    for s in &signaux {
        let tp = s["take_profit"]
            .as_array()
            .and_then(|a| a.first())
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        csv.push_str(&format!(
            "{},{},{},{},{:.2},{:.2},{:.2},{:.2},{},{}\n",
            s["id"].as_str().unwrap_or(""),
            s["asset"].as_str().unwrap_or(""),
            s["timeframe"].as_str().unwrap_or(""),
            s["direction"].as_str().unwrap_or(""),
            s["score"].as_f64().unwrap_or(0.0),
            s["prix_entree"].as_f64().unwrap_or(0.0),
            s["stop_loss"].as_f64().unwrap_or(0.0),
            tp,
            s["strategie"].as_str().unwrap_or(""),
            s["cree_le"].as_str().unwrap_or(""),
        ));
    }

    HttpResponse::Ok()
        .content_type("text/csv; charset=utf-8")
        .append_header(("Content-Disposition", "attachment; filename=\"signaux.csv\""))
        .body(csv)
}
