//! Balayages du laboratoire — SMC (fractions + trailing) et straddle (k par
//! mode). Extraits de simulation.rs (limite 600 lignes).

use crate::simulation::{valider_id, RequeteBalayageSmc};
use crate::state::AppState;
use actix_web::{web, HttpResponse};
use serde::Deserialize;

/// Une configuration de fractions du balayage.
#[derive(serde::Serialize)]
struct LigneBalayage {
    f1: f64,
    f2: f64,
    f3: f64,
    capital: f64,
    rendement: f64,
    capital_minimum: f64,
    r_total_pondere: f64,
    actuel: bool,
}

/// POST /api/strategies/{id}/simulation/balayage — grille des fractions
/// (pas 0,05, f1+f2+f3=1) sur les CLÔTURES VÉCUES : les verdicts ne changent
/// pas, seule la découpe du lot varie. Instantané. Le pondéré utilise le
/// prix vécu du solde TP2+BE (correctif 15/09 nuit) — miroir de
/// capital_simule, garanti au centime par test.
pub async fn post_balayage_dispatch(
    state: web::Data<AppState>,
    path: web::Path<String>,
    body: web::Json<serde_json::Value>,
) -> impl actix_web::Responder {
    let id = path.into_inner();
    if !valider_id(&id) {
        return HttpResponse::NotFound().json(serde_json::json!({ "error": "Stratégie inconnue" }));
    }
    if id == "straddle" {
        let requete: RequeteBalayageStraddle =
            match serde_json::from_value(body.into_inner()) {
                Ok(r) => r,
                Err(_) => RequeteBalayageStraddle { mode: None, fenetre: None, decay: None, time_stop_min: None, assets: None },
            };
        return balayage_straddle(state, web::Json(requete)).await;
    }
    if id != "SMC" {
        return HttpResponse::NotImplemented().json(serde_json::json!({
            "error": "Balayage pas encore branché pour cette stratégie (phases 2-3)"
        }));
    }
    let requete = serde_json::from_value::<RequeteBalayageSmc>(body.into_inner())
        .unwrap_or(RequeteBalayageSmc { cible: None, assets: None, tfs: None });
    let cible = requete.cible.unwrap_or_else(|| "fractions".into());
    if cible == "trailing" {
        return balayage_trailing_smc(state, requete.assets.unwrap_or_default(), requete.tfs.unwrap_or_default()).await;
    }
    let db = state.db.clone();
    let Ok(clotures) = db.clotures_pour_capital("SMC").await else {
        return HttpResponse::InternalServerError()
            .json(serde_json::json!({ "error": "Clôtures illisibles" }));
    };
    let reg = db.lire_strategie("SMC").await.ok().flatten().unwrap_or_default();
    let cap0 = reg.capital;
    let risque = reg.risque_pct / 100.0;
    let actuelles = crate::reglages_smc::lire_fractions(&db).await;

    let mut lignes: Vec<LigneBalayage> = Vec::new();
    let mut f1 = 0.0_f64;
    while f1 <= 1.0001 {
        let mut f2 = 0.0_f64;
        while f1 + f2 <= 1.0001 {
            let f3 = 1.0 - f1 - f2;
            let fr = crate::smc_pondere::Fractions { tp1: f1, tp2: f2, tp3: f3 };
            let (capital, mini, somme) = composer(&clotures, cap0, risque, fr);
            lignes.push(LigneBalayage {
                f1: (f1 * 100.0).round() / 100.0,
                f2: (f2 * 100.0).round() / 100.0,
                f3: (f3 * 100.0).round() / 100.0,
                capital,
                rendement: if cap0 > 0.0 { capital / cap0 - 1.0 } else { 0.0 },
                capital_minimum: mini,
                r_total_pondere: somme,
                actuel: (f1 - actuelles.tp1).abs() < 1e-9
                    && (f2 - actuelles.tp2).abs() < 1e-9
                    && (f3 - actuelles.tp3).abs() < 1e-9,
            });
            f2 = (f2 * 100.0).round() / 100.0 + 0.05;
        }
        f1 = (f1 * 100.0).round() / 100.0 + 0.05;
    }
    lignes.sort_by(|a, b| b.capital.total_cmp(&a.capital));
    HttpResponse::Ok().json(serde_json::json!({
        "capital_depart": cap0,
        "nb_clotures": clotures.len(),
        "configurations": lignes,
    }))
}

/// Compose le capital des clôtures vécues avec des fractions données —
/// EXACTEMENT la formule de capital_simule (r_solde_tp2 vécu inclus).
pub(crate) fn composer(
    clotures: &[db::signaux_capital::ClotureCapital],
    cap0: f64,
    risque: f64,
    fr: crate::smc_pondere::Fractions,
) -> (f64, f64, f64) {
    let mut capital = cap0;
    let mut mini = cap0;
    let mut somme = 0.0_f64;
    for t in clotures {
        let tps: Vec<f64> = serde_json::from_str(&t.take_profit).unwrap_or_default();
        let risque_trade = (t.prix_entree - t.stop_loss).abs();
        let (r_tp1, r_tp2) = if risque_trade > 0.0 {
            (
                (tps.first().copied().unwrap_or(t.prix_entree) - t.prix_entree).abs() / risque_trade,
                (tps.get(1).copied().unwrap_or(t.prix_entree) - t.prix_entree).abs() / risque_trade,
            )
        } else {
            (0.0, 0.0)
        };
        // Garde : prix ≤ 0 = pas un prix (défaut) — repli mécanique TP1.
        let r_solde_tp2 = if t.verdict.to_lowercase().starts_with("tp2") && risque_trade > 0.0 {
            t.prix_verdict
                .filter(|pv| *pv > 0.0)
                .map(|pv| (pv - t.prix_entree).abs() / risque_trade)
        } else {
            None
        };
        let rp = crate::smc_pondere::r_pondere(
            &t.verdict, t.r, r_tp1, r_tp2, fr, r_solde_tp2,
        );
        capital += rp * capital * risque;
        mini = mini.min(capital);
        somme += rp;
    }
    (capital, mini, somme)
}

/// POST /api/strategies/straddle/simulation/balayage — balaye le k du
/// trailing (pas 0.2) pour un mode donné, sur les passes vécues. La ligne
/// « moteur » = réglage actuel de production (mode statique, k réel).
#[derive(Deserialize)]
pub struct RequeteBalayageStraddle {
    pub mode: Option<String>,
    pub fenetre: Option<usize>,
    pub decay: Option<f64>,
    pub time_stop_min: Option<i64>,
    /// Périmètre d'étude (laboratoire).
    pub assets: Option<Vec<String>>,
}

/// Balayage du k de TRAILING SMC (17/09, extension laboratoire) : le levier
/// exact du moteur (k×R après TP2, réglage smc_tp3_trailing). Chaque k = un
/// re-jeu complet (~20 s, cache LRU) — 5 configurations ≈ 100 s. Référence :
/// moteur actuel (trailing du réglage réel, inactif en production).
/// NB : un trailing ATR-roulant SMC = déviation de l'étalon Pine — hors
/// périmètre ici, à voter séparément le cas échéant.
#[allow(clippy::too_many_arguments)]
async fn balayage_trailing_smc(
    state: web::Data<AppState>,
    filtre_assets: Vec<String>,
    filtre_tfs: Vec<String>,
) -> HttpResponse {
    let db = state.db.clone();
    let reg = db.lire_strategie("SMC").await.ok().flatten().unwrap_or_default();
    let cap0 = reg.capital;
    let risque = reg.risque_pct / 100.0;
    let tp1 = crate::reglages_smc::lire_tp1_reglage(&db).await;
    let tp2_reg = crate::reglages_smc::lire_tp2_reglage(&db).await;
    let fractions = crate::reglages_smc::lire_fractions(&db).await;
    let tp3_l = crate::smc_rejeu::lire_tp3_lointaine_pub(&db).await;
    let tp3_r = crate::smc_rejeu::lire_tp3_rfixe_pub(&db).await;

    let mut lignes: Vec<serde_json::Value> = Vec::new();
    let mut k = 0.2_f64;
    while k <= 1.0001 {
        match crate::smc_rejeu::calculer_etude(&db, tp1, tp2_reg, tp3_l, tp3_r, Some(k), fractions, &filtre_assets, &filtre_tfs).await {
            Ok(r) => {
                let mini = r.clotures.iter().map(|c| c.capital_apres).fold(r.capital_depart, f64::min);
                lignes.push(serde_json::json!({
                    "k": (k * 10.0).round() / 10.0,
                    "capital": r.capital_actuel,
                    "rendement": if cap0 > 0.0 { r.capital_actuel / cap0 - 1.0 } else { 0.0 },
                    "capital_minimum": mini,
                    "r_total_pondere": r.r_total_pondere,
                    "clotures": r.total,
                }));
            }
            Err(_) => {}
        }
        k = (k * 10.0).round() / 10.0 + 0.2;
    }
    lignes.sort_by(|a, b| b["capital"].as_f64().unwrap_or(0.0).total_cmp(&a["capital"].as_f64().unwrap_or(0.0)));
    // Référence : réglage réel actuel (trailing inactif en production).
    let mut moteur = None;
    if let Ok(r) = crate::smc_rejeu::calculer_etude(&db, tp1, tp2_reg, tp3_l, tp3_r, None, fractions, &filtre_assets, &filtre_tfs).await {
        moteur = Some(serde_json::json!({
            "k": 0.0,
            "capital": r.capital_actuel,
            "rendement": if cap0 > 0.0 { r.capital_actuel / cap0 - 1.0 } else { 0.0 },
            "capital_minimum": r.clotures.iter().map(|c| c.capital_apres).fold(r.capital_depart, f64::min),
            "r_total_pondere": r.r_total_pondere,
            "clotures": r.total,
        }));
    }
    HttpResponse::Ok().json(serde_json::json!({
        "cible": "trailing",
        "capital_depart": cap0,
        "risque_pct": reg.risque_pct,
        "configurations": lignes,
        "moteur_actuel": moteur,
    }))
}

async fn balayage_straddle(
    state: web::Data<AppState>,
    body: web::Json<RequeteBalayageStraddle>,
) -> HttpResponse {
    let filtre_assets = body.assets.clone().unwrap_or_default();
    let db = state.db.clone();
    let b = body.into_inner();
    let mode = match b.mode.as_deref() {
        Some("roulant") | Some("roulant_tp1") | Some("decay") => b.mode.unwrap_or_default(),
        _ => "statique".to_string(),
    };
    let reg = db.lire_strategie("straddle").await.ok().flatten().unwrap_or_default();
    let cap0 = reg.capital;
    let risque = reg.risque_pct / 100.0;
    let params_db = db::strategies_params::lire_straddle_params(db.pool()).await;

    let mut lignes: Vec<serde_json::Value> = Vec::new();
    let mut k = 0.3_f64;
    while k <= 2.0001 {
        let cfg = crate::straddle_rejeu::ParamsTrailing {
            mode: mode.clone(),
            k,
            fenetre: b.fenetre.unwrap_or(10).clamp(3, 60),
            decay: b.decay.unwrap_or(0.0).clamp(0.0, 0.8),
            time_stop_min: b.time_stop_min.unwrap_or(60).clamp(5, 240),
        };
        match crate::straddle_rejeu::calculer_avec_filtres(&db, &cfg, &filtre_assets).await {
            Ok(r) => {
                let capital_minimum = r
                    .clotures
                    .iter()
                    .map(|c| c.capital_apres)
                    .fold(r.capital_depart, f64::min);
                lignes.push(serde_json::json!({
                    "k": (k * 10.0).round() / 10.0,
                    "capital": r.capital_actuel,
                    "rendement": if cap0 > 0.0 { r.capital_actuel / cap0 - 1.0 } else { 0.0 },
                    "capital_minimum": capital_minimum,
                    "r_total_net": r.r_total_net,
                    "passes": r.total,
                    "moteur": false,
                }));
            }
            Err(_) => {}
        }
        k = (k * 10.0).round() / 10.0 + 0.2;
    }
    lignes.sort_by(|a, b| b["capital"].as_f64().unwrap_or(0.0).total_cmp(&a["capital"].as_f64().unwrap_or(0.0)));
    // Référence moteur : réglage réel actuel.
    let cfg_moteur = crate::straddle_rejeu::ParamsTrailing {
        mode: "statique".into(),
        k: params_db.trailing_atr,
        fenetre: 10,
        decay: 0.0,
        time_stop_min: 60,
    };
    let mut moteur = None;
    if let Ok(r) = crate::straddle_rejeu::calculer_avec_filtres(&db, &cfg_moteur, &filtre_assets).await {
        moteur = Some(serde_json::json!({
            "k": params_db.trailing_atr,
            "capital": r.capital_actuel,
            "rendement": if cap0 > 0.0 { r.capital_actuel / cap0 - 1.0 } else { 0.0 },
            "capital_minimum": r.clotures.iter().map(|c| c.capital_apres).fold(r.capital_depart, f64::min),
            "r_total_net": r.r_total_net,
            "passes": r.total,
            "moteur": true,
        }));
    }
    HttpResponse::Ok().json(serde_json::json!({
        "mode": mode,
        "capital_depart": cap0,
        "configurations": lignes,
        "moteur_actuel": moteur,
    }))
}

/// GET /api/strategies/{id}/simulation/essais — bibliothèque (30 derniers).
pub async fn get_essais(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> impl actix_web::Responder {
    let id = path.into_inner();
    if !valider_id(&id) {
        return HttpResponse::NotFound().json(serde_json::json!({ "error": "Stratégie inconnue" }));
    }
    match state.db.lister_essais_simulation(&id, 30).await {
        Ok(essais) => {
            let liste: Vec<serde_json::Value> = essais
                .into_iter()
                .filter_map(|e| {
                    Some(serde_json::json!({
                        "id": e.id,
                        "params": serde_json::from_str::<serde_json::Value>(&e.params_json).ok()?,
                        "resultat": serde_json::from_str::<serde_json::Value>(&e.resultats_json).ok()?,
                        "cree_le": e.cree_le,
                    }))
                })
                .collect();
            HttpResponse::Ok().json(serde_json::json!({ "essais": liste }))
        }
        Err(e) => HttpResponse::InternalServerError()
            .json(serde_json::json!({ "error": e.to_string() })),
    }
}

/// DELETE /api/strategies/{id}/simulation/essais/{essai}.
pub async fn delete_essai(
    state: web::Data<AppState>,
    path: web::Path<(String, String)>,
) -> impl actix_web::Responder {
    let (id, essai) = path.into_inner();
    if !valider_id(&id) {
        return HttpResponse::NotFound().json(serde_json::json!({ "error": "Stratégie inconnue" }));
    }
    match state.db.supprimer_essai_simulation(&essai).await {
        Ok(()) => HttpResponse::Ok().json(serde_json::json!({ "supprime": essai })),
        Err(e) => HttpResponse::InternalServerError()
            .json(serde_json::json!({ "error": e.to_string() })),
    }
}

