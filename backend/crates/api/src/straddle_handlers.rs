use actix_web::{web, HttpResponse, Responder};
use chrono::{DateTime, Duration, Utc};
use serde::Deserialize;

use crate::ollama;
use crate::ollama_types::{ReponseStraddleIA, RequeteStraddleIA};
use crate::state::AppState;

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
/// S21 — les annonces HIGH impact dans les 2 prochaines heures sont auto-injectées
/// depuis le cache calendrier si le client ne les fournit pas.
pub async fn generer_signal_straddle(
    state: web::Data<AppState>,
    body: web::Json<RequeteStraddleIA>,
) -> impl Responder {
    use common::{Direction, Signal};

    let kz = body
        .kill_zone_active
        .unwrap_or_else(|| smc::kill_zone::est_en_kill_zone(chrono::Utc::now()));

    let sessions = body.sessions_actives.as_deref().unwrap_or(&[]).join(", ");

    // ── S21 : auto-injection annonces High < 2h ──────────────────────────────
    let annonces_auto: Vec<String> = match &body.annonces_imminentes {
        Some(a) => a.clone(),
        None => {
            let horizon = Utc::now() + Duration::hours(2);
            match state.db.lire_calendrier_cache(7200).await {
                Ok(cache) => cache
                    .into_iter()
                    .filter(|ev| {
                        let est_high = ev["impact"].as_str() == Some("High");
                        let avant_horizon = ev["date_heure"]
                            .as_str()
                            .and_then(|s| s.parse::<DateTime<Utc>>().ok())
                            .map(|dt| dt > Utc::now() && dt <= horizon)
                            .unwrap_or(false);
                        est_high && avant_horizon
                    })
                    .map(|ev| {
                        format!(
                            "{} {} ({})",
                            ev["devise"].as_str().unwrap_or("?"),
                            ev["titre"].as_str().unwrap_or("?"),
                            ev["date_heure"]
                                .as_str()
                                .unwrap_or("?")
                                .get(11..16)
                                .unwrap_or("?")
                        )
                    })
                    .collect(),
                Err(e) => {
                    tracing::warn!("Straddle: lecture cache calendrier: {}", e);
                    vec![]
                }
            }
        }
    };
    let annonces = if annonces_auto.is_empty() {
        "aucune".to_string()
    } else {
        annonces_auto.join(", ")
    };

    let ratio_atr = if body.atr_moyen > 0.0 {
        body.atr_actuel / body.atr_moyen
    } else {
        1.0
    };

    if !annonces_auto.is_empty() {
        tracing::info!(
            "Straddle {}: {} annonce(s) High < 2h injectées: {}",
            body.asset,
            annonces_auto.len(),
            annonces
        );
    }

    // Contexte historique Straddle pour nourrir le LLM
    let historique_raw = state.db.obtenir_contexte_llm(&body.asset, 5).await;
    let contexte_historique = crate::ollama::formater_contexte_historique(
        &body.asset,
        "Straddle",
        &historique_raw,
    );

    let prompt = format!(
        "{contexte}{base}\n\nAsset: {asset} {tf}\n\
        Prix actuel: {prix:.5} | ATR actuel: {atr_a:.5} | ATR moyen: {atr_m:.5} | ratio_atr: {ratio:.2}\n\
        kill_zone_active: {kz} | sessions: {sessions}\n\
        Annonces imminentes: {annonces}",
        contexte = contexte_historique,
        base = crate::ollama::PROMPT_SIGNAL_STRADDLE,
        asset = body.asset,
        tf = body.timeframe,
        prix = body.prix_actuel,
        atr_a = body.atr_actuel,
        atr_m = body.atr_moyen,
        ratio = ratio_atr,
        kz = kz,
        sessions = sessions,
        annonces = if annonces.is_empty() { "aucune".to_string() } else { annonces },
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
    crate::utils::parse_asset(s).unwrap_or(common::Asset::BTC)
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
