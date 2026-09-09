//! Clôture manuelle des positions rockets (décision propriétaire 09/09) —
//! extraite de rockets_verticale (pré-audit 600 lignes). Le prix de marché
//! et le calcul R réutilisent les helpers publics du module d'origine.

use actix_web::{web, HttpResponse, Responder};
use sqlx::Row;

use crate::state::AppState;

#[derive(serde::Deserialize)]
pub struct BodyCloture {
    pub cle: String,
}

/// POST /api/rockets/positions/cloturer — clôture PROPRIÉTAIRE au prix de
/// marché : crypto → dernier Binance, action → Yahoo (même source que la
/// gestion). Même math que le moteur (50 % encaissés à R1 si neutralisée,
/// solde au prix de sortie), verdict « Manuel », passe par le point de
/// passage commun `fermer_signal_par_cle` → historique + capital + ML.
pub async fn cloturer_manuel(state: web::Data<AppState>, body: web::Json<BodyCloture>) -> impl Responder {
let row = match sqlx::query(
        "SELECT symbole, entree, stop, neutralise FROM rockets_positions WHERE cle = ? AND fermee = 0",
    )
    .bind(&body.cle)
    .fetch_optional(state.db.pool())
    .await
    {
        Ok(Some(r)) => r,
        Ok(None) => {
            return HttpResponse::NotFound()
                .json(serde_json::json!({ "error": "Position inconnue ou déjà fermée" }))
        }
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({ "error": e.to_string() }))
        }
    };
    let symbole: String = row.get("symbole");
    let entree: f64 = row.get("entree");
    let stop: f64 = row.get("stop");
    let neutralise: bool = row.get::<i64, _>("neutralise") != 0;

    // Prix de marché (identique aux sources de la gestion 30 s).
    let prix = if symbole.ends_with("USDT") {
        match crate::rockets_verticale::klines_d1(&symbole, 1).await.last().map(|b| b.close) {
            Some(p) => p,
            None => {
                return HttpResponse::ServiceUnavailable()
                    .json(serde_json::json!({ "error": "Cours Binance indisponible — réessaie" }))
            }
        }
    } else {
        match crate::yahoo_quotes::quotes(&[symbole.clone()]).await.get(&symbole).map(|q| q.prix) {
            Some(p) => p,
            None => {
                return HttpResponse::ServiceUnavailable()
                    .json(serde_json::json!({ "error": "Cours Yahoo indisponible — réessaie" }))
            }
        }
    };

    // Même math que pas_gestion : non neutralisée = position pleine au
    // prix ; neutralisée = 50 % encaissés à R1 (+0,5 R) + solde au prix.
    let risque = entree - stop;
    if risque <= 0.0 {
        return HttpResponse::InternalServerError().json(serde_json::json!({ "error": "Position incohérente (risque nul)" }));
    }
    let r_solde = (prix - entree) / risque;
    let r_realise = if neutralise { 0.5 + 0.5 * r_solde } else { r_solde };

    let maintenant = chrono::Utc::now().timestamp();
    let _ = sqlx::query(
        "UPDATE rockets_positions SET fermee = 1, verdict = 'Manuel', r_realise = ?, prix_sortie = ? WHERE cle = ?",
    )
    .bind(r_realise)
    .bind(prix)
    .bind(&body.cle)
    .execute(state.db.pool())
    .await;
    let _ = state
        .db
        .fermer_signal_par_cle(&body.cle, &symbole, "Manuel", prix, r_realise, maintenant)
        .await;
    tracing::info!("🚀 Rockets {symbole} : clôture manuelle à {prix:.4} ({r_realise:+.2} R)");
    HttpResponse::Ok().json(serde_json::json!({
        "ok": true, "symbole": symbole, "prix": prix, "r_realise": r_realise
    }))
}

