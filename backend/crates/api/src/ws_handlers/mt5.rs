//! Flux graphique des actifs MT5/Axi — protocole identique au flux Binance
//! (`candle`/`historical_start`/`historical_end`) : la bougie EN FORMATION
//! poussée par l'EA chaque seconde est servie telle quelle (vrai OHLC).
//! L'historique reste celui du REST (déjà chargé par le chart) — le batch
//! vide préserve les données du store.

use actix_web::{web, HttpRequest, HttpResponse};
use actix_ws::Message;
use std::time::Duration;

use crate::state::AppState;

/// Bucket M10 EN FORMATION d'un actif MT5 : fusion des M1 fermées de la
/// bucket courante (DB) avec la M1 vivante poussée par l'EA. Retour None
/// si aucune donnée (EA muet + DB vide pour la fenêtre).
async fn bucket_m10_formation(
    state: &actix_web::web::Data<AppState>,
    asset: &str,
) -> Option<(i64, f64, f64, f64, f64, f64)> {
    let maintenant = chrono::Utc::now().timestamp();
    let debut_bucket = maintenant / 600 * 600;
    let m1_fermees = state
        .db
        .obtenir_bougies(&common::Asset::from(asset), &common::Timeframe::M1, 10)
        .await
        .unwrap_or_default()
        .into_iter()
        .filter(|b| b.timestamp.timestamp() >= debut_bucket)
        .collect::<Vec<_>>();
    let vivante = crate::mt5_collecteur::bougie_en_formation(asset, "M1");
    if m1_fermees.is_empty() && vivante.is_none() {
        return None;
    }
    let mut o = f64::NAN; let mut h = f64::NEG_INFINITY; let mut l = f64::INFINITY;
    let mut c = f64::NAN; let mut v = 0.0;
    for b in &m1_fermees {
        if o.is_nan() { o = b.open; }
        h = h.max(b.high); l = l.min(b.low); c = b.close; v += b.volume;
    }
    if let Some((_, vo, vh, vl, vc, vv)) = vivante {
        if o.is_nan() { o = vo; }
        h = h.max(vh); l = l.min(vl); c = vc; v += vv;
    }
    Some((debut_bucket, o, h, l, c, v))
}

pub async fn stream_mt5(
    req: HttpRequest,
    body: web::Payload,
    query: web::Query<std::collections::HashMap<String, String>>,
    state: web::Data<AppState>,
) -> Result<HttpResponse, actix_web::Error> {
    let asset = query.get("asset").cloned().unwrap_or_default().to_uppercase();
    let tf = query.get("timeframe").cloned().unwrap_or_else(|| "M15".into());

    let (response, mut session, mut client) = actix_ws::handle(&req, body)?;
    actix_web::rt::spawn(async move {
        let _ = session.text(r#"{"type":"historical_start"}"#).await;
        let _ = session.text(r#"{"type":"historical_end"}"#).await;

        let mut tick = tokio::time::interval(Duration::from_secs(1));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tokio::select! {
                _ = tick.tick() => {
                    // M10 : l'EA ne pousse que la M1 en formation — la bucket
                    // de 10 min est reconstruite en fusionnant les M1 fermées
                    // de la bucket (DB) avec la M1 vivante.
                    if tf == "M10" {
                        if let Some((debut, o, h, l, c, v)) =
                            bucket_m10_formation(&state, &asset).await
                        {
                            let msg = serde_json::json!({
                                "type": "candle",
                                "data": { "timestamp": debut, "open": o, "high": h,
                                          "low": l, "close": c, "volume": v }
                            });
                            if session.text(serde_json::to_string(&msg).unwrap_or_default())
                                .await
                                .is_err()
                            {
                                return;
                            }
                        }
                        continue;
                    }
                    if let Some((debut, o, h, l, c, v)) =
                        crate::mt5_collecteur::bougie_en_formation(&asset, &tf)
                    {
                        let msg = serde_json::json!({
                            "type": "candle",
                            "data": { "timestamp": debut, "open": o, "high": h,
                                      "low": l, "close": c, "volume": v }
                        });
                        if session.text(serde_json::to_string(&msg).unwrap_or_default())
                            .await
                            .is_err()
                        {
                            return;
                        }
                    }
                }
                msg = client.recv() => match msg {
                    Some(Ok(Message::Close(_))) | None => return,
                    Some(Ok(Message::Ping(b))) => { let _ = session.pong(&b).await; }
                    _ => {}
                }
            }
        }
    });
    Ok(response)
}
