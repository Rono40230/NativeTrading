//! Registre des stratégies (étape 2) : manifestes STRUCTURELS déclarés dans
//! le code (une stratégie = un crate) fusionnés avec l'état PILOTABLE de la
//! table `strategies`. L'UI (menus, blocs, cartes) se construit depuis ici.

use actix_web::{web, HttpResponse, Responder};
use serde::Serialize;

use crate::state::AppState;

/// Manifeste structurel d'une stratégie (source de vérité : le code).
#[derive(Debug, Clone)]
pub struct Manifeste {
    pub id: &'static str,
    pub nom: &'static str,
    pub description: &'static str,
    pub icone: &'static str,
    pub couleur: &'static str,
    /// Nom du moteur sur le bus (`SignalBrut.moteur`).
    pub moteur: &'static str,
    /// Document canonique (page Définition).
    pub canonique: &'static str,
    /// Catégories de prompts possédées.
    pub prompts: &'static [&'static str],
}

/// Les trois verticales actuelles. L'ajout d'une stratégie = un crate + une
/// entrée ici + l'enregistrement runtime.
pub const MANIFESTES: &[Manifeste] = &[
    Manifeste {
        id: "SMC",
        nom: "SMC",
        description: "Clone fidèle du Pine v12 : structure, zones institutionnelles, scoring et lifecycle.",
        icone: "📐",
        couleur: "#F44336",
        moteur: "smc_v12",
        canonique: "docs/reference/smc_indicateur_v12.pine",
        prompts: &["definition", "analyse", "filtre_temps_reel", "signal_json"],
    },
    Manifeste {
        id: "straddle",
        nom: "Straddle",
        description: "News trading : ordres stop autour du range pré-annonce, OCO, time-stop.",
        icone: "⚡",
        couleur: "#FF9800",
        moteur: "straddle",
        canonique: "roadmap Phase 3 (fiche canonique 2026-08-17)",
        prompts: &["definition", "analyse", "signal_temps_reel"],
    },
    Manifeste {
        id: "rockets",
        nom: "Rockets",
        description: "VCP Minervini : contractions décroissantes, pivot avec confirmation volume.",
        icone: "🚀",
        couleur: "#2196F3",
        moteur: "rockets",
        canonique: "roadmap Phase 3 (fiche VCP)",
        prompts: &["definition", "analyse", "filtre_temps_reel"],
    },
];

/// Stratégie complète servie à l'UI (manifeste + état pilotable).
#[derive(Serialize)]
pub struct StrategieComplete {
    pub id: String,
    pub nom: String,
    pub description: String,
    pub icone: String,
    pub couleur: String,
    pub moteur: String,
    pub canonique: String,
    pub prompts: Vec<String>,
    pub etat: String,
    pub notifications: bool,
    pub capital: f64,
    pub risque_pct: f64,
}

/// GET /api/strategies — le registre complet (menus, blocs, cartes).
pub async fn lister_strategies(state: web::Data<AppState>) -> impl Responder {
    match state.db.lire_strategies().await {
        Ok(registres) => {
            let liste: Vec<StrategieComplete> = MANIFESTES
                .iter()
                .map(|m| {
                    let r = registres.iter().find(|r| r.id == m.id);
                    StrategieComplete {
                        id: m.id.to_string(),
                        nom: m.nom.to_string(),
                        description: m.description.to_string(),
                        icone: m.icone.to_string(),
                        couleur: m.couleur.to_string(),
                        moteur: m.moteur.to_string(),
                        canonique: m.canonique.to_string(),
                        prompts: m.prompts.iter().map(|s| s.to_string()).collect(),
                        etat: r.map(|r| r.etat.clone()).unwrap_or_else(|| "Construction".into()),
                        notifications: r.map(|r| r.notifications).unwrap_or(false),
                        capital: r.map(|r| r.capital).unwrap_or(0.0),
                        risque_pct: r.map(|r| r.risque_pct).unwrap_or(1.0),
                    }
                })
                .collect();
            HttpResponse::Ok().json(liste)
        }
        Err(e) => HttpResponse::InternalServerError()
            .json(serde_json::json!({ "error": e.to_string() })),
    }
}

#[derive(serde::Deserialize)]
pub struct BodyMajStrategie {
    pub etat: Option<String>,
    pub notifications: Option<bool>,
    pub capital: Option<f64>,
    pub risque_pct: Option<f64>,
}

/// PUT /api/strategies/{id} — état, son Telegram, allocation.
pub async fn maj_strategie(
    state: web::Data<AppState>,
    path: web::Path<String>,
    body: web::Json<BodyMajStrategie>,
) -> impl Responder {
    let id = path.into_inner();
    if !MANIFESTES.iter().any(|m| m.id == id) {
        return HttpResponse::NotFound()
            .json(serde_json::json!({ "error": "Stratégie inconnue" }));
    }
    let Ok(actuel) = state.db.lire_strategie(&id).await else {
        return HttpResponse::InternalServerError()
            .json(serde_json::json!({ "error": "Registre illisible" }));
    };
    let defaut = db::strategies::StrategieRegistre {
        id: id.clone(),
        ..Default::default()
    };
    let a = actuel.unwrap_or(defaut);
    let etat = body.etat.clone().unwrap_or(a.etat);
    if !["Officielle", "Observation", "Construction"].contains(&etat.as_str()) {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({ "error": "État invalide (Officielle | Observation | Construction)" }));
    }
    let notifications = body.notifications.unwrap_or(a.notifications);
    let capital = body.capital.unwrap_or(a.capital).max(0.0);
    let risque_pct = body.risque_pct.unwrap_or(a.risque_pct).clamp(1.0, 3.0);
    match state.db.maj_strategie(&id, &etat, notifications, capital, risque_pct).await {
        Ok(()) => HttpResponse::Ok().json(serde_json::json!({
            "id": id, "etat": etat, "notifications": notifications,
            "capital": capital, "risque_pct": risque_pct,
        })),
        Err(e) => HttpResponse::InternalServerError()
            .json(serde_json::json!({ "error": e.to_string() })),
    }
}

/// Au boot : enregistre les manifestes absents du registre DB (INSERT OR
/// IGNORE — la DB reste maîtresse des réglages déjà saisis).
pub async fn amorcer_registre(db: &std::sync::Arc<db::Database>) {
    for m in MANIFESTES {
        let s = db::strategies::StrategieRegistre {
            id: m.id.to_string(),
            ..Default::default()
        };
        if let Err(e) = db.enregistrer_si_absente(&s).await {
            tracing::warn!("Registre stratégies ({}): {}", m.id, e);
        }
    }
}

/// GET /api/strategies/{id}/performance — courbe des trades clôturés (R
/// cumulé), stats et signaux en cours, pour le bloc central du dashboard.
pub async fn performance_strategie(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> impl Responder {
    let id = path.into_inner();
    if !MANIFESTES.iter().any(|m| m.id == id) {
        return HttpResponse::NotFound()
            .json(serde_json::json!({ "error": "Stratégie inconnue" }));
    }
    // SMC : métriques re-dérivées du TP1 réglé (re-jeu paramétrique — choix
    // propriétaire). Cache chaud → servies ; sinon repli sur la base vécue
    // et lancement du calcul en tâche de fond.
    if id == "SMC" {
        if let Some(r) = crate::smc_rejeu::lire_cache().await {
            let mut cumul = 0.0;
            let clotures: Vec<serde_json::Value> = r
                .clotures
                .iter()
                .map(|c| {
                    cumul += c.r_ref;
                    serde_json::json!({ "ferme_le": c.ferme_le, "r_cumule": cumul })
                })
                .collect();
            return HttpResponse::Ok().json(serde_json::json!({
                "clotures": clotures,
                "en_cours": [],
                "total": r.total,
                "gagnants": r.gagnants,
                "non_remplis": 0,
                "taux_reussite": r.taux_reussite,
                "r_total": r.r_total_pondere,
                "r_total_reference": r.r_total,
                "source": "rejeu",
                "tp1": r.tp1,
                "recalcul": crate::smc_rejeu::recalcul_en_cours(),
            }));
        }
        crate::smc_rejeu::lancer_si_necessaire(state.db.clone()).await;
    }
    match state.db.performance_strategie(&id).await {
        Ok(p) => {
            // Straddle/rockets : le badge doit montrer le R RÉALISÉ (celui que
            // le capital compose — décision 03/09, même harmonisation que SMC)
            // avec la référence en info-bulle via r_total_reference.
            let mut v = serde_json::to_value(&p).unwrap_or_default();
            if v.is_object() {
                let reference = v.get("r_total").cloned();
                if let Some(realise) = v.get("r_total_realise").cloned() {
                    v["r_total"] = realise;
                }
                if let Some(reference) = reference {
                    v["r_total_reference"] = reference;
                }
            }
            HttpResponse::Ok().json(v)
        }
        Err(e) => HttpResponse::InternalServerError()
            .json(serde_json::json!({ "error": e.to_string() })),
    }
}
