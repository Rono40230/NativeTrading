use crate::state::AppState;
use actix_web::{web, HttpResponse, Responder};

// ── POST /api/straddle/backtest ───────────────────────────────────────────────
/// Backteste un créneau straddle filtré par plage horaire et jour sur l'historique H1.
#[derive(serde::Deserialize)]
pub struct RequeteSlotBacktest {
    pub asset: String,
    pub heure_debut: String,
    pub heure_fin: Option<String>,
    pub jour_semaine: Option<i64>,
    pub capital: Option<f64>,
    /// Si fourni, backteste sur une fenêtre centrée [timing - avant_min, timing + apres_min]
    /// avec des bougies M5 (ex: "14:32"). Sinon, backtest H1 sur l'heure de heure_debut.
    pub timing_optimal: Option<String>,
    pub avant_min: Option<u32>, // défaut 15
    pub apres_min: Option<u32>, // défaut 30
}

pub async fn handler_backtest_slot(
    state: web::Data<AppState>,
    body: web::Json<RequeteSlotBacktest>,
) -> impl Responder {
    use common::Timeframe;
    let asset = match crate::utils::parse_asset(&body.asset) {
        Some(a) => a,
        None => {
            return HttpResponse::BadRequest().json(serde_json::json!({ "error": "Asset inconnu" }))
        }
    };
    let capital = body.capital.unwrap_or(2000.0);
    let r = if let Some(ref timing) = body.timing_optimal {
        // Fenêtre centrée sur le timing précis — bougies M5
        let avant = body.avant_min.unwrap_or(15);
        let apres = body.apres_min.unwrap_or(30);
        let bougies = match state
            .db
            .obtenir_bougies(&asset, &Timeframe::M5, 20_000)
            .await
        {
            Ok(b) => b,
            Err(e) => {
                return HttpResponse::InternalServerError()
                    .json(serde_json::json!({ "error": e.to_string() }))
            }
        };
        crate::straddle_slot_backtest_fenetre::backtest_slot_fenetre(
            &bougies,
            body.jour_semaine,
            timing,
            avant,
            apres,
            capital,
        )
    } else {
        // Fallback : backtest H1 sur l'heure entière
        let h_debut: u32 = body
            .heure_debut
            .split(':')
            .next()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let h_fin: Option<u32> = body
            .heure_fin
            .as_deref()
            .and_then(|s| s.split(':').next())
            .and_then(|s| s.parse().ok());
        let bougies = match state.db.obtenir_bougies(&asset, &Timeframe::H1, 5000).await {
            Ok(b) => b,
            Err(e) => {
                return HttpResponse::InternalServerError()
                    .json(serde_json::json!({ "error": e.to_string() }))
            }
        };
        crate::straddle_slot_backtest::backtest_slot(
            &bougies,
            body.jour_semaine,
            h_debut,
            h_fin,
            capital,
        )
    };
    HttpResponse::Ok().json(serde_json::json!({
        "total_trades": r.total_trades,
        "win_rate": r.win_rate,
        "profit_factor": r.profit_factor,
        "max_drawdown_pct": r.max_drawdown_pct,
        "esperance_pct": r.esperance_pct,
        "payoff_ratio": r.payoff_ratio,
        "serie_pertes_max": r.serie_pertes_max,
        "direction_dominante": r.direction_dominante,
        "amplitude_moyenne": r.amplitude_moyenne,
    }))
}
