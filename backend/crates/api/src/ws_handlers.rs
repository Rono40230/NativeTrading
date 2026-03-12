use actix_web::{web, HttpRequest, HttpResponse};
use actix_ws::Message;
use futures_util::StreamExt;
use serde::Serialize;

use crate::state::AppState;

#[derive(Serialize)]
struct StreamEvent {
    r#type: &'static str,
    asset: String,
    timeframe: String,
    data: serde_json::Value,
}

/// GET /api/stream?asset=XAUUSD&timeframe=M1
/// Stream WebSocket — IB Gateway sera le provider unique.
/// Actuellement : connexion acceptée, messages "status" en attente du provider IB.
pub async fn stream_market(
    req: HttpRequest,
    body: web::Payload,
    query: web::Query<std::collections::HashMap<String, String>>,
    _state: web::Data<AppState>,
) -> Result<HttpResponse, actix_web::Error> {
    let asset_str = query
        .get("asset")
        .cloned()
        .unwrap_or_else(|| "XAUUSD".to_string())
        .to_uppercase();
    let timeframe_str = query
        .get("timeframe")
        .cloned()
        .unwrap_or_else(|| "M15".to_string());

    let (response, mut session, mut client_stream) = actix_ws::handle(&req, body)?;

    actix_web::rt::spawn(async move {
        // Envoyer un statut initial
        let status = serde_json::json!({
            "type": "status",
            "asset": asset_str,
            "timeframe": timeframe_str,
            "message": "En attente de la connexion IB Gateway"
        });
        if let Ok(payload) = serde_json::to_string(&status) {
            let _ = session.text(payload).await;
        }

        // Maintenir la connexion ouverte jusqu'à déconnexion du client
        loop {
            match client_stream.recv().await {
                Some(Ok(Message::Close(_))) | None => break,
                Some(Err(_)) => break,
                Some(Ok(Message::Ping(bytes))) => {
                    if session.pong(&bytes).await.is_err() {
                        break;
                    }
                }
                _ => {}
            }
        }
        let _ = session.close(None).await;
    });

    Ok(response)
}
