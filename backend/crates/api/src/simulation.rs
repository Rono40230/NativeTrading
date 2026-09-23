//! Laboratoire de simulation — page 🧪 Simulation (décision propriétaire
//! 15/09 nuit). Rejoue les clôtures avec des paramètres VIRTUELS : les
//! chiffres officiels (vécu) ne sont jamais touchés, le cache du re-jeu
//! officiel non plus. Phase 1 : SMC (les autres stratégies → 501 explicite).
//!
//! Endpoints :
//! - POST /api/strategies/{id}/simulation        → re-jeu complet (~35 s)
//! - POST /api/strategies/{id}/simulation/balayage → grille des fractions
//!   sur les clôtures vécues (instantané — verdicts inchangés)
//! - GET  /api/strategies/{id}/simulation/essais → bibliothèque (30 derniers)
//! - DELETE /api/strategies/{id}/simulation/essais/{essai}

use crate::state::AppState;
use actix_web::{web, HttpResponse};
use serde::Deserialize;

use std::sync::Arc;

/// Paramètres virtuels d'une simulation SMC — absents = réglages actuels.
#[derive(Deserialize)]
pub struct ParamsSimulationSmc {
    pub tp1_mult: Option<f64>,
    pub tp2_mult: Option<f64>,
    /// "lointaine" | "rfixe".
    pub tp3_mode: Option<String>,
    pub tp3_rfixe: Option<f64>,
    pub tp3_trailing: Option<bool>,
    pub tp3_trailing_r: Option<f64>,
    pub frac_tp1: Option<f64>,
    pub frac_tp2: Option<f64>,
    pub frac_tp3: Option<f64>,
    /// Straddle : trailing ATR × (défaut = réglage réel).
    pub trailing_atr: Option<f64>,
    /// Straddle : time-stop en minutes (défaut canonique 60).
    pub time_stop_min: Option<i64>,
    /// Straddle : mode de trailing étudié (statique | roulant | roulant_tp1 | decay).
    pub trailing_mode: Option<String>,
    /// Laboratoire : périmètre d'étude — assets simulés (vide/absent = tous).
    pub assets: Option<Vec<String>>,
    /// Laboratoire : périmètre d'étude — TF simulés (vide/absent = tous).
    pub tfs: Option<Vec<String>>,
    /// Straddle : fenêtre de l'ATR roulant (barres M1, modes dynamiques).
    pub atr_fenetre: Option<usize>,
    /// Straddle : réduction du k par 10 min (mode decay).
    pub k_decay: Option<f64>,
}

/// Résultat agrégé d'une simulation (et forme stockée dans la bibliothèque).
#[derive(serde::Serialize)]
struct ResultatSimulation {
    nb_trades: usize,
    taux_reussite: f64,
    /// Σ R-distance (qualité des zones).
    r_total: f64,
    /// Σ R pondéré (ce que le capital compose).
    r_total_pondere: f64,
    capital_depart: f64,
    capital_actuel: f64,
    /// Pire creux du capital composé — la vivabilité du réglage.
    capital_minimum: f64,
    verdicts: Vec<VerdictSim>,
    duree_ms: u128,
}

#[derive(serde::Serialize)]
struct VerdictSim {
    label: String,
    n: usize,
    /// Σ R pondéré du verdict.
    r: f64,
}

pub(crate) fn valider_id(id: &str) -> bool {
    crate::registre_strategies::MANIFESTES.iter().any(|m| m.id == id)
}

/// POST /api/strategies/{id}/simulation — re-jeu complet SMC avec paramètres
/// virtuels (bornes identiques aux réglages réels). Les autres stratégies
/// arrivent en phase 2/3.
pub async fn post_simulation(
    state: web::Data<AppState>,
    path: web::Path<String>,
    body: web::Json<ParamsSimulationSmc>,
) -> impl actix_web::Responder {
    let id = path.into_inner();
    if !valider_id(&id) {
        return HttpResponse::NotFound().json(serde_json::json!({ "error": "Stratégie inconnue" }));
    }
    if id == "straddle" {
        return simulation_straddle(state, body.into_inner()).await;
    }
    if id != "SMC" {
        return HttpResponse::NotImplemented().json(serde_json::json!({
            "error": "Simulation pas encore branchée pour cette stratégie (rockets/KDJ : effectif < 30 trades et backtesteur dédié à reconstruire)"
        }));
    }
    let db = state.db.clone();
    let b = body.into_inner();

    // Défauts = réglages actuels ; bornes = celles des réglages réels.
    let tp1 = b.tp1_mult.unwrap_or(smc_rejeu_config(&db, "smc_tp1_mult", 0.6, 0.2, 1.5).await);
    let tp2 = b.tp2_mult.unwrap_or(smc_rejeu_config(&db, "smc_tp2_mult", 2.0, 1.0, 4.0).await);
    let tp3_rfixe = b.tp3_rfixe.unwrap_or(smc_rejeu_config(&db, "smc_tp3_rfixe", 3.0, 3.0, 10.0).await);
    let tp3_lointaine = match b.tp3_mode.as_deref() {
        Some("rfixe") => false,
        Some("lointaine") => true,
        _ => {
            db.lire_config("smc_tp3_mode")
                .await
                .ok()
                .flatten()
                .map(|v| !v.trim().eq_ignore_ascii_case("rfixe"))
                .unwrap_or(true)
        }
    };
    let trailing_actif = match b.tp3_trailing {
        Some(v) => v,
        None => db
            .lire_config("smc_tp3_trailing")
            .await
            .ok()
            .flatten()
            .map(|v| v.trim() == "1")
            .unwrap_or(false),
    };
    let trailing_r = if trailing_actif {
        Some(b.tp3_trailing_r.unwrap_or(smc_rejeu_config(&db, "smc_tp3_trailing_r", 0.5, 0.05, 1.0).await))
    } else {
        None
    };
    let actuelles = crate::reglages_smc::lire_fractions(&db).await;
    let f1 = b.frac_tp1.unwrap_or(actuelles.tp1).clamp(0.0, 1.0);
    let f2 = b.frac_tp2.unwrap_or(actuelles.tp2).clamp(0.0, 1.0);
    let f3 = (b.frac_tp3.unwrap_or(actuelles.tp3)).clamp(0.0, 1.0);
    let somme = f1 + f2 + f3;
    let fractions = if somme > 1e-9 {
        crate::smc_pondere::Fractions { tp1: f1 / somme, tp2: f2 / somme, tp3: f3 / somme }
    } else {
        crate::smc_pondere::Fractions::default()
    };

    let debut = std::time::Instant::now();
    // calculer_etude : même moteur exact, avec le cache LRU du laboratoire
    // (simulation puis balayage partagent leurs re-jeus).
    let filtre_assets = b.assets.clone().unwrap_or_default();
    let filtre_tfs = b.tfs.clone().unwrap_or_default();
    let rejeu = match crate::smc_rejeu::calculer_etude(
        &db, tp1, tp2, tp3_lointaine, tp3_rfixe, trailing_r, fractions,
        &filtre_assets, &filtre_tfs,
    )
    .await
    {
        Ok(r) => r,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": format!("Re-jeu impossible : {e}") }));
        }
    };
    let duree_ms = debut.elapsed().as_millis();
    let rejeu = (*rejeu).clone();

    let mut par_verdict: std::collections::BTreeMap<String, (usize, f64)> =
        std::collections::BTreeMap::new();
    for c in &rejeu.clotures {
        let e = par_verdict.entry(c.verdict.clone()).or_insert((0, 0.0));
        e.0 += 1;
        e.1 += c.r_pondere;
    }
    let capital_minimum = rejeu
        .clotures
        .iter()
        .map(|c| c.capital_apres)
        .fold(rejeu.capital_depart, f64::min);
    let resultat = ResultatSimulation {
        nb_trades: rejeu.total,
        taux_reussite: rejeu.taux_reussite,
        r_total: rejeu.r_total,
        r_total_pondere: rejeu.r_total_pondere,
        capital_depart: rejeu.capital_depart,
        capital_actuel: rejeu.capital_actuel,
        capital_minimum,
        verdicts: par_verdict
            .into_iter()
            .map(|(label, (n, r))| VerdictSim { label, n, r })
            .collect(),
        duree_ms,
    };

    // Bibliothèque : l'essai est conservé (params + résultats).
    let params = serde_json::json!({
        "tp1_mult": tp1, "tp2_mult": tp2,
        "tp3_mode": if tp3_lointaine { "lointaine" } else { "rfixe" },
        "tp3_rfixe": tp3_rfixe,
        "tp3_trailing": trailing_r,
        "frac_tp1": fractions.tp1, "frac_tp2": fractions.tp2, "frac_tp3": fractions.tp3,
    });
    let id_essai = format!("essai-{}", chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0));
    let _ = db
        .enregistrer_essai_simulation(
            &id_essai,
            "SMC",
            &params.to_string(),
            &serde_json::to_string(&resultat).unwrap_or_default(),
            chrono::Utc::now().timestamp(),
        )
        .await;

    HttpResponse::Ok().json(serde_json::json!({
        "strategie": "SMC",
        "params": params,
        "resultat": resultat,
    }))
}

/// Lecture bornée d'un réglage numérique (défaut si absent).
async fn smc_rejeu_config(
    db: &Arc<db::Database>,
    cle: &str,
    defaut: f64,
    min: f64,
    max: f64,
) -> f64 {
    db.lire_config(cle)
        .await
        .ok()
        .flatten()
        .and_then(|v| v.trim().parse::<f64>().ok())
        .map(|v| v.clamp(min, max))
        .unwrap_or(defaut)
}

/// Simulation straddle : les DEUX paramètres que le re-jeu sait faire
/// varier — trailing ATR et time-stop. Les niveaux (ATR/TP mults) sont ceux
/// historiques des passes (chantier backtesteur pour les rejouer).
async fn simulation_straddle(
    state: web::Data<AppState>,
    b: ParamsSimulationSmc,
) -> HttpResponse {
    let db = state.db.clone();
    let params_db = db::strategies_params::lire_straddle_params(db.pool()).await;
    let mode = b.trailing_mode.unwrap_or_else(|| "statique".into());
    let mode = match mode.as_str() {
        "roulant" | "roulant_tp1" | "decay" => mode,
        _ => "statique".to_string(),
    };
    let filtre_assets = b.assets.clone().unwrap_or_default();
    let cfg = crate::straddle_rejeu::ParamsTrailing {
        mode: mode.clone(),
        k: b.trailing_atr.unwrap_or(params_db.trailing_atr).clamp(0.1, 5.0),
        fenetre: b.atr_fenetre.unwrap_or(10).clamp(3, 60),
        decay: b.k_decay.unwrap_or(0.0).clamp(0.0, 0.8),
        time_stop_min: b.time_stop_min.unwrap_or(60).clamp(5, 240),
    };

    let debut = std::time::Instant::now();
    let rejeu = match crate::straddle_rejeu::calculer_avec_filtres(&db, &cfg, &filtre_assets).await {
        Ok(r) => r,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": format!("Re-jeu impossible : {e}") }));
        }
    };
    let duree_ms = debut.elapsed().as_millis();

    let mut par_verdict: std::collections::BTreeMap<String, (usize, f64)> =
        std::collections::BTreeMap::new();
    for c in &rejeu.clotures {
        let e = par_verdict.entry(c.verdict.clone()).or_insert((0, 0.0));
        e.0 += 1;
        e.1 += c.r_net;
    }
    let capital_minimum = rejeu
        .clotures
        .iter()
        .map(|c| c.capital_apres)
        .fold(rejeu.capital_depart, f64::min);
    let resultat = ResultatSimulation {
        nb_trades: rejeu.total,
        taux_reussite: rejeu.taux_reussite,
        r_total: rejeu.r_total,
        r_total_pondere: rejeu.r_total_net,
        capital_depart: rejeu.capital_depart,
        capital_actuel: rejeu.capital_actuel,
        capital_minimum,
        verdicts: par_verdict
            .into_iter()
            .map(|(label, (n, r))| VerdictSim { label, n, r })
            .collect(),
        duree_ms,
    };
    let params = serde_json::json!({
        "trailing_mode": mode,
        "trailing_atr": cfg.k, "time_stop_min": cfg.time_stop_min,
        "atr_fenetre": cfg.fenetre, "k_decay": cfg.decay,
    });
    let id_essai = format!("essai-{}", chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0));
    let _ = db
        .enregistrer_essai_simulation(
            &id_essai,
            "straddle",
            &params.to_string(),
            &serde_json::to_string(&resultat).unwrap_or_default(),
            chrono::Utc::now().timestamp(),
        )
        .await;
    HttpResponse::Ok().json(serde_json::json!({
        "strategie": "straddle",
        "params": params,
        "resultat": resultat,
    }))
}

/// Cible du balayage SMC : fractions (par défaut) ou k du trailing.
#[derive(Deserialize)]
pub struct RequeteBalayageSmc {
    /// "fractions" (défaut) | "trailing"
    pub cible: Option<String>,
    /// Périmètre d'étude (laboratoire).
    pub assets: Option<Vec<String>>,
    pub tfs: Option<Vec<String>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simulation_balayages::composer;

    /// LE verrou du laboratoire : le balayage, sur les fractions ACTUELLES,
    /// doit redonner le capital officiel du vécu (même formule que
    /// capital_simule, correctif solde TP2+BE inclus).
    #[actix_web::test]
    async fn balayage_config_actuelle_egale_capital_vecu() {
        let db = db::Database::new(":memory:").await.expect("DB mémoire");
        db.run_migrations().await.expect("migrations OK");
        sqlx::query("UPDATE strategies SET capital = 1000.0, risque_pct = 1.0 WHERE id = 'SMC'")
            .execute(db.pool())
            .await
            .expect("stratégie SMC");
        // 3 clôtures : TP2+BE (solde vécu à TP1), SL, TP1+BE.
        for (id, verdict, r, pv) in [
            ("s1", "TP2+BE", 2.0, Some(2006.0)),
            ("s2", "SL", -1.0, Some(1990.0)),
            ("s3", "TP1+BE", 0.6, Some(2000.0)),
        ] {
            sqlx::query(
                "INSERT INTO signaux (id, asset, timeframe, direction, score, prix_entree,
                                      stop_loss, take_profit, strategie, statut, verdict,
                                      r_realise, cree_le, heure_entree, ferme_le, prix_verdict)
                 VALUES (?, 'BTC', 'M5', 'Long', 10, 2000.0, 1990.0, '[2006, 2020, 2030]',
                         'SMC', 'Fermé', ?, ?, 1700000000, 1700000600, 1700001200, ?)",
            )
            .bind(id)
            .bind(verdict)
            .bind(r)
            .bind(pv)
            .execute(db.pool())
            .await
            .expect("clôture");
        }
        let clotures = db.clotures_pour_capital("SMC").await.expect("clôtures");
        let fr = crate::smc_pondere::Fractions { tp1: 0.5, tp2: 0.3, tp3: 0.2 };
        let (capital, mini, somme) = composer(&clotures, 1000.0, 0.01, fr);

        // Référence : simuler() officiel (mêmes clôtures, mêmes fractions).
        let sim = crate::capital_simule::simuler(&db, "SMC").await.expect("simulation");
        assert!(
            (capital - sim.capital_actuel).abs() < 1e-6,
            "balayage {capital} ≠ capital officiel {}",
            sim.capital_actuel
        );
        // Série gagnante : le creux reste le capital de départ (jamais sous).
        assert!((mini - 1000.0).abs() < 1e-9, "creux {mini}");
        // TP2+BE pondéré = 0,3 + 0,6 + 0,2×0,6 (solde vécu à TP1) = 1,02.
        assert!((somme - (1.02 - 1.0 + 0.3)).abs() < 1e-9, "Σ pondéré {somme}");
    }
}
