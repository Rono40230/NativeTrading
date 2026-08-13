use actix_web::{web, HttpResponse, Responder};
use crate::state::AppState;

/// GET /api/ig/status — Force un re-login IG (bouton "Tester" dans Settings).
pub async fn ig_status(state: web::Data<AppState>) -> impl Responder {
    match state
        .ig_session
        .lock()
        .await
        .tester_connexion(&state.db)
        .await
    {
        Ok(()) => HttpResponse::Ok().json(serde_json::json!({
            "connecte": true,
            "source": "ig_markets"
        })),
        Err(e) => HttpResponse::Ok().json(serde_json::json!({
            "connecte": false,
            "source": "ig_markets",
            "erreur": format!("{}", e)
        })),
    }
}

/// GET /api/ig/statut-local — Retourne l'état de la session IG sans appel réseau.
/// Utilisé par le Dashboard pour afficher le badge.
/// Ne déclenche PLUS de re-login automatique (le circuit breaker dans IgSession
/// gère le cooldown). Le re-login ne se fait que via ig_status (bouton Tester).
pub async fn ig_statut_local(state: web::Data<AppState>) -> impl Responder {
    let connecte = state.ig_session.lock().await.est_connecte();
    HttpResponse::Ok().json(serde_json::json!({
        "connecte": connecte,
        "source": "ig_markets"
    }))
}

/// GET /api/ig/search?q=EURUSD
/// Recherche les marchés disponibles sur IG pour un terme donné.
/// Utilisé pour découvrir les epics valides pour le compte connecté.
pub async fn ig_search_markets(
    state: web::Data<AppState>,
    query: web::Query<std::collections::HashMap<String, String>>,
) -> impl Responder {
    let terme = match query.get("q") {
        Some(t)
            if !t.is_empty() && t.len() <= 20 && t.chars().all(|c| c.is_ascii_alphanumeric()) =>
        {
            t.to_uppercase()
        }
        _ => {
            return HttpResponse::BadRequest()
                .json(serde_json::json!({ "error": "Paramètre q requis (ex: EURUSD)" }))
        }
    };

    let (url_base, headers, client) = {
        let mut session = state.ig_session.lock().await;
        let url_base = session.url();
        let headers = match session.headers(&state.db).await {
            Ok(h) => h,
            Err(e) => {
                return HttpResponse::ServiceUnavailable()
                    .json(serde_json::json!({ "error": format!("Session IG: {}", e) }))
            }
        };
        let client = session.client().clone();
        (url_base, headers, client)
    };

    let url = format!("{}/markets?searchTerm={}", url_base, terme);
    match client
        .get(&url)
        .headers(headers)
        .header("Version", "1")
        .send()
        .await
    {
        Ok(r) if r.status().is_success() => match r.json::<serde_json::Value>().await {
            Ok(data) => HttpResponse::Ok().json(data),
            Err(e) => HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": format!("Parse: {}", e) })),
        },
        Ok(r) => HttpResponse::BadGateway()
            .json(serde_json::json!({ "error": format!("IG {}", r.status()) })),
        Err(e) => HttpResponse::ServiceUnavailable()
            .json(serde_json::json!({ "error": format!("{}", e) })),
    }
}

