use actix_web::{web, HttpResponse, Responder};
use serde::Serialize;
use std::sync::atomic::Ordering;

use crate::state::AppState;

#[derive(Serialize)]
pub struct ContexteMarche {
    /// true si un événement macro High-impact est dans ≤30 min → SMC suspendu
    pub smc_suspendu: bool,
    /// Nom de l'événement déclencheur (si smc_suspendu)
    pub raison: Option<String>,
    /// Minutes restantes avant l'événement (si smc_suspendu)
    pub minutes_restantes: Option<i64>,
    /// Score max des news actives (-1 = inconnu)
    pub score_news: i32,
    /// Valeur Fear & Greed en cours (-1 = inconnue)
    pub fear_greed: i32,
}

/// GET /api/news/contexte-marche
/// Retourne l'état macro actuel : suspension SMC éventuelle + score news + F&G.
pub async fn get_contexte_marche(state: web::Data<AppState>) -> impl Responder {
    let (smc_suspendu, raison, minutes_restantes) =
        match state.db.fenetre_macro_smc_dans_minutes().await {
            Ok(Some((titre, min))) => (true, Some(titre), Some(min)),
            _ => (false, None, None),
        };

    HttpResponse::Ok().json(ContexteMarche {
        smc_suspendu,
        raison,
        minutes_restantes,
        score_news: state.score_news.load(Ordering::Relaxed) as i32,
        fear_greed: state.fg_valeur.load(Ordering::Relaxed) as i32,
    })
}
