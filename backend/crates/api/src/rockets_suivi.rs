use actix_web::{web, HttpResponse, Responder};
use db::rockets;

use crate::rockets_prix::fetch_prix;
use crate::state::AppState;

pub use strategies::rockets_niveaux::calculer_verdict_rocket;

// ── POST /api/rockets/sync ───────────────────────────────────────────────────

/// Force un cycle de suivi immédiat (SL/TP attente + ouverts)
pub async fn sync_verdicts(state: web::Data<AppState>) -> impl Responder {
    let pool = state.db.pool();
    let client = &*crate::http_client::HTTP_CLIENT;

    let mut fermes = 0u32;
    let mut ouverts_nouveaux = 0u32;

    // Attente : SL avant entrée, ou ouvrir si prix atteint
    if let Ok(en_attente) = rockets::lister_en_attente(pool).await {
        for s in &en_attente {
            let Some(prix) = fetch_prix(&client, &s.ticker).await else {
                continue;
            };
            if prix <= s.stop_loss
                && rockets::maj_verdict(pool, s.id, "invalide", prix)
                    .await
                    .is_ok()
            {
                fermes += 1;
            } else if prix >= s.prix_entree && rockets::entrer_position(pool, s.id).await.is_ok() {
                ouverts_nouveaux += 1;
            }
        }
    }

    // Charger config pour respect du flag vente_partielle
    let cfg = rockets::lire_config(pool).await;

    // Ouverts : SL/TP
    if let Ok(signaux) = rockets::lister_ouverts(pool).await {
        for s in &signaux {
            let Some(prix) = fetch_prix(&client, &s.ticker).await else {
                continue;
            };
            let peak_precedent = s.prix_peak.unwrap_or(s.prix_entree);
            let peak = peak_precedent.max(prix);
            if peak > peak_precedent {
                let _ = rockets::maj_prix_peak(pool, s.id, peak).await;
            }
            match calculer_verdict_rocket(s, prix, peak, peak_precedent) {
                Some(v @ "TP1") | Some(v @ "TP2") if cfg.vente_partielle => {
                    // Vente partielle ⅓ — position reste ouverte
                    let _ = rockets::enregistrer_tp_partiel(pool, s.id, v, prix).await;
                }
                Some(v) if rockets::maj_verdict(pool, s.id, v, prix).await.is_ok() => {
                    fermes += 1;
                }
                Some(_) => {}
                None => {}
            }
        }
    }

    HttpResponse::Ok().json(serde_json::json!({
        "fermes": fermes,
        "ouverts_nouveaux": ouverts_nouveaux
    }))
}

// ── POST /api/rockets/sync-feedback ──────────────────────────────────────────

#[cfg(test)]
#[path = "rockets_suivi_tests.rs"]
mod tests;
