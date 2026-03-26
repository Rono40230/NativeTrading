use actix_web::{web, HttpResponse, Responder};
use crate::state::AppState;

// ── POST /api/straddle/backtest ───────────────────────────────────────────────
/// Backteste un créneau straddle filtré par plage horaire et jour sur l'historique H1.
#[derive(serde::Deserialize)]
pub struct RequeteSlotBacktest {
    pub asset: String,
    pub heure_debut: String,
    pub jour_semaine: Option<i64>,
    pub capital: Option<f64>,
}

pub async fn handler_backtest_slot(
    state: web::Data<AppState>,
    body: web::Json<RequeteSlotBacktest>,
) -> impl Responder {
    use common::Timeframe;
    let asset = match crate::utils::parse_asset(&body.asset) {
        Some(a) => a,
        None => {
            return HttpResponse::BadRequest()
                .json(serde_json::json!({ "error": "Asset inconnu" }))
        }
    };
    let h_debut: u32 = body
        .heure_debut
        .splitn(2, ':')
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let capital = body.capital.unwrap_or(2000.0);
    let bougies = match state.db.obtenir_bougies(&asset, &Timeframe::H1, 5000).await {
        Ok(b) => b,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": e.to_string() }))
        }
    };
    let r = crate::straddle_slot_backtest::backtest_slot(
        &bougies,
        body.jour_semaine,
        h_debut,
        capital,
    );
    HttpResponse::Ok().json(serde_json::json!({
        "total_trades": r.total_trades,
        "win_rate": r.win_rate,
        "profit_factor": r.profit_factor,
        "max_drawdown_pct": r.max_drawdown_pct,
    }))
}
