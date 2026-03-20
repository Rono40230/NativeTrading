use actix_web::{web, HttpResponse, Responder};
use serde::Deserialize;

use crate::ollama;
use crate::ollama_types::{ReponseStraddleIA, RequeteStraddleIA};

// ─── Struct interne pour parser la réponse JSON du LLM ───────────────────────

#[derive(Deserialize)]
struct SignalStraddleBrut {
    signal: String, // "STRADDLE" | "WAIT"
    prix_entree: f64,
    sl_long: f64,
    sl_short: f64,
    tp1_long: f64,
    tp1_short: f64,
    tp2_long: f64,
    tp2_short: f64,
    score_confiance: f64,
    declencheur: String,
    raisonnement: String,
}

// ─── POST /api/ia/signal/straddle ─────────────────────────────────────────────
/// Génère un signal Straddle (Long + Short simultané) via Ollama.
/// Déclencheurs : annonce HIGH impact, ATR × 1.4 en Kill Zone, ou pattern récurrent.
pub async fn generer_signal_straddle(body: web::Json<RequeteStraddleIA>) -> impl Responder {
    use common::{Direction, Signal};

    let kz = body
        .kill_zone_active
        .unwrap_or_else(|| smc::kill_zone::est_en_kill_zone(chrono::Utc::now()));

    let sessions = body.sessions_actives.as_deref().unwrap_or(&[]).join(", ");

    let annonces = body
        .annonces_imminentes
        .as_deref()
        .unwrap_or(&[])
        .join(", ");

    let ratio_atr = if body.atr_moyen > 0.0 {
        body.atr_actuel / body.atr_moyen
    } else {
        1.0
    };

    let prompt = format!(
        "{}\n\nAsset: {} {}\n\
        Prix actuel: {:.5} | ATR actuel: {:.5} | ATR moyen: {:.5} | ratio_atr: {:.2}\n\
        kill_zone_active: {} | sessions: {}\n\
        Annonces imminentes: {}",
        crate::ollama::PROMPT_SIGNAL_STRADDLE,
        body.asset,
        body.timeframe,
        body.prix_actuel,
        body.atr_actuel,
        body.atr_moyen,
        ratio_atr,
        kz,
        sessions,
        if annonces.is_empty() {
            "aucune".to_string()
        } else {
            annonces
        },
    );

    let modele = std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "qwen2.5:14b".to_string());

    match ollama::interroger(&prompt).await {
        Err(e) => {
            HttpResponse::ServiceUnavailable().json(serde_json::json!({ "error": format!("{e}") }))
        }
        Ok(texte) => {
            let debut = texte.find('{').unwrap_or(0);
            let fin = texte.rfind('}').map(|i| i + 1).unwrap_or(texte.len());
            let Ok(brut) = serde_json::from_str::<SignalStraddleBrut>(&texte[debut..fin]) else {
                return HttpResponse::UnprocessableEntity().json(serde_json::json!({
                    "error": "Réponse LLM non parsable en JSON",
                    "brut": texte
                }));
            };

            let signal = if brut.signal == "STRADDLE" {
                let asset = parse_asset(&body.asset);
                let tf = parse_timeframe(&body.timeframe);
                // Direction::Both — stop_loss = sl_long (borne inférieure)
                // take_profit = [tp1_long, tp2_long] (cibles haussières)
                Some(Signal::nouveau(
                    asset,
                    tf,
                    Direction::Both,
                    brut.score_confiance * 10.0,
                    brut.prix_entree,
                    brut.sl_long,
                    vec![brut.tp1_long, brut.tp2_long],
                    "Straddle",
                ))
            } else {
                None
            };

            tracing::info!(
                "Straddle {} {}: signal={} confiance={:.2} déclencheur={}",
                body.asset,
                body.timeframe,
                brut.signal,
                brut.score_confiance,
                brut.declencheur,
            );

            HttpResponse::Ok().json(ReponseStraddleIA {
                signal,
                sl_long: brut.sl_long,
                sl_short: brut.sl_short,
                tp1_long: brut.tp1_long,
                tp1_short: brut.tp1_short,
                tp2_long: brut.tp2_long,
                tp2_short: brut.tp2_short,
                score_confiance: brut.score_confiance,
                declencheur: brut.declencheur,
                raisonnement: brut.raisonnement,
                modele,
            })
        }
    }
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn parse_asset(s: &str) -> common::Asset {
    use common::Asset;
    match s {
        "ETH" => Asset::ETH,
        "XAUUSD" => Asset::XAUUSD,
        "XAGUSD" => Asset::XAGUSD,
        "EURUSD" => Asset::EURUSD,
        "GBPJPY" => Asset::GBPJPY,
        "CADJPY" => Asset::CADJPY,
        "NZDJPY" => Asset::NZDJPY,
        "USDCAD" => Asset::USDCAD,
        "USDJPY" => Asset::USDJPY,
        "DAX" => Asset::DAX,
        "NAS100" => Asset::NAS100,
        "SP500" => Asset::SP500,
        _ => Asset::BTC,
    }
}

fn parse_timeframe(s: &str) -> common::Timeframe {
    use common::Timeframe;
    match s {
        "M1" => Timeframe::M1,
        "M5" => Timeframe::M5,
        "H1" => Timeframe::H1,
        "H4" => Timeframe::H4,
        "D1" => Timeframe::D1,
        "W1" => Timeframe::W1,
        _ => Timeframe::M15,
    }
}
