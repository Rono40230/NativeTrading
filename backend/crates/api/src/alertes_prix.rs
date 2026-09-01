//! Alertes de prix — watcher du runtime + endpoints HTTP.
//!
//! Le runtime tick voit CHAQUE prix live (Bybit WS pour les cryptos, EA
//! chaque seconde pour les MT5) : c'est l'observateur naturel. Le watcher
//! maintient un cache mémoire des alertes actives (rechargé toutes les
//! 60 s avec la config) et déclenche au franchissement : désarmage DB +
//! message Telegram. L'app notifie côté poste (son + notification OS) via
//! son polling de la liste.

use std::sync::Arc;

use actix_web::{web, HttpResponse, Responder};
use db::Database;
use engine::types::EvenementPrix;
use serde::Deserialize;

use crate::state::AppState;

/// Cache mémoire des alertes actives (vue du watcher).
#[derive(Default)]
pub struct CacheAlertes {
    pub alertes: Vec<db::alertes_prix::AlertePrix>,
}

impl CacheAlertes {
    pub async fn recharger(&mut self, db: &Database) {
        match db.lister_alertes_actives().await {
            Ok(a) => self.alertes = a,
            Err(e) => tracing::warn!("Alertes prix (recharge cache): {}", e),
        }
    }
}

/// Vérifie un prix live contre les alertes actives de l'asset.
/// Déclenche : désarmage + Telegram + retrait du cache.
pub async fn verifier(db: &Database, cache: &mut CacheAlertes, ev: &EvenementPrix) {
    let asset = ev.asset.as_str();
    let prix = ev.event.prix();
    let mut declenchees: Vec<i64> = Vec::new();
    cache.alertes.retain(|a| {
        if a.asset != asset || !a.active {
            return true;
        }
        let franchi = if a.sens == "au_dessus" {
            prix >= a.prix
        } else {
            prix <= a.prix
        };
        if franchi {
            declenchees.push(a.id);
            false // retirée du cache
        } else {
            true
        }
    });
    for id in declenchees {
        let Some(a) = db.declencher_alerte_prix(id).await.ok().flatten() else {
            continue;
        };
        let sens_txt = if a.sens == "au_dessus" { "monté à" } else { "descendu à" };
        let note = a.note.as_deref().map(|n| format!("\n📝 {n}")).unwrap_or_default();
        let msg = format!(
            "🔔 Alerte prix\n{} a {} {:.2}\nSeuil : {:.2} ({}){}",
            a.asset,
            sens_txt,
            prix,
            a.prix,
            if a.sens == "au_dessus" { "au-dessus" } else { "en-dessous" },
            note,
        );
        let (token, chat) = notifications::telegram::lire_tokens_pool(db.pool()).await;
        if !token.is_empty() && !chat.is_empty() {
            if let Err(e) = notifications::telegram::post_message(&token, &chat, &msg).await {
                tracing::warn!("Alerte prix (Telegram): {}", e);
            }
        }
        tracing::info!("🔔 Alerte prix déclenchée : {} {} {:.2} (prix {:.2})", a.asset, a.sens, a.prix, prix);
    }
}

// ── Endpoints HTTP ───────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct BodyAlerte {
    pub asset: String,
    pub prix: f64,
    /// 'au_dessus' | 'en_dessous' — optionnel : auto si absent.
    pub sens: Option<String>,
    pub note: Option<String>,
}

/// GET /api/alertes-prix — toutes les alertes (actives d'abord).
pub async fn lister(state: web::Data<AppState>) -> impl Responder {
    match state.db.lister_alertes_prix().await {
        Ok(alertes) => HttpResponse::Ok().json(alertes),
        Err(e) => HttpResponse::InternalServerError().body(format!("{e}")),
    }
}

/// POST /api/alertes-prix — pose une alerte.
pub async fn creer(state: web::Data<AppState>, body: web::Json<BodyAlerte>) -> impl Responder {
    if body.prix <= 0.0 {
        return HttpResponse::BadRequest().json(serde_json::json!({ "error": "Prix invalide" }));
    }
    let sens = match body.sens.as_deref() {
        Some(s @ ("au_dessus" | "en_dessous")) => s.to_string(),
        _ => "au_dessus".to_string(),
    };
    match state
        .db
        .creer_alerte_prix(&body.asset, body.prix, &sens, body.note.as_deref())
        .await
    {
        Ok(id) => HttpResponse::Ok().json(serde_json::json!({ "id": id })),
        Err(e) => HttpResponse::InternalServerError().body(format!("{e}")),
    }
}

/// DELETE /api/alertes-prix/{id}
pub async fn supprimer(state: web::Data<AppState>, chemin: web::Path<i64>) -> impl Responder {
    match state.db.supprimer_alerte_prix(chemin.into_inner()).await {
        Ok(_) => HttpResponse::Ok().json(serde_json::json!({ "ok": true })),
        Err(e) => HttpResponse::InternalServerError().body(format!("{e}")),
    }
}

/// Helpers pour le runtime.
pub fn cache_vide() -> CacheAlertes {
    CacheAlertes::default()
}

pub type CacheAlertesPartage = Arc<tokio::sync::Mutex<CacheAlertes>>;
