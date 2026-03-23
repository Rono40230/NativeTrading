use actix_web::{web, HttpResponse, Responder};
use db::rockets;
use std::time::Duration;

use crate::state::AppState;

pub use crate::rockets_indicateurs::calculer_verdict_rocket;

// ── Helpers HTTP ─────────────────────────────────────────────────────────────

#[derive(serde::Deserialize)]
struct BinancePrix {
    price: String,
}

pub async fn fetch_prix(client: &reqwest::Client, ticker: &str) -> Option<f64> {
    let url = format!(
        "https://api.binance.com/api/v3/ticker/price?symbol={}USDT",
        ticker
    );
    let resp: BinancePrix = client.get(&url).send().await.ok()?.json().await.ok()?;
    resp.price.parse::<f64>().ok()
}

// ── POST /api/rockets/sync ───────────────────────────────────────────────────

/// Force un cycle de suivi immédiat (SL/TP attente + ouverts)
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

/// Worker lancé au démarrage : toutes les 3min, gère cycle de vie complet.
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
        let s = signal(1.0, 0.90, 1.10, Some(1.20), None, 0.05);
        assert_eq!(calculer_verdict_rocket(&s, 1.10, 1.10), None);
    }

    #[test]
    fn retour_breakeven_apres_tp1_invalide() {
        let s = signal(1.0, 0.90, 1.10, Some(1.20), None, 0.05);
        let peak = 1.12;
        assert_eq!(calculer_verdict_rocket(&s, 1.0, peak), Some("invalide"));
    }

    #[test]
    fn retour_sous_breakeven_apres_tp1_invalide() {
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
        let s = signal(1.0, 0.90, 1.10, Some(1.20), Some(1.50), 0.05);
        let peak = 1.25;
        assert_eq!(calculer_verdict_rocket(&s, 1.10, peak), Some("invalide"));
    }

    #[test]
    fn entre_tp1_et_tp2_apres_tp2_depasse_aucun_verdict() {
        let s = signal(1.0, 0.90, 1.10, Some(1.20), Some(1.50), 0.05);
        let peak = 1.25;
        assert_eq!(calculer_verdict_rocket(&s, 1.15, peak), None);
    }

    // ── Scénario 4 : trailing stop TP3 ──────────────────────────────────────

    #[test]
    fn tp3_zone_trailing_stop_non_touche() {
        let s = signal(1.0, 0.90, 1.10, Some(1.20), Some(1.50), 0.05);
        let peak = 1.60;
        let trailing = peak - 0.05 * 1.5;
        assert_eq!(calculer_verdict_rocket(&s, trailing + 0.01, peak), None);
    }

    #[test]
    fn tp3_zone_trailing_stop_touche() {
        let s = signal(1.0, 0.90, 1.10, Some(1.20), Some(1.50), 0.05);
        let peak = 1.60;
        let trailing = peak - 0.05 * 1.5;
        assert_eq!(calculer_verdict_rocket(&s, trailing - 0.001, peak), Some("TP3"));
    }
}
