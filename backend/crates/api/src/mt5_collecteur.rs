//! Phase 5 — collecteur MT5/Axi : l'EA pousse les bougies M1 de ton broker
//! vers l'app (même chemin que Bybit : runtime temps réel + bougies
//! officielles en base). L'EA découvre ses abonnements via GET symboles ;
//! statut servi à la vue Données.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use actix_web::{web, HttpResponse, Responder};
use chrono::{Datelike, TimeZone, Utc};
use common::{Asset, Timeframe};
use engine::types::{EvenementPrix, PrixEvent};
use sqlx::Row;
use tokio::sync::mpsc::UnboundedSender;

use crate::state::AppState;

/// État de connexion du collecteur (en mémoire — indicatif).
struct EtatMt5 {
    /// Dernier heartbeat de l'EA (epoch s).
    dernier_contact: i64,
    /// Dernier prix poussé par actif : (epoch s, prix).
    derniers_prix: HashMap<String, (i64, f64)>,
}

fn etat() -> &'static Mutex<EtatMt5> {
    static ETAT: OnceLock<Mutex<EtatMt5>> = OnceLock::new();
    ETAT.get_or_init(|| {
        Mutex::new(EtatMt5 {
            dernier_contact: 0,
            derniers_prix: HashMap::new(),
        })
    })
}

/// Canal vers le runtime (posé au démarrage par demarrer_runtime_tick).
fn canal_runtime() -> &'static Mutex<Option<UnboundedSender<EvenementPrix>>> {
    static CANAL: OnceLock<Mutex<Option<UnboundedSender<EvenementPrix>>>> = OnceLock::new();
    CANAL.get_or_init(|| Mutex::new(None))
}

/// Branche le canal runtime (appelé une fois au boot).
pub fn brancher_canal(tx: UnboundedSender<EvenementPrix>) {
    *canal_runtime().lock().unwrap_or_else(|e| e.into_inner()) = Some(tx);
}

// ── GET /api/mt5/symboles — liste d'abonnement de l'EA (texte simple) ──────
// Format par ligne : `symbole_mt5|asset_app` — un symbole par ligne, sans
// JSON (l'EA MQL5 n'a pas de parseur JSON natif).

pub async fn get_symboles(state: web::Data<AppState>) -> impl Responder {
    let Ok(rows) = sqlx::query(
        "SELECT symbol_mt5, id FROM assets WHERE source = 'mt5' AND actif = 1 AND symbol_mt5 IS NOT NULL",
    )
    .fetch_all(state.db.pool())
    .await
    else {
        return HttpResponse::Ok().content_type("text/plain").body("");
    };
    let corps: String = rows
        .iter()
        .map(|r| {
            format!(
                "{}|{}\n",
                r.get::<String, _>("symbol_mt5"),
                r.get::<String, _>("id")
            )
        })
        .collect();
    HttpResponse::Ok().content_type("text/plain").body(corps)
}

// ── POST /api/mt5/kline — une bougie M1 (formation ou confirmation) ────────

#[derive(serde::Deserialize)]
pub struct BodyKline {
    pub asset: String,
    pub debut: i64,
    pub o: f64,
    pub h: f64,
    pub l: f64,
    pub c: f64,
    pub v: f64,
    /// 1 = clôture officielle de la minute (écrite en base).
    pub conf: i64,
}

pub async fn post_kline(state: web::Data<AppState>, body: web::Form<BodyKline>) -> impl Responder {
    let Some(asset) = Asset::try_from(body.asset.as_str()).ok() else {
        return HttpResponse::BadRequest().json(serde_json::json!({ "error": "asset inconnu" }));
    };
    let k = body.into_inner();

    // 1. Runtime : même chemin que Bybit (snapshot de formation / clôture).
    let event = EvenementPrix {
        asset: asset.clone(),
        tf: Timeframe::M1,
        debut_bougie: k.debut,
        event: PrixEvent::Kline {
            ouverture: k.o,
            haut: k.h,
            bas: k.l,
            cloture: k.c,
            volume: k.v,
            confirmee: k.conf == 1,
        },
        recu_le: Utc::now(),
    };
    let envoye = canal_runtime()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .as_ref()
        .map(|tx| tx.send(event).is_ok())
        .unwrap_or(false);

    // 2. Base : les clôtures officielles du broker (série de vérité MT5).
    if k.conf == 1 {
        let bougie = common::Candle {
            timestamp: Utc.timestamp_opt(k.debut, 0).single().unwrap_or_default(),
            open: k.o,
            high: k.h,
            low: k.l,
            close: k.c,
            volume: k.v,
        };
        let _ = state
            .db
            .inserer_bougies_avec_source(&asset, &Timeframe::M1, &[bougie], "mt5")
            .await;
    }

    // 3. Statut.
    {
        let mut e = etat().lock().unwrap_or_else(|e| e.into_inner());
        e.dernier_contact = Utc::now().timestamp();
        e.derniers_prix
            .insert(asset.as_str().to_string(), (Utc::now().timestamp(), k.c));
    }
    HttpResponse::Ok().json(serde_json::json!({ "runtime": envoye }))
}

// ── POST /api/mt5/heartbeat — l'EA signale sa présence (30 s) ──────────────

pub async fn post_heartbeat(state: web::Data<AppState>) -> impl Responder {
    let _ = &state;
    etat().lock().unwrap_or_else(|e| e.into_inner()).dernier_contact = Utc::now().timestamp();
    HttpResponse::Ok().json(serde_json::json!({ "ok": true }))
}

// ── GET /api/mt5/statut — carte de la vue Données ──────────────────────────

pub async fn get_statut(state: web::Data<AppState>) -> impl Responder {
    let maintenant = Utc::now().timestamp();
    let (dernier_contact, derniers_prix) = {
        let e = etat().lock().unwrap_or_else(|e| e.into_inner());
        (e.dernier_contact, e.derniers_prix.clone())
    };
    let connecte = dernier_contact > 0 && maintenant - dernier_contact < 90;
    let Ok(rows) = sqlx::query(
        "SELECT id FROM assets WHERE source = 'mt5' AND actif = 1",
    )
    .fetch_all(state.db.pool())
    .await
    else {
        return HttpResponse::Ok().json(serde_json::json!({ "connecte": connecte, "symboles": [] }));
    };
    let symboles: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            let id: String = r.get("id");
            let (age, prix) = derniers_prix
                .get(&id)
                .map(|(t, p)| (maintenant - t, Some(*p)))
                .unwrap_or((-1, None));
            serde_json::json!({ "asset": id, "age_s": age, "dernier_prix": prix })
        })
        .collect();
    HttpResponse::Ok().json(serde_json::json!({
        "connecte": connecte,
        "dernier_contact": dernier_contact,
        "symboles": symboles,
    }))
}

/// Les annonces synthétiques du DAX : ouverture européenne (9h00 Paris,
/// jours ouvrés) pour les 7 prochains jours — même structure que le
/// calendrier, injectée au moteur straddle.
pub fn annonces_ouverture_europeenne() -> Vec<straddle::Annonce> {
    use chrono::TimeZone;
    let paris = chrono_tz::Europe::Paris;
    let mut annonces = Vec::new();
    let aujourd_hui = Utc::now().date_naive();
    for jours in 0..8 {
        let Some(date) = aujourd_hui.checked_add_signed(chrono::Duration::days(jours)) else {
            continue;
        };
        if date.weekday().number_from_monday() >= 6 {
            continue; // week-end : pas de session
        }
        let Some(h) = paris
            .with_ymd_and_hms(date.year(), date.month(), date.day(), 9, 0, 0)
            .single()
        else {
            continue;
        };
        let ts = h.timestamp();
        if ts > Utc::now().timestamp() {
            annonces.push(straddle::Annonce {
                ts,
                devise: "EUR".into(),
                titre: "Ouverture européenne".into(),
            });
        }
    }
    annonces
}
