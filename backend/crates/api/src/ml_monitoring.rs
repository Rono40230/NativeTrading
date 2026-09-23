//! Monitoring ML par stratégie — MIROIR de l'historique des trades clôturés
//! (décision propriétaire 23/09), en R DISTANCE (23/09 soir : LE R affiché
//! partout — juge la stratégie, indépendant des réglages de sortie ; le $
//! juge le résultat). Source : `signaux` Fermé+rempli, r = `r_realise`
//! (distance) pour toutes les stratégies — plus aucune pondération ici.

use actix_web::{web, HttpResponse, Responder};
use sqlx::Row;

use crate::state::AppState;

/// Statistiques d'une stratégie, miroir de son historique de trades
/// clôturés — R DISTANCE (`r_realise` : niveau le plus lointain atteint).
pub async fn stats_json(pool: &sqlx::SqlitePool, filtre: &str) -> serde_json::Value {
    let rows = sqlx::query(
        "SELECT verdict, r_realise
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
            let r: f64 = r.get::<Option<f64>, _>("r_realise").unwrap_or(0.0);
            (verdict, r)
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
                "r_somme":     somme,
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
    HttpResponse::Ok().json(stats_json(state.db.pool(), "%straddle%").await)
}

pub async fn rockets(state: web::Data<AppState>) -> impl Responder {
    HttpResponse::Ok().json(stats_json(state.db.pool(), "%rockets%").await)
}

pub async fn kdj(state: web::Data<AppState>) -> impl Responder {
    HttpResponse::Ok().json(stats_json(state.db.pool(), "%kdj%").await)
}

/// SMC : même miroir en R distance — le handler dédié ajoute les features
/// importances par-dessus.
pub async fn stats_smc(state: &web::Data<AppState>) -> serde_json::Value {
    stats_json(state.db.pool(), "%smc%").await
}
