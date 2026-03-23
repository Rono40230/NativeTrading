use actix_web::{web, HttpResponse, Responder};
use db::rockets::{self, NouveauRocket};
use serde::Deserialize;
use std::time::Duration;

use crate::rockets_scan;
use crate::state::AppState;

// ── Config endpoints ─────────────────────────────────────────────────────────

/// GET /api/rockets/config
pub async fn get_config(state: web::Data<AppState>) -> impl Responder {
    let cfg = rockets::lire_config(state.db.pool()).await;
    HttpResponse::Ok().json(cfg)
}

/// PUT /api/rockets/config
pub async fn put_config(
    state: web::Data<AppState>,
    body: web::Json<rockets::RocketsConfig>,
) -> impl Responder {
    match rockets::sauvegarder_config(state.db.pool(), &body).await {
        Ok(()) => HttpResponse::Ok().json(serde_json::json!({ "ok": true })),
        Err(e) => HttpResponse::InternalServerError()
            .json(serde_json::json!({ "error": e.to_string() })),
    }
}

// ── DTOs ────────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct RequeteSauvegarder {
    pub ticker: String,
    pub phase: String,
    pub score: i64,
    pub prix_entree: f64,
    pub stop_loss: f64,
    pub target: f64,
    pub target2: Option<f64>,
    pub target3: Option<f64>,
    pub ratio_volume: f64,
    pub atr_ratio: f64,
    pub atr14: Option<f64>,
    pub rsi: f64,
}

#[derive(Deserialize)]
pub struct QueryHistorique {
    pub limite: Option<i64>,
}

// ── Handlers ─────────────────────────────────────────────────────────────────

/// POST /api/rockets/signal — enregistre un signal détecté côté frontend
pub async fn sauvegarder_signal(
    state: web::Data<AppState>,
    body: web::Json<RequeteSauvegarder>,
) -> impl Responder {
    let pool = state.db.pool();
    let nouveau = NouveauRocket {
        ticker: body.ticker.clone(),
        phase: body.phase.clone(),
        score: body.score,
        prix_entree: body.prix_entree,
        stop_loss: body.stop_loss,
        target: body.target,
        target2: body.target2,
        target3: body.target3,
        ratio_volume: body.ratio_volume,
        atr_ratio: body.atr_ratio,
        atr14: body.atr14,
        rsi: body.rsi,
        llm_valide: None,
        llm_conviction: None,
        llm_raison: None,
        llm_sl_suggere: None,
        llm_tp1_suggere: None,
    };
    match rockets::sauvegarder(pool, &nouveau).await {
        Ok(Some(id)) => HttpResponse::Ok().json(serde_json::json!({ "id": id, "nouveau": true })),
        Ok(None) => HttpResponse::Ok().json(serde_json::json!({ "nouveau": false })),
        Err(e) => {
            tracing::error!("Sauvegarde rocket: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({ "error": e.to_string() }))
        }
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ScanReponse<'a> {
    signaux: &'a [rockets_scan::ScanResultat],
    total_candidats: usize,
}

/// GET /api/rockets/scan — résultats du dernier scan worker
pub async fn get_scan() -> impl Responder {
    let results = rockets_scan::get_scan_results();
    let total = rockets_scan::get_total_candidats();
    let locked = results.read().await;
    let nb_total = *total.read().await;
    HttpResponse::Ok().json(ScanReponse {
        signaux: &locked,
        total_candidats: nb_total,
    })
}

/// GET /api/rockets/historique?limite=50
pub async fn get_historique(
    state: web::Data<AppState>,
    query: web::Query<QueryHistorique>,
) -> impl Responder {
    let pool = state.db.pool();
    let limite = query.limite.unwrap_or(50);
    match rockets::historique(pool, limite).await {
        Ok(liste) => HttpResponse::Ok().json(liste),
        Err(e) => {
            tracing::error!("Historique rockets: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({ "error": e.to_string() }))
        }
    }
}

/// POST /api/rockets/sync — force un cycle de suivi immédiat (SL/TP attente + ouverts)
pub async fn sync_verdicts(state: web::Data<AppState>) -> impl Responder {
    let pool = state.db.pool();
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": format!("HTTP client: {}", e) }))
        }
    };

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

    // Ouverts : SL/TP
    if let Ok(signaux) = rockets::lister_ouverts(pool).await {
        for s in &signaux {
            let Some(prix) = fetch_prix(&client, &s.ticker).await else {
                continue;
            };
            let peak = s.prix_peak.unwrap_or(s.prix_entree).max(prix);
            if peak > s.prix_peak.unwrap_or(0.0) {
                let _ = rockets::maj_prix_peak(pool, s.id, peak).await;
            }
            if let Some(v) = calculer_verdict_rocket(s, prix, peak) {
                if rockets::maj_verdict(pool, s.id, v, prix).await.is_ok() {
                    fermes += 1;
                }
            }
        }
    }

    HttpResponse::Ok().json(serde_json::json!({
        "fermes": fermes,
        "ouverts_nouveaux": ouverts_nouveaux
    }))
}

// ── Worker de suivi ──────────────────────────────────────────────────────────

#[derive(serde::Deserialize)]
struct BinancePrix {
    price: String,
}

async fn fetch_prix(client: &reqwest::Client, ticker: &str) -> Option<f64> {
    let url = format!(
        "https://api.binance.com/api/v3/ticker/price?symbol={}USDT",
        ticker
    );
    let resp: BinancePrix = client.get(&url).send().await.ok()?.json().await.ok()?;
    resp.price.parse::<f64>().ok()
}

/// Worker lancé au démarrage : toutes les 15min, gère cycle de vie complet.
pub async fn demarrer_worker_suivi(pool: sqlx::SqlitePool) {
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Worker rockets HTTP: {}", e);
            return;
        }
    };

    loop {
        tokio::time::sleep(Duration::from_secs(3 * 60)).await;

        // 1. Expirer les signaux EN ATTENTE depuis >6h (position jamais ouverte)
        if let Ok(n) = rockets::marquer_expires(&pool).await {
            if n > 0 {
                tracing::info!("Rockets: {} signal(s) expirés (jamais entrés)", n);
            }
        }

        // 2. Signaux en attente : SL touché avant entrée → invalide, sinon ouvrir si prix atteint
        let en_attente = match rockets::lister_en_attente(&pool).await {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!("Worker rockets attente: {}", e);
                continue;
            }
        };
        for s in &en_attente {
            let Some(prix) = fetch_prix(&client, &s.ticker).await else {
                continue;
            };
            if prix <= s.stop_loss {
                // SL touché avant même d'entrer en position → clôturer en perte
                if let Err(e) = rockets::maj_verdict(&pool, s.id, "invalide", prix).await {
                    tracing::warn!("Rocket {} SL avant entrée: {}", s.ticker, e);
                } else {
                    tracing::info!(
                        "Rocket {} → invalide (SL avant entrée) @ {:.5}",
                        s.ticker,
                        prix
                    );
                }
            } else if prix >= s.prix_entree {
                if let Err(e) = rockets::entrer_position(&pool, s.id).await {
                    tracing::warn!("Rocket {} entrée position: {}", s.ticker, e);
                } else {
                    tracing::info!("Rocket {} → ouvert @ {:.5}", s.ticker, prix);
                }
            }
        }

        // 3. Signaux OUVERTS : TP pyramidal + trailing TP3 + SL
        let signaux = match rockets::lister_ouverts(&pool).await {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!("Worker rockets ouverts: {}", e);
                continue;
            }
        };
        for s in &signaux {
            let Some(prix) = fetch_prix(&client, &s.ticker).await else {
                continue;
            };

            // Mettre à jour le prix peak
            let peak = s.prix_peak.unwrap_or(s.prix_entree).max(prix);
            if peak > s.prix_peak.unwrap_or(0.0) {
                if let Err(e) = rockets::maj_prix_peak(&pool, s.id, peak).await {
                    tracing::warn!("Rocket {} maj peak: {}", s.ticker, e);
                }
            }

            let verdict = calculer_verdict_rocket(s, prix, peak);
            if let Some(v) = verdict {
                if let Err(e) = rockets::maj_verdict(&pool, s.id, v, prix).await {
                    tracing::warn!("Worker rockets verdict: {}", e);
                } else {
                    tracing::info!("Rocket {} → {} @ {:.5}", s.ticker, v, prix);
                }
            }
        }
    }
}

fn calculer_verdict_rocket(
    s: &db::rockets::RocketSignal,
    prix: f64,
    peak: f64,
) -> Option<&'static str> {
    let atr14 = s.atr14.unwrap_or(s.prix_entree * 0.01);
    let trailing_stop = peak - atr14 * 1.5;

    // SL effectif progressif selon le niveau TP atteint (break-even)
    let sl_effectif = match (s.target2, s.target3) {
        (Some(_tp2), Some(tp3)) if peak >= tp3 => {
            // TP3 en route : trailing stop
            return if prix <= trailing_stop {
                Some("TP3")
            } else {
                None
            };
        }
        (Some(tp2), _) if peak >= tp2 => s.target, // BE = TP1
        _ if peak >= s.target => s.prix_entree,    // BE = entrée
        _ => s.stop_loss,                          // SL original
    };

    if prix <= sl_effectif {
        return Some("invalide");
    }
    // TP2 : fermeture immédiate si prix >= TP2 et pas encore en zone TP3
    if let Some(tp2) = s.target2 {
        if prix >= tp2 {
            return Some("TP2");
        }
    }
    // TP1 : fermeture uniquement si pas de TP2 (sinon on attend TP2, SL=BE)
    if prix >= s.target && s.target2.is_none() {
        return Some("TP1");
    }
    None
}

// ── Tests unitaires — progression position ───────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use db::rockets::RocketSignal;

    /// Construit un signal de test minimal
    fn signal(entree: f64, sl: f64, tp1: f64, tp2: Option<f64>, tp3: Option<f64>, atr14: f64) -> RocketSignal {
        RocketSignal {
            id: 1,
            ticker: "TEST".into(),
            phase: "breakout".into(),
            score: 75,
            prix_entree: entree,
            stop_loss: sl,
            target: tp1,
            target2: tp2,
            target3: tp3,
            ratio_volume: 2.0,
            atr_ratio: 1.5,
            atr14: Some(atr14),
            rsi: 60.0,
            statut: "ouvert".into(),
            prix_peak: None,
            verdict: None,
            prix_verdict: None,
            cree_le: "2026-01-01T00:00:00".into(),
            maj_le: None,
        }
    }

    // ── Scénario 1 : position simple (TP1 uniquement) ────────────────────────

    #[test]
    fn entre_prix_neutre_aucun_verdict() {
        let s = signal(1.0, 0.90, 1.10, None, None, 0.05);
        assert_eq!(calculer_verdict_rocket(&s, 1.0, 1.0), None);
    }

    #[test]
    fn sl_touche_invalide() {
        let s = signal(1.0, 0.90, 1.10, None, None, 0.05);
        assert_eq!(calculer_verdict_rocket(&s, 0.89, 0.89), Some("invalide"));
    }

    #[test]
    fn tp1_atteint_sans_tp2_fermeture() {
        let s = signal(1.0, 0.90, 1.10, None, None, 0.05);
        assert_eq!(calculer_verdict_rocket(&s, 1.10, 1.10), Some("TP1"));
    }

    // ── Scénario 2 : break-even après TP1 ───────────────────────────────────

    #[test]
    fn prix_sur_tp1_avec_tp2_pas_de_fermeture() {
        // TP2 existe → TP1 ne ferme pas, SL monte au break-even
        let s = signal(1.0, 0.90, 1.10, Some(1.20), None, 0.05);
        // peak atteint TP1 mais pas encore TP2
        assert_eq!(calculer_verdict_rocket(&s, 1.10, 1.10), None);
    }

    #[test]
    fn retour_breakeven_apres_tp1_invalide() {
        // Après que le peak a dépassé TP1, SL effectif = prix_entree (1.0)
        // Si prix retombe à l'entrée → invalide
        let s = signal(1.0, 0.90, 1.10, Some(1.20), None, 0.05);
        let peak = 1.12; // TP1 dépassé
        assert_eq!(calculer_verdict_rocket(&s, 1.0, peak), Some("invalide"));
    }

    #[test]
    fn retour_sous_breakeven_apres_tp1_invalide() {
        // SL effectif = entrée (1.0), prix à 0.95 → invalide
        let s = signal(1.0, 0.90, 1.10, Some(1.20), None, 0.05);
        let peak = 1.12;
        assert_eq!(calculer_verdict_rocket(&s, 0.95, peak), Some("invalide"));
    }

    // ── Scénario 3 : progression TP2, SL monte à TP1 ────────────────────────

    #[test]
    fn tp2_atteint_fermeture() {
        let s = signal(1.0, 0.90, 1.10, Some(1.20), Some(1.50), 0.05);
        let peak = 1.20;
        assert_eq!(calculer_verdict_rocket(&s, 1.20, peak), Some("TP2"));
    }

    #[test]
    fn retour_tp1_apres_tp2_invalide() {
        // SL effectif = TP1 (1.10) après que peak >= TP2
        let s = signal(1.0, 0.90, 1.10, Some(1.20), Some(1.50), 0.05);
        let peak = 1.25; // TP2 dépassé
        assert_eq!(calculer_verdict_rocket(&s, 1.10, peak), Some("invalide"));
    }

    #[test]
    fn entre_tp1_et_tp2_apres_tp2_depasse_aucun_verdict() {
        // Peak a dépassé TP2, prix en retrait mais au-dessus de TP1
        let s = signal(1.0, 0.90, 1.10, Some(1.20), Some(1.50), 0.05);
        let peak = 1.25;
        assert_eq!(calculer_verdict_rocket(&s, 1.15, peak), None);
    }

    // ── Scénario 4 : trailing stop TP3 ──────────────────────────────────────

    #[test]
    fn tp3_zone_trailing_stop_non_touche() {
        // peak >= TP3, prix encore au-dessus du trailing
        let s = signal(1.0, 0.90, 1.10, Some(1.20), Some(1.50), 0.05);
        let peak = 1.60;
        let trailing = peak - 0.05 * 1.5; // 1.60 - 0.075 = 1.525
        assert_eq!(calculer_verdict_rocket(&s, trailing + 0.01, peak), None);
    }

    #[test]
    fn tp3_zone_trailing_stop_touche() {
        // prix <= trailing stop → clôture "TP3"
        let s = signal(1.0, 0.90, 1.10, Some(1.20), Some(1.50), 0.05);
        let peak = 1.60;
        let trailing = peak - 0.05 * 1.5; // 1.525
        assert_eq!(calculer_verdict_rocket(&s, trailing - 0.001, peak), Some("TP3"));
    }
}

