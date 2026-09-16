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
    /// R-DISTANCE (meilleur palier atteint) — donnée d'étude (laboratoire,
    /// analyse des zones).
    pub r_distance: f64,
    /// R ENCAISSÉ du trade (gagnants − perdants) : SMC = pondéré ventes
    /// partielles (LE R qui compose le profit $), autres = R net réalisé.
    /// Convention d'affichage officielle (décision propriétaire 16/09).
    pub r_pondere: f64,
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
    // 15/09 soir : le VÉCU est l'unique source (décision propriétaire) —
    // simuler() porte déjà les conventions officielles ($ pondéré SMC,
    // r_distance). Les rejeus ne sont plus servis.
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
pub async fn simuler(db: &db::Database, id_strategie: &str) -> anyhow::Result<SimulationCapital> {
    let capital_depart = db
        .lire_strategie(id_strategie)
        .await?
        .map(|r| r.capital)
        .unwrap_or(0.0);
    let fraction = fraction_risque(db, id_strategie).await;
    let clotures = db.clotures_pour_capital(id_strategie).await?;

    let mut capital = capital_depart;
    let mut points = Vec::with_capacity(clotures.len());
    for t in clotures {
        // Le risque en $ se calcule sur le capital au moment du trade —
        // c'est la définition même de la composition.
        // SMC : le vécu stocke le PALIER (distance) — le capital doit
        // composer le R PONDÉRÉ (ventes partielles 0,5/0,3/0,2), la
        // convention $ réels du projet. Les autres stratégies n'ont pas de
        // ventes partielles : leur r réalisé compose tel quel.
        let tps: Vec<f64> = serde_json::from_str(&t.take_profit).unwrap_or_default();
        let r_capital = if id_strategie == "SMC" {
            let risque = (t.prix_entree - t.stop_loss).abs();
            let (r_tp1, r_tp2) = if risque > 0.0 {
                (
                    (tps.first().copied().unwrap_or(t.prix_entree) - t.prix_entree).abs() / risque,
                    (tps.get(1).copied().unwrap_or(t.prix_entree) - t.prix_entree).abs() / risque,
                )
            } else {
                (0.0, 0.0)
            };
            // Correctif 15/09 nuit : le solde d'un TP2+BE sort au prix VÉCU
            // (stop suiveur post-TP2 à TP1 — la base montre aussi des sorties
            // à l'entrée), pas à 0.
            // Garde : un prix ≤ 0 n'est pas un prix (NULL/défaut mal lu) —
            // repli sur la mécanique (stop suiveur post-TP2 = TP1).
            let r_solde_tp2 = if t.verdict.to_lowercase().starts_with("tp2") && risque > 0.0 {
                t.prix_verdict
                    .filter(|pv| *pv > 0.0)
                    .map(|pv| (pv - t.prix_entree).abs() / risque)
            } else {
                None
            };
            crate::smc_pondere::r_pondere(
                &t.verdict, t.r, r_tp1, r_tp2,
                crate::smc_pondere::Fractions::default(),
                r_solde_tp2,
            )
        } else {
            t.r
        };
        let profit = r_capital * capital * fraction;
        capital += profit;
        let r_distance = db::signaux_palier::r_reference_palier(
            &t.verdict, &id_strategie, t.prix_entree, t.stop_loss, &tps,
        )
        .unwrap_or(t.r);
        points.push(PointCapital {
            id: t.id,
            ferme_le: t.ferme_le,
            r: t.r,
            profit,
            r_pondere: r_capital,
            capital_apres: capital,
            asset: t.asset,
            tf: t.tf,
            verdict: t.verdict,
            r_distance,
        });
    }
    Ok(SimulationCapital {
        capital_depart,
        fraction_risque: fraction,
        capital_actuel: capital,
        points,
    })
}
