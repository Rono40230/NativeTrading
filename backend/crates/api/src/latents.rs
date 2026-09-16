//! R latent TOTAL des trades en cours (15/09) — ingrédients servis au front,
//! qui compose au prix WS vivant : `r = r_fixe + coef_prix × prix`.
//!
//! Conventions (celles des moteurs, calibrées sur la base vécue) :
//! - SMC : paliers au toucher, fractions 0,5 (TP1) / 0,3 (TP2) / 0,2 (solde)
//!   (gestion_trades::pondere). Les touchers sont reconstruits depuis les
//!   bougies du TF du signal (mèche touchée = vente, cf. replay v12).
//! - KDJ : pleine position, pas de paliers → r = dir×(prix−E)/risque.
//! - Straddle : unité R = |E−SL|/1,5 (SL vécu = −1,5 R) ; la jambe opposée
//!   est comptée −1,5 R dès le déclenchement (schéma dominant : la cassure
//!   emporte le stop opposé — approximation documentée, tooltip côté front).
//! Direction : dérivée de la position du TP (fiable aussi pour les passes
//! straddle « Both », dont le TP suit la jambe gagnante).

use actix_web::{web, HttpResponse, Responder};

use crate::state::AppState;

/// Fractions officielles SMC (miroir de gestion_trades::pondere).
const F_TP1: f64 = 0.5;
const F_TP2: f64 = 0.3;

#[derive(serde::Serialize)]
struct LatentOut {
    id: String,
    /// Part réalisée ou fixe du R (paliers vendus SMC, jambe morte straddle).
    r_fixe: f64,
    /// Coefficient appliqué à l'ÉCART au prix d'entrée :
    /// r = r_fixe + coef_prix × (prix − prix_entree).
    coef_prix: f64,
    prix_entree: f64,
}

fn premier_tp(brut: &str) -> Option<f64> {
    let v: Vec<f64> = serde_json::from_str(brut).ok()?;
    v.first().copied()
}

fn deuxieme_tp(brut: &str) -> Option<f64> {
    let v: Vec<f64> = serde_json::from_str(brut).ok()?;
    v.get(1).copied()
}

pub async fn latents(state: web::Data<AppState>) -> impl Responder {
    #[derive(sqlx::FromRow)]
    struct Ligne {
        id: String,
        asset: String,
        timeframe: String,
        strategie: String,
        prix_entree: f64,
        stop_loss: f64,
        take_profit: String,
        heure_entree: i64,
    }
    let lignes: Vec<Ligne> = match sqlx::query_as(
        "SELECT id, asset, timeframe, strategie, prix_entree, stop_loss, take_profit, heure_entree
         FROM signaux
         WHERE statut = 'Actif' AND heure_entree IS NOT NULL",
    )
    .fetch_all(state.db.pool())
    .await
    {
        Ok(l) => l,
        Err(_) => return HttpResponse::InternalServerError().json(serde_json::json!({"error": "lecture signaux"})),
    };

    let mut out = Vec::with_capacity(lignes.len());
    for s in &lignes {
        let Some(tp1) = premier_tp(&s.take_profit) else { continue };
        let risque = (s.prix_entree - s.stop_loss).abs();
        if risque <= 0.0 || tp1 == s.prix_entree {
            continue;
        }
        let dir = if tp1 > s.prix_entree { 1.0 } else { -1.0 };
        let strategie = s.strategie.to_lowercase();

        let (r_fixe, coef) = if s.strategie == "straddle" {
            // Unité R = risque/1,5 ; jambe opposée morte à −1,5 R.
            (-1.5, 1.5 * dir / risque)
        } else if strategie.starts_with("smc") {
            // Touchers de paliers depuis l'entrée, sur les bougies du TF.
            let (Ok(asset), Ok(tf)) = (
                common::Asset::try_from(s.asset.as_str()),
                common::Timeframe::try_from(s.timeframe.as_str()),
            ) else {
                continue;
            };
            let nb_barres = ((chrono::Utc::now().timestamp() - s.heure_entree)
                / (tf.minutes() as i64 * 60)
                + 4)
            .clamp(2, 1000) as i64;
            let bougies = state
                .db
                .obtenir_bougies(&asset, &tf, nb_barres)
                .await
                .unwrap_or_default();
            let depuis: Vec<&common::Candle> = bougies
                .iter()
                .filter(|b| b.timestamp.timestamp() >= s.heure_entree)
                .collect();
            let mut r_fixe = 0.0;
            let mut vendu = 0.0;
            if !depuis.is_empty() {
                let extremum = if dir > 0.0 {
                    depuis.iter().map(|b| b.high).fold(f64::MIN, f64::max)
                } else {
                    depuis.iter().map(|b| b.low).fold(f64::MAX, f64::min)
                };
                if dir * (extremum - tp1) >= 0.0 {
                    r_fixe += F_TP1 * (dir * (tp1 - s.prix_entree) / risque);
                    vendu += F_TP1;
                    if let Some(tp2) = deuxieme_tp(&s.take_profit) {
                        if dir * (extremum - tp2) >= 0.0 {
                            r_fixe += F_TP2 * (dir * (tp2 - s.prix_entree) / risque);
                            vendu += F_TP2;
                        }
                    }
                }
            }
            (r_fixe, (1.0 - vendu) * dir / risque)
        } else {
            // KDJ et assimilés : pleine position.
            (0.0, dir / risque)
        };

        out.push(LatentOut {
            id: s.id.clone(),
            r_fixe,
            coef_prix: coef,
            prix_entree: s.prix_entree,
        });
    }

    HttpResponse::Ok().json(serde_json::json!({ "latents": out }))
}
