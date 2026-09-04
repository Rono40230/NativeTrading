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
    /// Bougie EN FORMATION par (asset, tf) — l'EA la pousse chaque seconde ;
    /// le flux du graphique la sert telle quelle (vrai OHLC).
    formations: HashMap<(String, String), (i64, f64, f64, f64, f64, f64)>,
}

fn etat() -> &'static Mutex<EtatMt5> {
    static ETAT: OnceLock<Mutex<EtatMt5>> = OnceLock::new();
    ETAT.get_or_init(|| {
        Mutex::new(EtatMt5 {
            dernier_contact: 0,
            derniers_prix: HashMap::new(),
            formations: HashMap::new(),
        })
    })
}

/// Canal vers le runtime (posé au démarrage par demarrer_runtime_tick).
fn canal_runtime() -> &'static Mutex<Option<UnboundedSender<EvenementPrix>>> {
    static CANAL: OnceLock<Mutex<Option<UnboundedSender<EvenementPrix>>>> = OnceLock::new();
    CANAL.get_or_init(|| Mutex::new(None))
}

/// Dernier prix LIVE d'un actif MT5 (bougie en formation poussée par l'EA,
/// fraîche à la seconde) — None si absent ou muet depuis > 120 s.
pub fn prix_live(asset: &str) -> Option<f64> {
    let e = etat().lock().unwrap_or_else(|e| e.into_inner());
    let (ts, prix) = e.derniers_prix.get(asset)?;
    if chrono::Utc::now().timestamp() - ts < 120 {
        Some(*prix)
    } else {
        None
    }
}

/// Bougie en formation d'un couple (asset, tf) — servie au graphique.
/// None si l'EA est muet depuis > 120 s (MT5 fermé) ou TF inconnu.
pub fn bougie_en_formation(
    asset: &str,
    tf: &str,
) -> Option<(i64, f64, f64, f64, f64, f64)> {
    let e = etat().lock().unwrap_or_else(|e| e.into_inner());
    if Utc::now().timestamp() - e.dernier_contact > 120 {
        return None;
    }
    e.formations
        .get(&(asset.to_string(), tf.to_string()))
        .copied()
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
    // En-tête #TF : les timeframes moteurs configurés + D1/W1 pour l'amorce
    // MTF des actifs v12 (l'EA pousse l'historique de chaque TF listé).
    let tfs_configures = data::worker_config::lire_timeframes(&state.db).await;
    let mut tfs: Vec<String> = tfs_configures.iter().map(|t| t.as_str().to_string()).collect();
    for extra in ["D1", "W1"] {
        if !tfs.contains(&extra.to_string()) {
            tfs.push(extra.to_string());
        }
    }
    let entete = format!(
        "#TF {}\n",
        tfs.iter()
            .map(|t| format!("{}:{}", t, profondeur_historique(t)))
            .collect::<Vec<_>>()
            .join(",")
    );
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
    HttpResponse::Ok()
        .content_type("text/plain")
        .body(entete + &corps)
}

/// Profondeur d'historique demandée à l'EA par TF — alignée sur la
/// RÉTENTION (24 mois) pour que la couverture affiche la vérité ; les
/// moteurs rejouent leurs 7 jours par-dessus. L'EA pousse par morceaux.
fn profondeur_historique(tf: &str) -> i64 {
    match tf {
        "M1" => 720_000,
        "M5" => 144_000,
        "M15" => 48_000,
        "M30" => 24_000,
        "H1" => 12_500,
        "H4" => 3_100,
        "D1" => 520,
        "W1" => 110,
        _ => 500,
    }
}

// ── POST /api/mt5/kline — une bougie M1 (formation ou confirmation) ────────

#[derive(serde::Deserialize)]
pub struct BodyKline {
    pub asset: String,
    /// Timeframe de la bougie (M1 par défaut — rétrocompatible EA v1).
    pub tf: Option<String>,
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
    let tf_str = body.tf.clone().unwrap_or_else(|| "M1".into());
    let Some(tf) = Timeframe::try_from(tf_str.as_str()).ok() else {
        return HttpResponse::BadRequest().json(serde_json::json!({ "error": "timeframe inconnu" }));
    };
    let k = body.into_inner();

    // 1. Runtime : même chemin que Bybit (snapshot de formation / clôture).
    let event = EvenementPrix {
        asset: asset.clone(),
        tf,
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
            .inserer_bougies_avec_source(&asset, &tf, &[bougie], "mt5")
            .await;
    }

    // 3. Statut.
    {
        let mut e = etat().lock().unwrap_or_else(|e| e.into_inner());
        e.dernier_contact = Utc::now().timestamp();
        e.derniers_prix
            .insert(asset.as_str().to_string(), (Utc::now().timestamp(), k.c));
        e.formations.insert(
            (asset.as_str().to_string(), tf.as_str().to_string()),
            (k.debut, k.o, k.h, k.l, k.c, k.v),
        );
    }
    HttpResponse::Ok().json(serde_json::json!({ "runtime": envoye }))
}

// ── POST /api/mt5/historique — batch initial d'un TF (JSON) ────────────────
// L'EA pousse l'historique Axi d'un (asset, TF) en un seul appel : les
// bougies sont insérées (source mt5) AVANT que le runtime n'arme les
// moteurs v12 (garde de profondeur) — replays SMC complets, amorce MTF
// comprise (W1/MN depuis D1).

#[derive(serde::Deserialize)]
pub struct BodyHistorique {
    pub asset: String,
    pub tf: String,
    /// [[debut, o, h, l, c, v], ...] chronologique.
    pub b: Vec<(i64, f64, f64, f64, f64, f64)>,
}

pub async fn post_historique(state: web::Data<AppState>, body: web::Json<BodyHistorique>) -> impl Responder {
    let Some(asset) = Asset::try_from(body.asset.as_str()).ok() else {
        return HttpResponse::BadRequest().json(serde_json::json!({ "error": "asset inconnu" }));
    };
    let Some(tf) = Timeframe::try_from(body.tf.as_str()).ok() else {
        return HttpResponse::BadRequest().json(serde_json::json!({ "error": "timeframe inconnu" }));
    };
    let bougies: Vec<common::Candle> = body
        .b
        .iter()
        .filter_map(|(debut, o, h, l, c, v)| {
            Some(common::Candle {
                timestamp: Utc.timestamp_opt(*debut, 0).single()?,
                open: *o,
                high: *h,
                low: *l,
                close: *c,
                volume: *v,
            })
        })
        .collect();
    let n = bougies.len();
    match state
        .db
        .inserer_bougies_avec_source(&asset, &tf, &bougies, "mt5")
        .await
    {
        Ok(_) => {
            tracing::info!("🖥️ MT5 historique {} {} : {} bougies Axi", asset.as_str(), tf.as_str(), n);
            HttpResponse::Ok().json(serde_json::json!({ "inserees": n }))
        }
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({ "error": e.to_string() })),
    }
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

/// Profondeur de replay v12 par TF (7 jours bornés — même règle que le
/// runtime Bybit).
pub(crate) fn profondeur_replay(tf: common::Timeframe) -> i64 {
    (7 * 1440 / tf.minutes() as i64).clamp(60, 10_080)
}

/// Assez d'historique Axi en base pour armer le replay v12 proprement ?
/// (l'EA pousse l'historique au premier passage — on attend qu'il soit là).
pub(crate) async fn historique_mt5_pret(db: &std::sync::Arc<db::Database>, asset: &str, tf: common::Timeframe) -> bool {
    let besoin = (profondeur_replay(tf) as f64 * 0.6) as i64;
    let a = Asset::from(asset);
    match db.compter_bougies(&a, &tf).await {
        Ok(n) => n >= besoin.min(60),
        Err(_) => false,
    }
}
