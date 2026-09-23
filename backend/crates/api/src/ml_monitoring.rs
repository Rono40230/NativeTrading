//! Monitoring ML par stratégie — MIROIR de l'historique des trades clôturés
//! (décision propriétaire 23/09 : « les cartes SMC et Straddle doivent être
//! en miroir avec les trades clôturés = c'est la DB de référence »).
//!
//! Source unique : `signaux` filtré exactement comme l'historique affiché
//! (statut Fermé + position remplie) ; le R par trade est le MÊME calcul que
//! `get_signaux` (r_encaisse, décision 16/09) : SMC = `r_pondere` (ventes
//! partielles, fractions réelles), autres stratégies = `r_realise` (R net
//! déjà encaissé à la clôture — rockets pondère à R1, straddle stocke le R
//! net de la passe).
//!
//! (Ancienne source `ml_training_samples` abandonnée pour l'AFFICHAGE :
//! elle porte le r_realise distance du 15/09 pour SMC et des filtres
//! différents — les samples restent la table d'entraînement ML.)

use actix_web::{web, HttpResponse, Responder};
use sqlx::Row;

use crate::smc_pondere::{r_pondere, Fractions};
use crate::state::AppState;

/// R encaissé d'un trade fermé — même logique que `get_signaux`
/// (signaux_handlers) : SMC pondéré, autres = r_realise. Paramètres alignés
/// champ à champ pour un miroir exact.
fn r_encaisse(
    smc: bool,
    verdict: &str,
    r_realise: f64,
    entree: f64,
    sl: f64,
    tps_texte: &str,
    prix_verdict: Option<f64>,
    f: Fractions,
) -> f64 {
    if !smc {
        return r_realise;
    }
    let risque = (entree - sl).abs();
    if risque <= 0.0 {
        return 0.0;
    }
    let tps: Vec<f64> = serde_json::from_str(tps_texte).unwrap_or_default();
    let r_tp1 = tps.first().map(|tp| (tp - entree).abs() / risque).unwrap_or(0.0);
    let r_tp2 = tps.get(1).map(|tp| (tp - entree).abs() / risque).unwrap_or(r_tp1);
    // Garde : prix ≤ 0 = défaut, pas un prix → None (repli TP2).
    let r_solde = prix_verdict
        .filter(|pv| *pv > 0.0)
        .map(|pv| (pv - entree).abs() / risque);
    r_pondere(verdict, r_realise, r_tp1, r_tp2, f, r_solde)
}

/// Statistiques d'une stratégie, miroir de son historique de trades
/// clôturés. `filtre` = LIKE sur strategie ; `smc` = pondération ventes
/// partielles avec les fractions réglées (config).
pub async fn stats_json(
    pool: &sqlx::SqlitePool,
    filtre: &str,
    smc: bool,
    fractions: &Fractions,
) -> serde_json::Value {
    let rows = sqlx::query(
        "SELECT verdict, r_realise, prix_entree, stop_loss, take_profit, prix_verdict
         FROM signaux
         WHERE LOWER(strategie) LIKE ?
           AND statut = 'Fermé'
           AND heure_entree IS NOT NULL
         ORDER BY ferme_le DESC",
    )
    .bind(filtre)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let trades: Vec<(String, f64)> = rows
        .iter()
        .map(|r| {
            let verdict: String = r.get("verdict");
            let r_realise: f64 = r.get::<Option<f64>, _>("r_realise").unwrap_or(0.0);
            let entree: f64 = r.get::<Option<f64>, _>("prix_entree").unwrap_or(0.0);
            let sl: f64 = r.get::<Option<f64>, _>("stop_loss").unwrap_or(0.0);
            let tps: String = r.get::<Option<String>, _>("take_profit").unwrap_or("[]".into());
            let pv: Option<f64> = r.get("prix_verdict");
            (
                verdict.clone(),
                r_encaisse(smc, &verdict, r_realise, entree, sl, &tps, pv, *fractions),
            )
        })
        .collect();

    let nb = trades.len() as i64;
    let wins = trades.iter().filter(|(_, r)| *r > 0.0).count() as i64;
    let somme_r: f64 = trades.iter().map(|(_, r)| *r).sum();
    let r_moyen = if nb > 0 { Some(somme_r / nb as f64) } else { None };

    // Par verdict (les catégories affichées = verdicts de l'historique).
    use std::collections::BTreeMap;
    let mut par_verdict: BTreeMap<String, (i64, i64, f64)> = BTreeMap::new();
    for (verdict, r) in &trades {
        let e = par_verdict.entry(verdict.clone()).or_insert((0, 0, 0.0));
        e.0 += 1;
        if *r > 0.0 {
            e.1 += 1;
        }
        e.2 += *r;
    }
    let par_categorie: Vec<serde_json::Value> = par_verdict
        .into_iter()
        .map(|(verdict, (n, w, somme))| {
            serde_json::json!({
                "categorie":   verdict,
                "nb_trades":   n,
                "win_rate":    if n > 0 { w as f64 / n as f64 } else { 0.0 },
                "pnl_r_moyen": if n > 0 { Some(somme / n as f64) } else { None },
            })
        })
        .collect();

    // Dérive : WR des 20 dernières clôtures < 45 % (ferme_le décroissant).
    let recents: Vec<&f64> = trades.iter().take(20).map(|(_, r)| r).collect();
    let derive_detectee = if recents.len() >= 10 {
        let w = recents.iter().filter(|r| ***r > 0.0).count() as f64;
        w / (recents.len() as f64) < 0.45
    } else {
        false
    };

    serde_json::json!({
        "nb_signals_total":      nb,
        "nb_feedbacks_clotures": nb,
        "nb_gagnants":           wins,
        "nb_perdants":           nb - wins,
        "nb_invalides":          0,
        "win_rate_global":       if nb > 0 { wins as f64 / nb as f64 } else { 0.0 },
        "pnl_moyen_r":           r_moyen,
        "somme_r":               somme_r,
        "par_categorie":         par_categorie,
        "derive_detectee":       derive_detectee,
        "derniere_maj":          chrono::Utc::now().timestamp(),
    })
}

pub async fn straddle(state: web::Data<AppState>) -> impl Responder {
    let f = crate::reglages_smc::lire_fractions(&state.db).await;
    HttpResponse::Ok().json(stats_json(state.db.pool(), "%straddle%", false, &f).await)
}

pub async fn rockets(state: web::Data<AppState>) -> impl Responder {
    let f = crate::reglages_smc::lire_fractions(&state.db).await;
    HttpResponse::Ok().json(stats_json(state.db.pool(), "%rockets%", false, &f).await)
}

pub async fn kdj(state: web::Data<AppState>) -> impl Responder {
    let f = crate::reglages_smc::lire_fractions(&state.db).await;
    HttpResponse::Ok().json(stats_json(state.db.pool(), "%kdj%", false, &f).await)
}

/// SMC : même miroir + pondération ventes partielles. Exposé publiquement —
/// le handler SMC dédié ajoute les features importances par-dessus.
pub async fn stats_smc(state: &web::Data<AppState>) -> serde_json::Value {
    let f = crate::reglages_smc::lire_fractions(&state.db).await;
    stats_json(state.db.pool(), "%smc%", true, &f).await
}
