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
        "_note_definition": "Les prompts de Définition sont INJECTÉS en tête de chaque analyse stratégique (26/08) — la promesse 'ancre le LLM' est désormais réelle. Constitution : toute évolution de stratégie se reflète immédiatement ici.",
        "rockets": {
            "definition": p("rockets_definition", "Définition (dérivée)", "Ancre le LLM dans la logique VCP Minervini. Dérivé de la page Définition.", "Toute analyse rockets"),
            "catalyseur_news":    p("rockets_catalyseur", "Catalyseur news", "Évalue les dépêches d'un candidat : pour ou contre une cassure haussière à 15 jours ? POUR/CONTRE/NEUTRE + conviction.", "Après chaque scan quotidien"),
            "ranker_pivots":      p("rockets_ranker", "Ranker de pivots", "Départage les vraies cassures des fausses : conviction 0–100 + raison, sur le dossier complet du candidat.", "Avant chaque signal (seuil réglable)"),
            "filtre_temps_reel":  p("rockets_filtre",  "Filtre temps réel", "Valide ou rejette un signal Rocket candidat (JSON conviction).", "DORMANT — remplacé par le ranker de pivots"),
            "analyse_strategique":p("rockets_analyse", "Analyse stratégique", "Analyse les signaux clôturés pour évaluer la performance et recommander des ajustements.", "Sur demande (≥ 5 trades clôturés)")
        },
        "smc": {
            "definition": p("smc_definition", "Définition", "Ancre l'analyste dans la logique SMC v12 — injectée en tête de chaque analyse.", "Toute analyse SMC (injectée automatiquement)"),
            "filtre_temps_reel":  p("smc_filtre",  "Filtre temps réel", "Valide ou rejette un signal SMC candidat (JSON conviction).", "DORMANT — retour possible après accumulation (roadmap §5)"),
            "analyse_strategique":p("smc_analyse", "Analyse stratégique", "Analyse la performance globale des trades SMC clôturés (moteur v12, verdicts TP/SL/Expire).", "Bouton 📊 Analyse (page Signaux SMC)")
        },
        "straddle": {
            "definition": p("straddle_definition", "Définition (dérivée)", "Ancre le LLM dans la logique news-trading. Dérivé de la page Définition.", "Toute analyse straddle"),
            "signal_temps_reel":  p("straddle_signal",  "Génération signal (héritage v1)", "Décide d'une passe sur évènement de volatilité. Mécanique actée intégrée (timer T-10 s, R × ATR H1).", "HÉRITAGE Gate 3 — le moteur v2 n'y passe plus"),
            "analyse_strategique":p("straddle_analyse", "Analyse stratégique", "Analyse les backtests Straddle et recommande des ajustements de créneaux/paramètres.", "Sur demande")
        },
        "outils_ia": {
            "coach":         p("coach",         "Coach SMC", "System prompt du Coach IA — définit la personnalité, les règles de réponse et la génération de diagrammes HTML.", "Outils IA → Coach IA (toutes les conversations)"),
            "vision_1tf":    p("vision_1tf",    "Analyse graphique — 1 TF", "Analyse ICT/SMC d'un graphique unique en 5 étapes (biais, liquidité, POI, scoring, signal).", "Outils IA → Analyse graphique (1 screenshot)"),
            "vision_multi_tf":p("vision_multi_tf","Analyse graphique — Multi-TF", "Analyse top-down HTF→ITF→LTF sur plusieurs screenshots du même actif.", "Outils IA → Analyse graphique (2+ screenshots)"),
            "analyse_rapport":p("analyse_rapport","Analyse des rapports d'activité", "Consigne de l'analyste du Rapport d'activité : lit les métriques $/R consolidées et répond en JSON structuré (état, points forts/faibles, pistes, confiance). L'effectif vs règle des 30 trades est injecté dynamiquement dans le contexte, pas ici.", "📊 Rapport d'activité → bouton Générer (cache du jour)")
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
