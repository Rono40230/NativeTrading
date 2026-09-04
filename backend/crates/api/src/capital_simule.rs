//! Simulation composée du capital par stratégie, en $.
//!
//! Le capital de départ (registre, propre à chaque stratégie) évolue à chaque
//! clôture : `capital += R_réalisé × capital × fraction_risque`. Chaque trade
//! suivant calcule donc son lot sur le capital mis à jour par les clôtures
//! précédentes. Sans état persistant : la série est recalculée depuis
//! l'historique des clôtures (quelques centaines de trades, millisecondes) —
//! pas de dérive possible, et changer le capital de départ re-simule toute la
//! courbe depuis la nouvelle base.
//!
//! Fraction de risque : SMC/straddle = `risque_pct` du registre ; rockets =
//! profil de risque du Journal de Trading — exactement la fraction qu'utilise
//! le calcul du lot à l'émission.

use crate::state::AppState;
use actix_web::{web, HttpResponse};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct PointCapital {
    pub id: String,
    pub ferme_le: i64,
    /// R réalisé du trade (sortie réelle).
    pub r: f64,
    /// Profit/perte simulé en $.
    pub profit: f64,
    /// Capital simulé après la clôture.
    pub capital_apres: f64,
    /// Asset / timeframe / verdict — camemberts $ et centre d'analyse.
    pub asset: String,
    pub tf: String,
    pub verdict: String,
}

#[derive(Debug, Serialize)]
pub struct SimulationCapital {
    /// Capital de départ saisi pour la stratégie (registre).
    pub capital_depart: f64,
    /// Fraction du capital risquée par trade (ex. 0.01).
    pub fraction_risque: f64,
    /// Capital simulé courant (après la dernière clôture).
    pub capital_actuel: f64,
    pub points: Vec<PointCapital>,
}

/// GET /api/strategies/{id}/capital — simulation composée du capital.
pub async fn capital_strategie(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> impl actix_web::Responder {
    let id = path.into_inner();
    if !crate::registre_strategies::MANIFESTES.iter().any(|m| m.id == id) {
        return HttpResponse::NotFound()
            .json(serde_json::json!({ "error": "Stratégie inconnue" }));
    }
    // SMC : capital re-dérivé du re-jeu TP1 (même base que la performance).
    if id == "SMC" {
        if let Some(r) = crate::smc_rejeu::lire_cache().await {
            let mut precedent = r.capital_depart;
            let points: Vec<serde_json::Value> = r
                .clotures
                .iter()
                .map(|c| {
                    let profit = c.capital_apres - precedent;
                    precedent = c.capital_apres;
                    serde_json::json!({
                        "id": format!("{}-{}", c.asset, c.tf),
                        "ferme_le": c.ferme_le,
                        "r": c.r,
                        "profit": profit,
                        "capital_apres": c.capital_apres,
                        "asset": c.asset,
                        "tf": c.tf,
                        "verdict": c.verdict,
                    })
                })
                .collect();
            return HttpResponse::Ok().json(serde_json::json!({
                "capital_depart": r.capital_depart,
                "fraction_risque": r.fraction_risque,
                "capital_actuel": r.capital_actuel,
                "points": points,
            }));
        }
    }
    match simuler(&state.db, &id).await {
        Ok(s) => HttpResponse::Ok().json(s),
        Err(e) => HttpResponse::InternalServerError()
            .json(serde_json::json!({ "error": e.to_string() })),
    }
}

/// Fraction de risque utilisée par le calcul du lot de la stratégie
/// (même source que l'émission — voir signaux_officiels::formater_message).
async fn fraction_risque(db: &db::Database, id: &str) -> f64 {
    if id == "rockets" {
        crate::rockets_verticale::lire_params(db).await.profil.fraction()
    } else {
        db.lire_strategie(id)
            .await
            .ok()
            .flatten()
            .map(|r| r.risque_pct / 100.0)
            .unwrap_or(0.01)
    }
}

/// Capital simulé courant — injecté dans le calcul du lot à l'émission :
/// chaque trade mise sur le capital mis à jour par les clôtures précédentes.
pub async fn capital_actuel(db: &db::Database, id: &str) -> Option<f64> {
    simuler(db, id).await.ok().map(|s| s.capital_actuel)
}

/// Rejoue les clôtures remplies dans l'ordre et compose le capital.
pub async fn simuler(db: &db::Database, id: &str) -> anyhow::Result<SimulationCapital> {
    let capital_depart = db
        .lire_strategie(id)
        .await?
        .map(|r| r.capital)
        .unwrap_or(0.0);
    let fraction = fraction_risque(db, id).await;
    let clotures = db.clotures_pour_capital(id).await?;

    let mut capital = capital_depart;
    let mut points = Vec::with_capacity(clotures.len());
    for t in clotures {
        // Le risque en $ se calcule sur le capital au moment du trade —
        // c'est la définition même de la composition.
        let profit = t.r * capital * fraction;
        capital += profit;
        points.push(PointCapital {
            id: t.id,
            ferme_le: t.ferme_le,
            r: t.r,
            profit,
            capital_apres: capital,
            asset: t.asset,
            tf: t.tf,
            verdict: t.verdict,
        });
    }
    Ok(SimulationCapital {
        capital_depart,
        fraction_risque: fraction,
        capital_actuel: capital,
        points,
    })
}
