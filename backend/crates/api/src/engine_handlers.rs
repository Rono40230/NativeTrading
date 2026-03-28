//! Endpoints REST pour le Signal Engine — start / stop / status / stream WS.
use actix_web::{web, HttpRequest, HttpResponse, Responder};
use chrono::Utc;

use crate::state::AppState;

// ── Commandes ────────────────────────────────────────────────────────────────

/// POST /api/signal-engine/start
/// Démarre la boucle de génération automatique de signaux.
pub async fn demarrer_engine(state: web::Data<AppState>) -> impl Responder {
    if state.signal_engine.est_actif() {
        return HttpResponse::Ok().json(serde_json::json!({
            "statut": "deja_actif",
            "message": "Signal Engine déjà en cours d'exécution"
        }));
    }

    state
        .signal_engine
        .demarrer(state.db.clone(), state.pipeline_ml.clone());

    tracing::info!("Signal Engine démarré via API");
    HttpResponse::Ok().json(serde_json::json!({
        "statut": "demarre",
        "message": "Signal Engine démarré — analyse toutes les 5 min"
    }))
}

/// POST /api/signal-engine/stop
/// Arrête la boucle de génération.
pub async fn arreter_engine(state: web::Data<AppState>) -> impl Responder {
    if !state.signal_engine.est_actif() {
        return HttpResponse::Ok().json(serde_json::json!({
            "statut": "deja_arrete",
            "message": "Signal Engine déjà arrêté"
        }));
    }

    state.signal_engine.arreter();

    tracing::info!("Signal Engine arrêté via API");
    HttpResponse::Ok().json(serde_json::json!({
        "statut": "arrete",
        "message": "Signal Engine arrêté"
    }))
}

/// GET /api/signal-engine/status
/// Retourne l'état courant du moteur et les statistiques 24h.
pub async fn statut_engine(state: web::Data<AppState>) -> impl Responder {
    let actif = state.signal_engine.est_actif();
    let ts_prochain = state.signal_engine.ts_prochain_cycle();
    let maintenant = Utc::now().timestamp();

    // Secondes avant le prochain cycle (0 si passé ou inactif)
    let secs_restantes = if actif && ts_prochain > maintenant {
        ts_prochain - maintenant
    } else {
        0
    };

    // Signaux générés dans les dernières 24h
    let signaux_24h = state.db.compter_signaux_recents(24 * 60).await.unwrap_or(0);

    HttpResponse::Ok().json(serde_json::json!({
        "actif": actif,
        "prochain_cycle_dans_secs": secs_restantes,
        "signaux_24h": signaux_24h,
        "assets_surveilles": 13,
        "timeframes": ["M5", "M15"],
        "intervalle_secs": 300
    }))
}

/// GET /api/signal-engine/stream
/// WebSocket — reçoit chaque nouveau signal généré par le moteur.
pub async fn stream_signaux(
    req: HttpRequest,
    body: web::Payload,
    state: web::Data<AppState>,
) -> Result<HttpResponse, actix_web::Error> {
    let (response, mut session, _client_stream) = actix_ws::handle(&req, body)?;

    let mut rx = state.signal_engine.abonner();

    actix_web::rt::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(signal) => {
                    let json = match serde_json::to_string(&signal) {
                        Ok(s) => s,
                        Err(e) => {
                            tracing::warn!("Signal sérialisation WS: {}", e);
                            continue;
                        }
                    };
                    if session.text(json).await.is_err() {
                        break; // Client déconnecté
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("WS signal stream — {} messages manqués", n);
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });

    Ok(response)
}
