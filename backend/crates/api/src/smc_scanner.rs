//! Scanner SMC (11/09) — exposition pour la page dédiée :
//! GET /api/smc/mtf            → tendances H1/H4 par asset du runtime
//!                               (f_htf Pine, bougies clôturées — la
//!                               confirmation multi-TF du propriétaire,
//!                               enfin visible)
//! GET /api/smc/setups-journal → cycle de vie des setups (signal/dissipe),
//!                               matière première de l'étude §3.2.

use actix_web::{web, HttpResponse, Responder};

use crate::state::AppState;

/// GET /api/smc/mtf — { asset: { h1: -1|0|1, h4: -1|0|1 } }. H1 : ~90 jours,
/// H4 : ~1 an de bougies (f_htf rejoue ses pivots dessus). 0 = pas assez
/// d'historique pour trancher.
pub async fn get_mtf(state: web::Data<AppState>) -> impl Responder {
    let db = &state.db;
    // Filtrer par armement (fix 27/09) : le scanner ne montre que les
    // couples ARMÉS — les assets décochés dans « Assets & TF » doivent
    // disparaître du scanner au tick suivant (60 s), pas rester visibles.
    let armes = crate::reglages_smc::lire_couples_armes(db).await;
    let tous = crate::runtime_tick::assets_runtime(db).await;
    let assets: Vec<_> = tous
        .into_iter()
        .filter(|a| {
            match armes.get(a.as_str()) {
                Some(tfs) => !tfs.is_empty(),
                None => false, // non configuré = désarmé (l'ancien défaut « tout armé » rendait le filtre inopérant)
            }
        })
        .collect();
    let mut out = serde_json::Map::new();
    for a in &assets {
        let h1 = db
            .obtenir_bougies_depuis_jours(&a, &common::Timeframe::H1, 90)
            .await
            .unwrap_or_default();
        let h4 = db
            .obtenir_bougies_depuis_jours(&a, &common::Timeframe::H4, 365)
            .await
            .unwrap_or_default();
        out.insert(
            a.as_str().to_string(),
            serde_json::json!({
                "h1": if h1.len() >= 30 { smc::v12::tendance_htf(&h1) } else { 0 },
                "h4": if h4.len() >= 30 { smc::v12::tendance_htf(&h4) } else { 0 },
            }),
        );
    }
    HttpResponse::Ok().json(serde_json::Value::Object(out))
}

#[derive(serde::Deserialize, Default)]
pub struct JournalQuery {
    limite: Option<i64>,
}

/// GET /api/smc/setups-journal?limite=300 — cycles de vie les plus récents
/// d'abord (vivants comme clôturés : issue = signal | dissipe | null).
pub async fn get_journal(
    state: web::Data<AppState>,
    q: web::Query<JournalQuery>,
) -> impl Responder {
    let limite = q.limite.unwrap_or(300).clamp(1, 1000);
    // Filtrer par armement (fix 27/09) : ne montrer que les cycles des
    // couples armés — les assets décochés disparaissent du scanner.
    let armes = crate::reglages_smc::lire_couples_armes(&state.db).await;
    let armes_set: std::collections::HashSet<&str> = armes
        .iter()
        .filter(|(_, tfs)| !tfs.is_empty())
        .map(|(a, _)| a.as_str())
        .collect();
    match db::smc_setups_journal::lister(state.db.pool(), limite).await {
        Ok(rows) => {
            let filtres: Vec<_> = rows
                .into_iter()
                .filter(|r| armes_set.contains(r.asset.as_str()))
                .collect();
            HttpResponse::Ok().json(filtres)
        }
        Err(e) => HttpResponse::InternalServerError()
            .json(serde_json::json!({ "error": e.to_string() })),
    }
}
