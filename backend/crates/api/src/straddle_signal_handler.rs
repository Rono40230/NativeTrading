use actix_web::{web, HttpResponse, Responder};
use chrono::{Datelike, Timelike, Utc};
use common::{Direction, Signal, Timeframe};
use std::time::Duration;

use crate::state::AppState;
use crate::straddle_prompt::PROMPT_SIGNAL_STRADDLE;
use crate::straddle_types::{OllamaResp, ReponseLlm, RequeteStraddleSignal};
use crate::utils::parse_asset;

// ── Handler : POST /api/ia/signal/straddle ───────────────────────────────────

pub async fn generer_signal_straddle(
    state: web::Data<AppState>,
    body: web::Json<RequeteStraddleSignal>,
) -> impl Responder {
    // ── Garde-fous immédiats (sans appel LLM) ──
    let positions = body.positions_actives.unwrap_or(0);
    let drawdown = body.drawdown_actuel_pct.unwrap_or(0.0);
    if positions >= 3 {
        return HttpResponse::Ok().json(serde_json::json!({
            "signal": "WAIT",
            "raison": "Exposition maximale atteinte (3 positions actives)",
            "score_confiance": 0.0
        }));
    }
    if drawdown >= 18.0 {
        return HttpResponse::Ok().json(serde_json::json!({
            "signal": "WAIT",
            "raison": format!("Drawdown {:.1}% — trop proche du seuil d'arrêt (20%)", drawdown),
            "score_confiance": 0.0
        }));
    }

    let asset_str = body.asset.trim().to_uppercase();
    let asset = match parse_asset(&asset_str) {
        Some(a) => a,
        None => {
            return HttpResponse::BadRequest()
                .json(serde_json::json!({ "error": "Asset non supporté" }))
        }
    };

    let ratio_atr = if body.atr_moyen_14 > 0.0 {
        body.atr_actuel / body.atr_moyen_14
    } else {
        1.0
    };

    let now = Utc::now();
    let kill_zone = body
        .kill_zone_active
        .unwrap_or_else(|| smc::kill_zone::est_en_kill_zone(now));

    // Annonces HIGH impact imminentes (< 90 min)
    let maintenant = now.timestamp();
    let dans_90min = maintenant + 5400;
    let annonces: Vec<serde_json::Value> = state
        .db
        .lire_calendrier_cache(3600)
        .await
        .unwrap_or_default()
        .into_iter()
        .filter(|a| {
            a["impact"].as_str() == Some("High")
                && a["date_heure"]
                    .as_str()
                    .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                    .map(|dt| {
                        let ts = dt.timestamp();
                        ts >= maintenant && ts <= dans_90min
                    })
                    .unwrap_or(false)
        })
        .collect();

    // Créneaux historiques validés pour cet asset (contexte LLM)
    let creneaux = db::straddle::lister_creneaux_asset(state.db.pool(), &asset_str)
        .await
        .unwrap_or_default();
    let creneaux_actifs: Vec<_> = creneaux
        .iter()
        .filter(|c| c.statut == "valide" || c.statut == "a_tester")
        .collect();

    // Construction du contexte
    let jours = [
        "Lundi", "Mardi", "Mercredi", "Jeudi", "Vendredi", "Samedi", "Dimanche",
    ];
    let jour = jours[now.weekday().num_days_from_monday() as usize % 7];
    let heure = now.hour();

    let mut ctx = format!(
        "=== CONTEXTE STRADDLE TEMPS RÉEL ===\n\
        Asset: {asset_str} | Timeframe: {tf} | {jour} {heure:02}h UTC\n\
        Prix: {prix:.5} | ATR actuel: {atr:.5} | ATR moyen 14p: {moy:.5} | Ratio ATR: {ratio:.2}×\n\
        Kill Zone active: {kz} | Positions ouvertes: {pos} | Drawdown: {dd:.1}%\n",
        tf = body.timeframe,
        prix = body.prix_actuel,
        atr = body.atr_actuel,
        moy = body.atr_moyen_14,
        ratio = ratio_atr,
        kz = kill_zone,
        pos = positions,
        dd = drawdown,
    );

    if annonces.is_empty() {
        ctx.push_str("Annonces HIGH impact < 90min: aucune\n");
    } else {
        ctx.push_str("Annonces HIGH impact < 90min:\n");
        for a in &annonces {
            let dans = a["date_heure"]
                .as_str()
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| (dt.timestamp() - maintenant) / 60)
                .unwrap_or(0);
            ctx.push_str(&format!(
                "  - {} | {} | dans {}min\n",
                a["titre"].as_str().unwrap_or("?"),
                a["devise"].as_str().unwrap_or("?"),
                dans
            ));
        }
    }

    if !creneaux_actifs.is_empty() {
        ctx.push_str("Créneaux validés:\n");
        for c in creneaux_actifs.iter().take(3) {
            ctx.push_str(&format!(
                "  {}h–{}h | ATR×{:.2} | freq {:.0}% | wr {}%\n",
                c.heure_debut,
                c.heure_fin,
                c.atr_moyen.unwrap_or(0.0),
                c.frequence.unwrap_or(0.0) * 100.0,
                c.backtest_winrate
                    .map(|w| format!("{:.0}", w))
                    .unwrap_or_else(|| "?".to_string())
            ));
        }
    } else {
        ctx.push_str("Créneaux historiques: aucun\n");
    }

    let prompt = format!("{PROMPT_SIGNAL_STRADDLE}\n\n{ctx}");
    let modele = std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "qwen2.5:14b".to_string());
    let url = std::env::var("OLLAMA_URL")
        .unwrap_or_else(|_| "http://localhost:11434/api/chat".to_string());

    let corps = serde_json::json!({
        "model": modele,
        "messages": [{"role": "user", "content": prompt}],
        "stream": false,
        "options": { "temperature": 0.1, "num_predict": 300 }
    });

    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return HttpResponse::ServiceUnavailable()
                .json(serde_json::json!({ "error": e.to_string() }))
        }
    };

    let reponse = match client.post(&url).json(&corps).send().await {
        Ok(r) => r,
        Err(e) => {
            return HttpResponse::ServiceUnavailable().json(serde_json::json!({
                "error": format!("Ollama indisponible: {}", e)
            }))
        }
    };

    let data: OllamaResp = match reponse.json().await {
        Ok(d) => d,
        Err(e) => {
            return HttpResponse::UnprocessableEntity()
                .json(serde_json::json!({ "error": format!("JSON Ollama: {}", e) }))
        }
    };

    let texte = data.message.content;

    // Extraction robuste : cherche le premier { et le dernier } correspondant.
    // Si le LLM enveloppe dans {"response":{...}}, on descend d'un niveau.
    let debut = texte.find('{').unwrap_or(0);
    let fin = texte.rfind('}').map(|i| i + 1).unwrap_or(texte.len());
    let fragment = &texte[debut..fin];

    // Tentative directe, puis descente dans le premier objet imbriqué si ça échoue.
    let brut: ReponseLlm = match serde_json::from_str::<ReponseLlm>(fragment) {
        Ok(b) => b,
        Err(_) => {
            // Le LLM a peut-être enveloppé dans {"response":{...}} ou similaire.
            let inner = serde_json::from_str::<serde_json::Value>(fragment)
                .ok()
                .and_then(|v| {
                    v.as_object()?
                        .values()
                        .find_map(|child| serde_json::from_value::<ReponseLlm>(child.clone()).ok())
                });
            match inner {
                Some(b) => b,
                None => {
                    tracing::warn!(
                        "Straddle LLM: réponse non parsable — {}",
                        &texte[..texte.len().min(200)]
                    );
                    return HttpResponse::Ok().json(serde_json::json!({
                        "signal": "WAIT",
                        "raison": "Réponse LLM non parsable",
                        "score_confiance": 0.0
                    }));
                }
            }
        }
    };

    if brut.signal != "STRADDLE" || brut.score_confiance < 6.0 {
        return HttpResponse::Ok().json(serde_json::json!({
            "signal": "WAIT",
            "raison": brut.raison,
            "score_confiance": brut.score_confiance
        }));
    }

    // Calcul SL/TP (3 niveaux par jambe)
    let atr = body.atr_actuel;
    let prix = body.prix_actuel;
    let sl_long = prix - 0.5 * atr;
    let sl_short = prix + 0.5 * atr;
    let tp1_long = prix + 2.0 * atr;
    let tp1_short = prix - 2.0 * atr;
    let tp2_long = prix + 3.5 * atr;
    let tp2_short = prix - 3.5 * atr;
    let tp3_long = prix + 5.0 * atr;
    let tp3_short = prix - 5.0 * atr;

    // Enregistrement du signal en DB (deux jambes complètes)
    let tf = match body.timeframe.as_str() {
        "M1" => Timeframe::M1,
        "M5" => Timeframe::M5,
        "H1" => Timeframe::H1,
        _ => Timeframe::M15,
    };
    let signal = Signal::nouveau(
        asset,
        tf,
        Direction::Both,
        brut.score_confiance * 10.0,
        prix,
        sl_long,
        vec![tp1_long, tp2_long, tp3_long],
        "Straddle",
    );
    let signal_id = signal.id.to_string();
    let _ = state
        .db
        .inserer_signal_straddle_complet(&signal, sl_short, &[tp1_short, tp2_short, tp3_short])
        .await;

    HttpResponse::Ok().json(serde_json::json!({
        "signal": "STRADDLE",
        "declencheur": brut.declencheur,
        "raison": brut.raison,
        "score_confiance": brut.score_confiance,
        "amplitude_attendue_pct": brut.amplitude_attendue_pct,
        "duree_exposition_estimee_min": brut.duree_exposition_estimee_min,
        "signal_id": signal_id,
        "prix_entree": prix,
        "sl_long": sl_long,
        "sl_short": sl_short,
        "tp1_long": tp1_long,
        "tp1_short": tp1_short,
        "tp2_long": tp2_long,
        "tp2_short": tp2_short,
        "tp3_long": tp3_long,
        "tp3_short": tp3_short,
        "modele": modele
    }))
}
