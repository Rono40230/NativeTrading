use actix_web::{web, HttpResponse, Responder};
use serde::Deserialize;
use std::collections::HashMap;

// Couche de données des prompts (defaults / overrides / prompt_effectif) extraite
// vers le crate `llm` (phase 1.6b) — découple le cycle anthropic → prompts_handler.
use llm::{charger_overrides, defaults, sauvegarder_overrides};

fn entree(
    id: &str,
    label: &str,
    description: &str,
    usage: &str,
    defs: &HashMap<&str, &str>,
    ovs: &HashMap<String, String>,
) -> serde_json::Value {
    let (contenu, modifie) = if let Some(ov) = ovs.get(id) {
        (ov.clone(), true)
    } else {
        (defs.get(id).copied().unwrap_or("").to_string(), false)
    };
    serde_json::json!({
        "id": id, "label": label, "description": description,
        "usage": usage, "contenu": contenu, "modifie": modifie
    })
}

/// GET /api/prompts — retourne tous les prompts (overrides ou défauts) avec flag `modifie`.
pub async fn lister_prompts() -> impl Responder {
    let d = defaults();
    let o = charger_overrides();
    let p = |id, label, desc, usage| entree(id, label, desc, usage, &d, &o);

    HttpResponse::Ok().json(serde_json::json!({
        "_note_definition": "Le prompt de Définition est DÉRIVÉ de la page Définition de la stratégie (source unique) — squelette étape 2, contenu complet à l'étape 3.",
        "rockets": {
            "definition": p("rockets_definition", "Définition (dérivée)", "Ancre le LLM dans la logique VCP Minervini. Dérivé de la page Définition.", "Toute analyse rockets"),
            "filtre_temps_reel":  p("rockets_filtre",  "Filtre temps réel", "Valide ou rejette chaque signal candidat avant sauvegarde. Retourne conviction 0–100 + raison.", "Avant chaque signal"),
            "analyse_strategique":p("rockets_analyse", "Analyse stratégique", "Analyse les signaux clôturés pour évaluer la performance et recommander des ajustements.", "Sur demande (≥ 5 trades clôturés)")
        },
        "smc": {
            "definition": p("smc_definition", "Définition (dérivée)", "Ancre le LLM dans la logique SMC v12 (Pine figé). Dérivé de la page Définition.", "Toute analyse SMC"),
            "filtre_temps_reel":  p("smc_filtre",  "Filtre temps réel", "Valide ou rejette chaque signal SMC Directionnel candidat.", "Avant chaque signal SMC"),
            "signal_json":        p("smc_signal",  "Génération signal JSON", "Génère un signal structuré JSON complet avec direction, SL, TP, confluences.", "POST /api/ia/signal"),
            "analyse_strategique":p("smc_analyse", "Analyse stratégique", "Analyse la performance globale des trades SMC clôturés.", "Sur demande")
        },
        "straddle": {
            "definition": p("straddle_definition", "Définition (dérivée)", "Ancre le LLM dans la logique news-trading. Dérivé de la page Définition.", "Toute analyse straddle"),
            "signal_temps_reel":  p("straddle_signal",  "Génération signal temps réel", "Décide en temps réel d'entrer une position LONG + SHORT simultanée sur évènement de volatilité.", "Boucle de surveillance Straddle"),
            "analyse_strategique":p("straddle_analyse", "Analyse stratégique", "Analyse les backtests Straddle et recommande des ajustements de créneaux/paramètres.", "Sur demande")
        },
        "outils_ia": {
            "coach":         p("coach",         "Coach SMC", "System prompt du Coach IA — définit la personnalité, les règles de réponse et la génération de diagrammes HTML.", "Outils IA → Coach IA (toutes les conversations)"),
            "vision_1tf":    p("vision_1tf",    "Analyse graphique — 1 TF", "Analyse ICT/SMC d'un graphique unique en 5 étapes (biais, liquidité, POI, scoring, signal).", "Outils IA → Analyse graphique (1 screenshot)"),
            "vision_multi_tf":p("vision_multi_tf","Analyse graphique — Multi-TF", "Analyse top-down HTF→ITF→LTF sur plusieurs screenshots du même actif.", "Outils IA → Analyse graphique (2+ screenshots)")
        }
    }))
}

#[derive(Deserialize)]
pub struct CorpsModification {
    pub contenu: String,
}

/// PUT /api/prompts/{id} — sauvegarde un override pour le prompt spécifié.
pub async fn modifier_prompt(
    id: web::Path<String>,
    corps: web::Json<CorpsModification>,
) -> impl Responder {
    let id = id.into_inner();
    if !defaults().contains_key(id.as_str()) {
        return HttpResponse::NotFound().json(serde_json::json!({ "error": "Prompt inconnu" }));
    }
    let mut ovs = charger_overrides();
    ovs.insert(id, corps.into_inner().contenu);
    match sauvegarder_overrides(&ovs) {
        Ok(_) => HttpResponse::Ok().json(serde_json::json!({ "ok": true })),
        Err(e) => {
            HttpResponse::InternalServerError().json(serde_json::json!({ "error": format!("{e}") }))
        }
    }
}

/// DELETE /api/prompts/{id} — restaure le prompt par défaut (supprime l'override).
pub async fn restaurer_prompt(id: web::Path<String>) -> impl Responder {
    let mut ovs = charger_overrides();
    ovs.remove(id.as_str());
    match sauvegarder_overrides(&ovs) {
        Ok(_) => HttpResponse::Ok().json(serde_json::json!({ "ok": true })),
        Err(e) => {
            HttpResponse::InternalServerError().json(serde_json::json!({ "error": format!("{e}") }))
        }
    }
}
