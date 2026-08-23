use actix_web::{web, HttpResponse, Responder};
use chrono::{Datelike, Timelike, Utc};
use common::{Direction, Signal, Timeframe};

use llm::ReponseOllama;
use crate::state::AppState;
use crate::straddle_types::{ReponseLlm, RequeteStraddleSignal};
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

    ctx.push_str(&crate::straddle_utils::formater_annonces_contexte(&annonces, maintenant));

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

    // /no_think : mode non-thinking Qwen3 — classification de contexte macro
    let prompt = format!(
        "{}\n\n{ctx}\n/no_think",
        llm::prompt_effectif("straddle_signal")
    );
    let modele = std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "qwen3:32b".to_string());
    let url = std::env::var("OLLAMA_URL")
        .unwrap_or_else(|_| "http://localhost:11434/api/chat".to_string());

    let corps = serde_json::json!({
        "model": modele,
        "messages": [{"role": "user", "content": prompt}],
        "stream": false,
        "options": { "temperature": 0.7, "num_predict": 300 }
    });

    let _permit = llm::OLLAMA_SEMAPHORE.acquire().await.ok();
    let client = &*llm::OLLAMA_HTTP_CLIENT;

    let reponse = client.post(&url).json(&corps).send().await
        .map_err(|e| HttpResponse::ServiceUnavailable().json(serde_json::json!({
            "error": format!("Ollama indisponible: {}", e)
        })));
    let reponse = match reponse { Ok(r) => r, Err(r) => return r };

    let data: ReponseOllama = match reponse.json().await {
        Ok(d) => d,
        Err(e) => {
            return HttpResponse::UnprocessableEntity()
                .json(serde_json::json!({ "error": format!("JSON Ollama: {}", e) }))
        }
    };

    let texte = llm::filtrer_think(data.message.content);

    // Extraction robuste : cherche le premier { et le dernier } correspondant.
    // Si le LLM enveloppe dans {"response":{...}}, on descend d'un niveau.
    let debut = texte.find('{').unwrap_or(0);
    let fin = texte.rfind('}').map(|i| i + 1).unwrap_or(texte.len());
    let fragment = &texte[debut..fin];

    // Tentative directe, puis descente dans le premier objet imbriqué si ça échoue.
    let mut brut: ReponseLlm = match serde_json::from_str::<ReponseLlm>(fragment) {
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
    // Normalisation : si le LLM retourne 0-1 au lieu de 0-10, ramener à l'échelle 0-10
    brut.score_confiance = crate::utils::normaliser_score_llm(brut.score_confiance);

    if brut.signal != "STRADDLE" || brut.score_confiance < 5.5 {
        return HttpResponse::Ok().json(serde_json::json!({
            "signal": "WAIT",
            "raison": brut.raison,
            "score_confiance": brut.score_confiance
        }));
    }

    // Calcul SL/TP (3 niveaux par jambe) — ratios calibrés par catégorie si disponibles
    let atr = body.atr_actuel;
    let prix = body.prix_actuel;

    // Catégoriser le contexte pour charger les ratios calibrés
    let creneaux_valides_complets: Vec<_> = creneaux
        .iter()
        .filter(|c| c.statut == "valide" || c.statut == "a_tester")
        .cloned()
        .collect();
    let categorie_ctx = crate::straddle_categorisation::categoriser(
        &annonces,
        now,
        &creneaux_valides_complets,
        &asset_str,
    );
    let seuils = db::straddle_calibration::charger_seuils(
        state.db.pool(),
        &asset_str,
        categorie_ctx.categorie.as_str(),
    )
    .await;

    let sl = seuils.sl_ratio;
    let r1 = seuils.tp1_ratio;
    let r2 = seuils.tp2_ratio;
    let r3 = r2 + (r2 - r1).max(0.5); // TP3 = TP2 + écart TP2−TP1

    let sl_long = prix - sl * atr;
    let sl_short = prix + sl * atr;
    let tp1_long = prix + r1 * atr;
    let tp1_short = prix - r1 * atr;
    let tp2_long = prix + r2 * atr;
    let tp2_short = prix - r2 * atr;
    let tp3_long = prix + r3 * atr;
    let tp3_short = prix - r3 * atr;

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

    // Heure d'entrée : LLM en priorité, sinon première annonce HIGH impact
    let heure_entree: Option<i64> = brut
        .heure_entree_utc
        .as_deref()
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.timestamp())
        .or_else(|| {
            annonces
                .first()
                .and_then(|a| a["date_heure"].as_str())
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.timestamp())
        });

    let _ = state
        .db
        .inserer_signal_straddle_complet(&signal, sl_short, &[tp1_short, tp2_short, tp3_short], heure_entree)
        .await;

    // Snapshot features ML
    let ratio_atr_calc = if body.atr_moyen_14 > 0.0 { body.atr_actuel / body.atr_moyen_14 } else { 1.0 };
    crate::straddle_utils::sauvegarder_snapshot_ml(
        state.db.pool(), &state.db, &signal_id, &asset_str, &tf,
        ratio_atr_calc, categorie_ctx.categorie.as_str(), brut.score_confiance, now,
    ).await;

    crate::straddle_utils::reponse_signal_straddle(
        &brut, &signal_id, heure_entree, prix,
        sl_long, sl_short, tp1_long, tp1_short, tp2_long, tp2_short, tp3_long, tp3_short,
        &modele,
    )
}
