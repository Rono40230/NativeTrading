use actix_web::{web, HttpResponse, Responder};
use serde::Deserialize;

use crate::ollama;
use crate::ollama_types::{ReponseSignalIA, RequeteSignalIA};
use crate::state::AppState;

// ─── POST /api/ia/signal ─────────────────────────────────────────────────────
#[derive(Deserialize)]
struct SignalBrut {
    direction: String,
    prix_entree: f64,
    stop_loss: f64,
    tp1: f64,
    tp2: f64,
    tp3: f64,
    score_confiance: f64,
    niveau_invalidation: f64,
    confluences: Vec<String>,
    raisonnement: String,
}

/// POST /api/ia/signal — Signal SMC structuré (JSON) via LLM Ollama.
pub async fn generer_signal(
    state: web::Data<AppState>,
    body: web::Json<RequeteSignalIA>,
) -> impl Responder {
    use common::{Asset, Direction, Signal, Timeframe};

    let historique_raw = state.db.obtenir_contexte_llm(&body.asset, 5).await;
    let contexte = crate::ollama::formater_contexte_historique(
        &body.asset,
        "SMC Directionnel",
        &historique_raw,
    );

    let smc_signal_prompt = crate::prompts_handler::prompt_effectif("smc_signal");
    let prompt = format!(
        "{contexte}{base}\n\nAsset: {asset} {tf}\nPrix actuel: {prix:.5} | ATR: {atr:.5}\n\
        kill_zone_active: {kz} | sweep_detecte: {sw}\n\
        SMC: Tendance={tend:.1} OB={ob:.1} Imbalance={imb:.1} IFVG={ifvg:.1} Fib={fib:.1}\n\
        ML confiance={ml:.1}% | Score SMC total={score:.1}/100",
        contexte = contexte,
        base = smc_signal_prompt,
        asset = body.asset,
        tf = body.timeframe,
        prix = body.prix_actuel,
        atr = body.atr,
        kz = body
            .kill_zone_active
            .unwrap_or_else(|| smc::kill_zone::est_en_kill_zone(chrono::Utc::now())),
        sw = body.sweep_detecte.unwrap_or(false),
        tend = body.tendance,
        ob = body.order_block,
        imb = body.imbalance,
        ifvg = body.ifvg,
        fib = body.fibonacci,
        ml = body.confiance_ml * 100.0,
        score = body.score_smc,
    );
    let modele = std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "qwen2.5:14b".to_string());

    match ollama::interroger(&prompt).await {
        Err(e) => {
            HttpResponse::ServiceUnavailable().json(serde_json::json!({ "error": format!("{e}") }))
        }
        Ok(texte) => {
            let debut: usize = texte.find('{').unwrap_or(0);
            let fin: usize = texte.rfind('}').map(|i| i + 1).unwrap_or(texte.len());
            let Ok(brut) = serde_json::from_str::<SignalBrut>(&texte[debut..fin]) else {
                return HttpResponse::UnprocessableEntity().json(serde_json::json!({
                    "error": "Réponse LLM non parsable en JSON", "brut": texte
                }));
            };
            let signal = if brut.direction != "Neutre" {
                let asset = crate::utils::parse_asset(&body.asset).unwrap_or(Asset::BTC);
                let tf = match body.timeframe.as_str() {
                    "M1" => Timeframe::M1,
                    "M5" => Timeframe::M5,
                    "H1" => Timeframe::H1,
                    "H4" => Timeframe::H4,
                    "D1" => Timeframe::D1,
                    "W1" => Timeframe::W1,
                    _ => Timeframe::M15,
                };
                let dir = if brut.direction == "Short" {
                    Direction::Short
                } else {
                    Direction::Long
                };
                Some(Signal::nouveau(
                    asset,
                    tf,
                    dir,
                    brut.score_confiance * 10.0,
                    brut.prix_entree,
                    brut.stop_loss,
                    vec![brut.tp1, brut.tp2, brut.tp3],
                    "SMC-IA",
                ))
            } else {
                None
            };

            // Pipeline unifié : DB + broadcast (modale) + Telegram
            if let Some(ref sig) = signal {
                let _ = state.db.inserer_signal(sig).await;
                state.signal_engine.publier(sig.clone());
                crate::telegram::notifier_telegram(sig.clone());
            }

            HttpResponse::Ok().json(ReponseSignalIA {
                signal,
                score_confiance: brut.score_confiance,
                niveau_invalidation: brut.niveau_invalidation,
                confluences: brut.confluences,
                raisonnement: brut.raisonnement,
                modele,
            })
        }
    }
}
