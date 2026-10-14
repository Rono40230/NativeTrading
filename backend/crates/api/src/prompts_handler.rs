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
        "_note_definition": "Les prompts de Définition sont INJECTÉS en tête de chaque analyse stratégique (26/08 — injection réelle depuis le 13/10) avec les paramètres moteur en vigueur. Constitution : toute évolution de stratégie se reflète immédiatement ici.",
        // Organisation par RÔLE (owner 13/10) : ce qui ancre les analystes,
        // ce qui tourne seul, ce que le propriétaire déclenche.
        "definitions": {
            "smc":       p("smc_definition", "Définition SMC", "Doctrine pure de la stratégie SMC v12 — les valeurs en vigueur (trailing, TP, fractions, surcharges) sont injectées en direct dans le contexte de chaque analyse.", "Injectée en tête de toute analyse SMC"),
            "straddle":  p("straddle_definition", "Définition Straddle", "Doctrine pure : événements à l'horloge + annonces calendrier, armement par case de la matrice, lifecycle des deux jambes, surcharges par asset.", "Injectée en tête des analyses straddle (rapport + créneaux)"),
            "rockets":   p("rockets_definition", "Définition Rockets", "Les quatre piliers VCP × Rocket Hunter, verdicts /10, univers, gestion et véto unlocks.", "Injectée en tête des analyses rockets"),
        },
        "arriere_plan": {
            "presse_notation":  p("presse_notation","Notation presse (pertinence)","Note la pertinence 1-100 des titres pour le trading forex/métaux/crypto/indices — alimente les barres de score de la page Presse.","Chaque lot d'articles sans score"),
            "presse_sentiment": p("presse_sentiment","Sentiment presse","Un mot : haussier / neutre / baissier — l'impact probable d'un titre sur les prix.","Chaque article traduit"),
            "presse_traduction":p("presse_traduction","Traduction des titres","Traduction FR naturelle d'un titre financier (repli quand DeepL est absent).","Chaque titre (repli DeepL)"),
            "conviction_signal":  p("conviction_signal","Conviction à l'émission","Note chaque signal officiel a priori (0-100 + raison) — remplit la colonne IA des tableaux. Observation d'abord : aucun filtrage, re-test après vécu des réglages par asset (AUC 0,477 au 13/10).","À chaque signal émis, toutes stratégies"),
            "rockets_catalyseur": p("rockets_catalyseur","Catalyseur news (rockets)","Évalue les dépêches d'un candidat : pour ou contre une cassure haussière à 15 jours ? POUR/CONTRE/NEUTRE + conviction.","Après chaque scan quotidien rockets"),
            "rockets_ranker":     p("rockets_ranker","Ranker de pivots (rockets)","Départage les vraies cassures des fausses : conviction 0-100 + raison, sur le dossier complet du candidat.","Avant chaque signal rockets (seuil réglable)"),
            "unlock_detection":   p("unlock_detection","Détection unlocks (véto rockets)","Extrait des dépêches les déverrouillages de tokens DATÉS (symbole, date, USD) — alimente le véto éliminatoire du scanner (< N jours).","Quotidien, pour le véto rockets"),
        },
        "analystes": {
            "briefing_session":  p("briefing_session","Briefing du jour","Synthèse du jour : événements armés + annonces High + positions + presse — « ce qui compte aujourd'hui » en 3 lignes. Données 100 % locales.","Cockpit → badge 📋 Briefing (cache du jour)"),
            "autopsie_semaine":  p("autopsie_semaine","Autopsie de la semaine","Les clôtures des 7 derniers jours décrites factuellement : ce qui a distingué gagnants et perdants — descriptif, jamais prédictif.","Analyses → vue d'ensemble (cache du jour)"),
            "presse_brief":      p("presse_brief","Brief de presse","Le brief matinal en tête de la page Presse : contexte marché + articles marquants.","Page Presse (cache)"),
            "analyse_rapport":        p("analyse_rapport","Analyse des rapports d'activité", "L'analyste du Rapport d'activité : métriques $/R consolidées, pistes sobres à levier, droit de dire « rien à changer ».", "📊 Rapport d'activité → ⚡ Générer (cache du jour)"),
            "analyse_portefeuille":   p("analyse_portefeuille","Synthèse IA du portefeuille","Vue d'ensemble multi-stratégies : qui porte, qui coince, LA priorité de la semaine en tête des recommandations.","Vue d'ensemble analyses → ⚡ Générer (cache du jour)"),
            "analyse_rapport_asset": p("analyse_rapport_asset","Conseil par asset","Conseil ACTIONNABLE propre à UN asset : vécu R distance + $, réglages (surcharge ⊕ défaut), écart du labo — tranche : défaut / surcharge / désarmement.","Rapport d'activité → 🤖 par asset (cache du jour)"),
            "analyse_creneaux":       p("analyse_creneaux","Analyse de la matrice des créneaux (straddle)","Synthèse de la matrice asset × événement : événements prioritaires, meilleurs croisements, cases à désarmer — règle des 30 PAR CASE.","Straddle → analyse IA 🤖 de la matrice (cache du jour)"),
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
