use actix_web::{web, HttpResponse, Responder};
use db::rockets::{self, NouveauRocket};
use serde::Deserialize;

use crate::rockets_scan;
use crate::state::AppState;

// ── Config endpoints ─────────────────────────────────────────────────────────

/// GET /api/rockets/config
pub async fn get_config(state: web::Data<AppState>) -> impl Responder {
    let cfg = rockets::lire_config(state.db.pool()).await;
    HttpResponse::Ok().json(cfg)
}

/// PUT /api/rockets/config
pub async fn put_config(
    state: web::Data<AppState>,
    body: web::Json<rockets::RocketsConfig>,
) -> impl Responder {
    match rockets::sauvegarder_config(state.db.pool(), &body).await {
        Ok(()) => HttpResponse::Ok().json(serde_json::json!({ "ok": true })),
        Err(e) => {
            HttpResponse::InternalServerError().json(serde_json::json!({ "error": e.to_string() }))
        }
    }
}

// ── DTOs ────────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct RequeteSauvegarder {
    pub ticker: String,
    pub phase: String,
    pub score: i64,
    pub prix_entree: f64,
    pub stop_loss: f64,
    pub target: f64,
    pub target2: Option<f64>,
    pub target3: Option<f64>,
    pub ratio_volume: f64,
    pub atr_ratio: f64,
    pub atr14: Option<f64>,
    pub rsi: f64,
}

#[derive(Deserialize)]
pub struct QueryHistorique {
    pub limite: Option<i64>,
}

// ── Handlers ─────────────────────────────────────────────────────────────────

/// POST /api/rockets/signal — enregistre un signal détecté côté frontend
pub async fn sauvegarder_signal(
    state: web::Data<AppState>,
    body: web::Json<RequeteSauvegarder>,
) -> impl Responder {
    let pool = state.db.pool();
    let nouveau = NouveauRocket {
        ticker: body.ticker.clone(),
        phase: body.phase.clone(),
        score: body.score,
        prix_entree: body.prix_entree,
        stop_loss: body.stop_loss,
        target: body.target,
        target2: body.target2,
        target3: body.target3,
        ratio_volume: body.ratio_volume,
        atr_ratio: body.atr_ratio,
        atr14: body.atr14,
        rsi: body.rsi,
        llm_valide: None,
        llm_conviction: None,
        llm_raison: None,
        llm_sl_suggere: None,
        llm_tp1_suggere: None,
        trailing_coeff: 2.5,
        pct_tp1: 0.25,
        pct_tp2: 0.25,
        pct_trailing: 0.50,
        entree_limite: None,
        entree_stop: None,
        niveau_invalidation: None,
        type_entree_rec: None,
    };
    match rockets::sauvegarder(pool, &nouveau).await {
        Ok(Some(id)) => {
            let ticker_base = body
                .ticker
                .trim_end_matches("USDT")
                .trim_end_matches("USD")
                .trim_end_matches("BTC");
            if let Some(asset) = crate::utils::parse_asset(ticker_base) {
                use common::{Direction, Signal, Timeframe};
                let tp1 = body.target;
                let signal = Signal::nouveau(
                    asset,
                    Timeframe::M15,
                    Direction::Long,
                    body.score as f64,
                    body.prix_entree,
                    body.stop_loss,
                    vec![tp1, body.target2.unwrap_or(tp1), body.target3.unwrap_or(tp1)],
                    "Rockets",
                );
            }
            HttpResponse::Ok().json(serde_json::json!({ "id": id, "nouveau": true }))
        }
        Ok(None) => HttpResponse::Ok().json(serde_json::json!({ "nouveau": false })),
        Err(e) => {
            tracing::error!("Sauvegarde rocket: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({ "error": e.to_string() }))
        }
    }
}

/// GET /api/rockets/scan — résultats du dernier scan worker
pub async fn get_scan(state: web::Data<AppState>) -> impl Responder {
    use strategies::rockets_indicateurs::MAX_DISPLAY;
    let results = rockets_scan::get_scan_results();
    let total = rockets_scan::get_total_candidats();
    let locked = results.read().await;
    let nb_total = *total.read().await;

    // Exclure les tickers qui ont déjà un trade actif (statut='ouvert')
    let tickers_actifs: std::collections::HashSet<String> = match rockets::lister_ouverts(state.db.pool()).await {
        Ok(ouverts) => ouverts.into_iter().map(|s| s.ticker).collect(),
        Err(_) => std::collections::HashSet::new(),
    };

    let signaux: Vec<_> = locked
        .iter()
        .filter(|r| !tickers_actifs.contains(&r.ticker))
        .take(MAX_DISPLAY)
        .collect();
    HttpResponse::Ok().json(serde_json::json!({
        "signaux": signaux,
        "total_candidats": nb_total,
    }))
}

/// GET /api/rockets/scan/debug
/// Lit les résultats du dernier scan worker et retourne les candidats momentum-compression
/// (phase=compression + change1h ≥ seuil) AVANT filtre LLM.
pub async fn scan_momentum_debug() -> impl Responder {
    use db::rockets;

    const CHANGE_1H_MOMENTUM_MIN: f64 = 0.5;
    const SCORE_MOMENTUM_MIN: i64 = 15;

    // Lire les résultats déjà calculés par le worker (pas de re-scan)
    let scan_lock = crate::rockets_scan::get_scan_results();
    let total_eligibles_lock = crate::rockets_scan::get_total_candidats();
    let resultats = scan_lock.read().await;
    let total_eligibles = *total_eligibles_lock.read().await;
    let cfg = rockets::RocketsConfig::default();

    // Distribution des phases pour diagnostic
    let mut nb_compression = 0usize;
    let mut nb_prelancement = 0usize;
    let mut nb_breakout = 0usize;
    for r in resultats.iter() {
        match r.phase.as_str() {
            "compression" => nb_compression += 1,
            "prelancement" => nb_prelancement += 1,
            "breakout" => nb_breakout += 1,
            _ => {}
        }
    }

    // Toutes les compressions avec leurs métriques
    let compressions_detail: Vec<serde_json::Value> = resultats
        .iter()
        .filter(|r| r.phase == "compression")
        .map(|r| {
            serde_json::json!({
                "ticker":   r.ticker,
                "score":    r.score,
                "change1h": r.change1h,
                "rsi":      r.rsi,
                "atrRatio": r.atr_ratio,
                "volRatio": r.ratio_volume,
                "passeScore":   r.score >= SCORE_MOMENTUM_MIN,
                "passe1h":      r.change1h >= CHANGE_1H_MOMENTUM_MIN,
                "passeRsi":     r.rsi <= cfg.rsi_max,
            })
        })
        .collect();

    let momentum: Vec<serde_json::Value> = resultats
        .iter()
        .filter(|r| {
            r.phase == "compression"
                && r.change1h >= CHANGE_1H_MOMENTUM_MIN
                && r.score >= SCORE_MOMENTUM_MIN
                && r.rsi <= cfg.rsi_max
        })
        .map(|r| {
            serde_json::json!({
                "ticker":   r.ticker,
                "score":    r.score,
                "change1h": r.change1h,
                "rsi":      r.rsi,
                "atrRatio": r.atr_ratio,
                "volRatio": r.ratio_volume,
            })
        })
        .collect();

    HttpResponse::Ok().json(serde_json::json!({
        "source":                   "dernier_scan_worker",
        "total_eligibles_usdt":     total_eligibles,    // paires USDT avec volume suffisant
        "total_avec_phase":         resultats.len(),    // assets avec phase détectée (atrRatio<0.80)
        "distribution_phases": {
            "compression":  nb_compression,
            "prelancement": nb_prelancement,
            "breakout":     nb_breakout,
        },
        "compressions_detail":      compressions_detail,
        "total_candidats_momentum": momentum.len(),
        "seuil_change1h":           CHANGE_1H_MOMENTUM_MIN,
        "seuil_score":              SCORE_MOMENTUM_MIN,
        "candidats":                momentum,
    }))
}

/// GET /api/rockets/historique?limite=50 — uniquement les trades clôturés (statut='ferme')
pub async fn get_historique(
    state: web::Data<AppState>,
    query: web::Query<QueryHistorique>,
) -> impl Responder {
    let pool = state.db.pool();
    let limite = query.limite.unwrap_or(50);
    match rockets::historique(pool, limite).await {
        Ok(liste) => HttpResponse::Ok().json(liste),
        Err(e) => {
            tracing::error!("Historique rockets: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({ "error": e.to_string() }))
        }
    }
}

/// GET /api/rockets/actifs — trades en cours (statut='ouvert' ou 'attente')
pub async fn get_actifs(state: web::Data<AppState>) -> impl Responder {
    match rockets::lister_actifs(state.db.pool()).await {
        Ok(liste) => HttpResponse::Ok().json(liste),
        Err(e) => {
            tracing::error!("Rockets actifs: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({ "error": e.to_string() }))
        }
    }
}

/// DELETE /api/rockets/signal/{id} — annule et supprime un signal actif
pub async fn supprimer_signal(
    state: web::Data<AppState>,
    path: web::Path<i64>,
) -> impl Responder {
    let id = path.into_inner();
    match rockets::supprimer(state.db.pool(), id).await {
        Ok(true)  => HttpResponse::Ok().json(serde_json::json!({ "ok": true })),
        Ok(false) => HttpResponse::NotFound()
            .json(serde_json::json!({ "error": "Signal introuvable ou déjà clôturé" })),
        Err(e) => {
            tracing::error!("Suppression rocket {}: {}", id, e);
            HttpResponse::InternalServerError().json(serde_json::json!({ "error": e.to_string() }))
        }
    }
}
