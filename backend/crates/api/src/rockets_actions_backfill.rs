//! Backfill hiérarchisé des actions US (étape A2, 31/08).
//!
//! Quota Tiingo gratuit — MESURÉ le 31/08 : ~65 requêtes/HEURE (le cap
//! horaire mord avant le journalier) + garde journalière à 900. Rythme :
//! lots de 50 toutes les 65 min → univers complet en ~5-6 jours, puis
//! tournante de rafraîchissement. Priorité :
//! 1. les tickers PRIORITAIRES sans bougies (noyau liquide — résultats de
//!    trend template exploitables dès le premier jour),
//! 2. le reste de l'univers sans bougies (marché couvert en ~6 jours),
//! 3. en tournante, le rafraîchissement des tickers déjà backfillés
//!    (les moins récents d'abord).
//! Historique initial : 400 jours (MM200 + 1 mois + fenêtre 52 semaines).

use actix_web::{web, HttpResponse, Responder};
use serde::Deserialize;
use std::sync::Arc;

use crate::state::AppState;
use db::Database;

const QUOTA_JOUR: i64 = 900;
/// Cap horaire mesuré ~65 — marge : 55 (le lot est borné par les deux).
const QUOTA_HEURE: i64 = 55;
const LOT_DEFAUT: usize = 50;

/// Noyau liquide prioritaire (proposition propriétaire du 31/08, curée) :
/// backfillé en premier pour que le pré-screen parle dès le jour 1.
const PRIORITAIRES: &[&str] = &[
    "AAPL", "MSFT", "NVDA", "GOOGL", "AMZN", "META", "TSLA", "AVGO", "ORCL", "ADBE", "CRM", "NOW",
    "AMD", "INTC", "MU", "QCOM", "TXN", "ADI", "MRVL", "ON", "AMAT", "LRCX", "KLAC", "ASML",
    "PLTR", "SNOW", "DDOG", "MDB", "ZS", "NET", "CRWD", "PANW", "TEAM", "WDAY", "SHOP", "UBER",
    "ABNB", "APP", "NFLX", "DIS", "SPOT", "ROKU", "PINS", "ETSY", "EBAY", "BKNG", "ZM", "MTCH",
    "V", "MA", "PYPL", "COIN", "HOOD", "AFRM", "SOFI", "NU", "INTU", "TOST",
    "WMT", "COST", "TGT", "HD", "LOW", "NKE", "SBUX", "CMG", "MCD", "LULU", "TJX", "DHI",
    "LLY", "UNH", "JNJ", "PFE", "MRK", "ABBV", "TMO", "ISRG", "VRTX", "REGN", "BIIB", "SRPT",
    "BA", "GE", "RTX", "LMT", "NOC", "CAT", "DE", "HON",
    "XOM", "CVX", "OXY", "SLB", "COP", "HAL", "VLO",
    "RIVN", "ENPH", "SEDG", "MPWR", "FCX", "NEM",
    "TMUS", "WBD", "CMCSA", "T", "VZ", "PARA",
    "SMCI", "MSTR", "IONQ", "OKLO", "RKLB", "ASTS", "RGTI", "CVNA",
    "DELL", "TER", "APH", "NTAP", "QQQ",
];

/// Date du jour (UTC) au format YYYY-MM-DD pour la clé de quota.
fn jour_utc() -> String {
    chrono::Utc::now().format("%Y-%m-%d").to_string()
}

/// Heure civile courante (epoch tronqué) pour la clé de quota horaire.
fn heure_epoch() -> i64 {
    chrono::Utc::now().timestamp() / 3600
}

/// Date de début du backfill initial : 400 jours en arrière.
fn date_debut_400j() -> String {
    (chrono::Utc::now() - chrono::Duration::days(400))
        .format("%Y-%m-%d")
        .to_string()
}

/// Quotas restants (persistés en configuration — survivent aux
/// redémarrages). Retour (restant jour, restant heure, consommées jour).
async fn quotas(db: &Database) -> (i64, i64, i64) {
    let jour = db.lire_config("tiingo_quota_jour").await.ok().flatten();
    let consomme_jour = match jour {
        Some(j) if j == jour_utc() => db
            .lire_config("tiingo_quota_n")
            .await
            .ok()
            .flatten()
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(0),
        _ => 0,
    };
    let heure = db.lire_config("tiingo_quota_heure").await.ok().flatten();
    let consomme_heure = match heure {
        Some(h) if h.parse::<i64>().ok() == Some(heure_epoch()) => db
            .lire_config("tiingo_quota_heure_n")
            .await
            .ok()
            .flatten()
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(0),
        _ => 0,
    };
    (
        QUOTA_JOUR - consomme_jour,
        QUOTA_HEURE - consomme_heure,
        consomme_jour,
    )
}

async fn consommer_quota(db: &Database, n: i64) {
    let (_, _, consomme_jour) = quotas(db).await;
    let heure = db.lire_config("tiingo_quota_heure").await.ok().flatten();
    let consomme_heure = match heure {
        Some(h) if h.parse::<i64>().ok() == Some(heure_epoch()) => db
            .lire_config("tiingo_quota_heure_n")
            .await
            .ok()
            .flatten()
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(0),
        _ => 0,
    };
    db.ecrire_config("tiingo_quota_jour", &jour_utc()).await.ok();
    db.ecrire_config("tiingo_quota_n", &(consomme_jour + n).to_string()).await.ok();
    db.ecrire_config("tiingo_quota_heure", &heure_epoch().to_string()).await.ok();
    db.ecrire_config("tiingo_quota_heure_n", &(consomme_heure + n).to_string())
        .await
        .ok();
}

/// Exécute UN lot de backfill (jusqu'à `taille` tickers, borné par le quota).
/// Retourne un compte-rendu JSON détaillé.
pub async fn backfill_lot(db: &Arc<Database>, taille: usize) -> serde_json::Value {
    let cle = match db.lire_config("tiingo_api_key").await.ok().flatten() {
        Some(c) if c.len() >= 10 => c,
        _ => {
            return serde_json::json!({ "erreur": "Clé Tiingo absente (Paramètres › Connexion/API)" })
        }
    };

    let (restant_jour, restant_heure, consomme) = quotas(db).await;
    if restant_jour <= 0 {
        return serde_json::json!({
            "erreur": "Quota Tiingo quotidien épuisé", "consommees": consomme, "quota": QUOTA_JOUR
        });
    }
    if restant_heure <= 0 {
        return serde_json::json!({
            "erreur": "Quota Tiingo horaire épuisé (limite gratuite ~65/h)",
            "consommees": consomme, "quota_heure": QUOTA_HEURE
        });
    }
    let mut budget = (taille as i64).min(restant_jour).min(restant_heure) as usize;

    let provider = data::providers::tiingo::TiingoProvider::nouveau(cle);
    let mut n_nouveaux = 0usize;
    let mut n_rafraichis = 0usize;
    let mut echecs = 0usize;
    let debut_400j = date_debut_400j();

    // Phase 1-2 : tickers sans bougies (prioritaires d'abord).
    let mut cibles = db.tickers_sans_bougies(PRIORITAIRES, budget).await.unwrap_or_default();
    if cibles.len() < budget {
        if let Some(complement) = db
            .tickers_a_rafraichir(budget.saturating_sub(cibles.len()))
            .await
            .ok()
        {
            cibles.extend(complement);
        }
    }

    for ticker in &cibles {
        if budget == 0 {
            break;
        }
        let deja_backfille = db
            .bougies_actions(ticker)
            .await
            .map(|v| !v.is_empty())
            .unwrap_or(false);
        // Récup : depuis la dernière bougie connue (−5 j de recouvrement),
        // sinon backfill initial 400 jours.
        let depuis = if deja_backfille {
            match db.bougies_actions(ticker).await.ok().and_then(|v| v.last().map(|b| b.0)) {
                Some(dernier_ts) => chrono::DateTime::from_timestamp(dernier_ts - 5 * 86400, 0)
                    .map(|d| d.format("%Y-%m-%d").to_string())
                    .unwrap_or_else(|| debut_400j.clone()),
                None => debut_400j.clone(),
            }
        } else {
            debut_400j.clone()
        };

        match provider.eod(ticker, &depuis).await {
            Ok(bougies) if !bougies.is_empty() => {
                let lignes: Vec<(i64, f64, f64, f64, f64, f64)> = bougies
                    .iter()
                    .map(|b| (b.ts, b.open, b.high, b.low, b.close, b.volume))
                    .collect();
                if db.inserer_bougies_actions(ticker, &lignes).await.is_ok() {
                    if deja_backfille {
                        n_rafraichis += 1;
                    } else {
                        n_nouveaux += 1;
                    }
                } else {
                    echecs += 1;
                }
            }
            Ok(_) => {} // ticker sans données (delisting…) — pas une erreur
            Err(_) => echecs += 1,
        }
        budget -= 1;
        // Respiration : ~4 req/s pour rester courtois avec l'API.
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }

    let utilisees = (taille as i64).min(QUOTA_HEURE) - budget as i64;
    let utilisees = utilisees.max(0);
    consommer_quota(db, utilisees).await;
    let (total, avec) = db.avancement_backfill().await.unwrap_or((0, 0));

    tracing::info!(
        "🚀 Backfill actions : {n_nouveaux} nouveaux, {n_rafraichis} rafraîchis, {echecs} échecs — univers {avec}/{total}"
    );
    serde_json::json!({
        "nouveaux": n_nouveaux,
        "rafraichis": n_rafraichis,
        "echecs": echecs,
        "requetes": utilisees,
        "quota_restant_jour": restant_jour - utilisees,
        "univers_total": total,
        "univers_avec_bougies": avec,
    })
}

/// Boucle de fond : un lot au boot, puis un lot de 50 toutes les 65 min
/// (cap horaire Tiingo ~65 req/h — plein régime = ~1 100/j borné à 900).
pub async fn boucle_backfill(db: Arc<Database>) {
    tracing::info!("🚀 Backfill actions armé (boot + lot de {LOT_DEFAUT} toutes les 65 min)");
    backfill_lot(&db, LOT_DEFAUT).await;
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(65 * 60)).await;
        backfill_lot(&db, LOT_DEFAUT).await;
    }
}

// ── Endpoints ────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct BodyBackfill {
    pub taille: Option<usize>,
}

/// POST /api/rockets/actions/backfill — exécute un lot immédiatement
/// (défaut 200 tickers, borné par le quota restant).
pub async fn post_backfill(
    state: web::Data<AppState>,
    body: Option<web::Json<BodyBackfill>>,
) -> impl Responder {
    let taille = body.and_then(|b| b.taille).unwrap_or(LOT_DEFAUT).clamp(1, 50);
    HttpResponse::Ok().json(backfill_lot(&state.db, taille).await)
}

/// GET /api/rockets/actions/backfill/etat — avancement univers + quota.
pub async fn etat_backfill(state: web::Data<AppState>) -> impl Responder {
    let (total, avec) = state.db.avancement_backfill().await.unwrap_or((0, 0));
    let (restant_jour, restant_heure, consomme) = quotas(&state.db).await;
    HttpResponse::Ok().json(serde_json::json!({
        "univers_total": total,
        "univers_avec_bougies": avec,
        "progression_pct": if total > 0 { (avec as f64 / total as f64 * 100.0).round() } else { 0.0 },
        "quota_jour": QUOTA_JOUR,
        "quota_consomme": consomme,
        "quota_restant_jour": restant_jour,
        "quota_heure": QUOTA_HEURE,
        "quota_restant_heure": restant_heure,
    }))
}
