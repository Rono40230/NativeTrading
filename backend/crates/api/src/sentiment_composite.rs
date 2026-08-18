//! Sentiment composite agrégé (Phase 1 + 2).
//!
//! Combine le sentiment technique local (`smc::v12::sentiment`) avec les
//! composantes externes reconnectées :
//!   - Fear & Greed Index (alternative.me) — crypto
//!   - VIX (Yahoo `%5EVIX`) — forex/métaux/indices
//!
//! `calculer_composite` orchestre le tout avec renormalisation des poids quand
//! une composante est indisponible (fallback technique seul). Un worker 30 min
//! rafraîchit `AppState.sentiment` et persiste un snapshot quotidien.
//!
//! Poids par classe :
//!   - Crypto   = technique×0.3 + F&G×0.4
//!   - Forex    = technique×0.3 + VIX_inversé×0.3
//!   - Métaux   = technique×0.4 + VIX_direct×0.25   (or = safe haven → VIX haussier = greed métaux)
//!   - Indices  = technique×0.35 + VIX_inversé×0.35

use std::sync::Arc;
use std::time::Duration;

use actix_web::{web, HttpResponse, Responder};
use chrono::Utc;
use common::{Asset, Candle, Timeframe};
use db::Database;
use serde_json::Value;
use smc::v12::sentiment::{agreg_par_classe, calculer_sentiment_technique, SentimentScore};
use tokio::sync::RwLock;
use tokio::time::{interval, sleep};

use crate::state::AppState;

/// Type du cache Fear & Greed partagé avec `AppState`.
pub type FgCache = Arc<RwLock<Option<(std::time::Instant, Value)>>>;

/// Type du slot sentiment partagé avec `AppState`.
pub type SentimentSlot = Arc<RwLock<Option<SentimentScore>>>;

/// Actifs principaux utilisés pour le sentiment technique D1.
/// ETH retiré du pipeline le 2026-08-18 (décision propriétaire : BTC seul).
/// Les assets Dukascopy figés sont filtrés par la garde d'ancienneté.
const ASSETS_PRINCIPAUX: &[&str] = &[
    "BTC", "EURUSD", "GBPJPY", "USDJPY", "XAUUSD", "XAGUSD", "DAX", "NAS100", "SP500",
];

/// TTL du cache F&G (1 h).
const FG_TTL_SEC: u64 = 3600;

// ── Combinaison pondérée avec renormalisation ────────────────────────────────

/// Moyenne pondérée des composantes disponibles, poids renormalisés.
///
/// Si une composante est `None`, son poids est retiré et les autres sont
/// renormalisés pour sommer à 1. Retourne `None` si aucune composante
/// n'est disponible.
fn combine_dispo(composantes: &[(Option<f64>, f64)]) -> Option<f64> {
    let (somme_pond, total_poids) = composantes
        .iter()
        .filter_map(|(val, poids)| val.map(|v| (v * poids, *poids)))
        .fold((0.0_f64, 0.0_f64), |(sp, tp), (vp, w)| (sp + vp, tp + w));
    if total_poids <= 0.0 {
        return None;
    }
    Some(somme_pond / total_poids)
}

// ── Composantes externes ─────────────────────────────────────────────────────

/// Lit le Fear & Greed depuis le cache (si < 1h) sinon fetch alternative.me.
/// Met à jour le cache partagé. Retourne la valeur 0-100 (None si indispo).
async fn lire_ou_fetcher_fg(fg_cache: &FgCache) -> Option<f64> {
    // 1. Cache < TTL.
    {
        let cache = fg_cache.read().await;
        if let Some((fetched_at, data)) = cache.as_ref() {
            if fetched_at.elapsed() < Duration::from_secs(FG_TTL_SEC) {
                return data["valeur"]
                    .as_f64()
                    .or_else(|| data["valeur"].as_str().and_then(|s| s.parse().ok()));
            }
        }
    }

    // 2. Fetch.
    let client = &*crate::http_client::HTTP_CLIENT;
    let resp = client
        .get("https://api.alternative.me/fng/?limit=1")
        .header(reqwest::header::USER_AGENT, "NativeTrading/1.0")
        .send()
        .await;
    let raw = match resp {
        Ok(r) => r.json::<Value>().await.ok(),
        Err(e) => {
            tracing::warn!("Fear&Greed fetch (worker sentiment): {e}");
            None
        }
    }?;

    let valeur = raw["data"][0]["value"]
        .as_str()
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(50.0);
    let label = raw["data"][0]["value_classification"]
        .as_str()
        .unwrap_or("Neutral")
        .to_string();
    let data = serde_json::json!({
        "valeur": valeur as u8,
        "label": label,
        "source": "alternative.me",
    });

    let mut cache = fg_cache.write().await;
    *cache = Some((std::time::Instant::now(), data));
    Some(valeur)
}

/// Fetch le VIX depuis Yahoo (`%5EVIX`).
///
/// Retourne `(vix_brut, vix_inversé)` où `vix_inversé` = `100 - (vix-10)*2.5`
/// (VIX 10 = 100 greed, VIX 50 = 0 fear). `(None, None)` si indispo.
async fn fetch_vix() -> (Option<f64>, Option<f64>) {
    let client = &*crate::http_client::HTTP_CLIENT;
    let url = "https://query2.finance.yahoo.com/v8/finance/chart/%5EVIX?interval=1d&range=2d";
    let resp = client
        .get(url)
        .header("User-Agent", "Mozilla/5.0 (X11; Linux x86_64)")
        .send()
        .await;
    let raw = match resp {
        Ok(r) => match r.json::<Value>().await {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!("VIX parse (worker sentiment): {e}");
                return (None, None);
            }
        },
        Err(e) => {
            tracing::warn!("VIX fetch (worker sentiment): {e}");
            return (None, None);
        }
    };

    let prix = raw["chart"]["result"][0]["meta"]["regularMarketPrice"].as_f64();
    match prix {
        Some(v) => {
            // VIX inversé : VIX 10 = 100 (greed), VIX 30 = 50, VIX 50 = 0 (fear).
            let inverse = (100.0 - (v - 10.0) * 2.5).clamp(0.0, 100.0);
            (Some(v), Some(inverse))
        }
        None => (None, None),
    }
}

// ── Cœur : calcul du composite ───────────────────────────────────────────────

/// Calcule le sentiment composite complet.
///
/// Étapes :
///   1. Fetch D1 bougies des actifs principaux.
///   2. Sentiment technique (RSI14 + MA20) puis agrégation par classe.
///   3. Lecture/fetch Fear & Greed.
///   4. Fetch VIX (inversé pour risk assets, direct pour métaux).
///   5. Combinaison pondérée par classe (renormalisée si composante absente).
///   6. Recalcul du global (moyenne des classes disponibles).
pub async fn calculer_composite(db: &Database, fg_cache: &FgCache) -> SentimentScore {
    // 1. Fetch D1 bougies — garde d'ancienneté : un asset dont la dernière
    //    bougie D1 date de plus de 4 jours (assets Dukascopy figés depuis
    //    avril — collecteur en phase 5) est IGNORÉ : son RSI périmé
    //    fausserait les jauges. `combine_dispo` renormalise sans lui.
    let maintenant = Utc::now();
    let mut bougies_d1: Vec<(String, Vec<Candle>)> = Vec::with_capacity(ASSETS_PRINCIPAUX.len());
    for asset_str in ASSETS_PRINCIPAUX {
        let asset = match Asset::try_from(*asset_str) {
            Ok(a) => a,
            Err(_) => continue,
        };
        match db.obtenir_bougies(&asset, &Timeframe::D1, 30).await {
            Ok(b) if b.len() >= 20 => {
                let derniere = b.last().map(|c| c.timestamp).unwrap_or_default();
                if crate::sentiment_handlers::trop_ancienne(derniere, maintenant) {
                    tracing::warn!(
                        "composite : {asset_str} D1 figé ({}) — ignoré du score technique",
                        derniere.format("%Y-%m-%d")
                    );
                    continue;
                }
                bougies_d1.push((asset_str.to_string(), b));
            }
            _ => {}
        }
    }

    // 2. Sentiment technique + agrégation par classe.
    let tech_scores = calculer_sentiment_technique(&bougies_d1);
    let mut score = agreg_par_classe(&tech_scores);

    // 3. Fear & Greed.
    let fg = lire_ou_fetcher_fg(fg_cache).await;
    score.fear_greed = fg;

    // 4. VIX.
    let (vix_brut, vix_inverse) = fetch_vix().await;
    score.vix_brut = vix_brut;
    score.vix_score = vix_inverse;

    // 5. Combinaison pondérée par classe.
    let vix_direct = vix_inverse.map(|inv| 100.0 - inv); // métaux : VIX haut = greed (safe haven)

    score.crypto = combine_dispo(&[
        (score.crypto, 0.30),
        (fg, 0.40),
    ]);
    score.forex = combine_dispo(&[
        (score.forex, 0.30),
        (vix_inverse, 0.30),
    ]);
    score.metaux = combine_dispo(&[
        (score.metaux, 0.40),
        (vix_direct, 0.25),
    ]);
    score.indices = combine_dispo(&[
        (score.indices, 0.35),
        (vix_inverse, 0.35),
    ]);

    // 6. Global = moyenne des classes disponibles.
    let classes: Vec<f64> = [score.crypto, score.forex, score.metaux, score.indices]
        .into_iter()
        .flatten()
        .collect();
    score.global = if classes.is_empty() {
        None
    } else {
        Some(classes.iter().sum::<f64>() / classes.len() as f64)
    };

    score
}

// ── Persistance snapshot quotidien ───────────────────────────────────────────

/// Insère un snapshot PAR CYCLE 30 min (5 classes + global) — matière première
/// de la « moyenne de la veille » : le composite servi au jour J est la
/// moyenne de tous les snapshots du J-1 (décision propriétaire 2026-08-17 :
/// référence veille, pas flux tendu).
async fn persister_snapshot_quotidien(db: &Database, score: &SentimentScore) {
    let today = Utc::now().format("%Y-%m-%d").to_string();

    let composantes = serde_json::json!({
        "rsi_btc": score.rsi_btc,
        "rsi_eth": score.rsi_eth,
        "rsi_xau": score.rsi_xau,
        "breadth_pct": score.breadth_pct,
        "fear_greed": score.fear_greed,
        "vix_score": score.vix_score,
        "vix_brut": score.vix_brut,
    })
    .to_string();

    let insert = |classe: &str, val: Option<f64>| {
        let db_pool = db.pool();
        let classe = classe.to_string();
        let composantes = composantes.clone();
        let date = today.clone();
        async move {
            if let Some(v) = val {
                if let Err(e) = sqlx::query(
                    "INSERT INTO sentiment_historique (date, classe, score, composantes)
                     VALUES (?, ?, ?, ?)",
                )
                .bind(&date)
                .bind(&classe)
                .bind(v)
                .bind(&composantes)
                .execute(db_pool)
                .await
                {
                    tracing::warn!("snapshot sentiment {classe}: {e}");
                }
            }
        }
    };

    insert("global", score.global).await;
    insert("crypto", score.crypto).await;
    insert("forex", score.forex).await;
    insert("metaux", score.metaux).await;
    insert("indices", score.indices).await;
}

// ── Worker 30 min ────────────────────────────────────────────────────────────

/// Démarre le worker de sentiment composite en arrière-plan (cycle 30 min).
///
/// Calcule `calculer_composite`, stocke dans `AppState.sentiment`, et persiste
/// un snapshot quotidien dans `sentiment_historique`. Ne bloque pas.
pub fn demarrer_worker_sentiment(db: Arc<Database>, sentiment: SentimentSlot, fg_cache: FgCache) {
    tokio::spawn(async move {
        // Délai initial : laisse les sources externes et la DB se stabiliser.
        sleep(Duration::from_secs(180)).await;
        let mut tick = interval(Duration::from_secs(1800)); // 30 min
        loop {
            tick.tick().await;
            let sc = calculer_composite(&db, &fg_cache).await;
            persister_snapshot_quotidien(&db, &sc).await;

            // Référence veille des listes du bloc (décision 2026-08-18) :
            // clôtures J-1 figées, idempotent — aucun flux tendu servi au front.
            let figees = crate::sentiment_handlers::figer_veille_marche(&db).await;

            *sentiment.write().await = Some(sc.clone());
            tracing::info!(
                "📊 Sentiment composite MAJ — global={:.0} crypto={:.0} forex={:.0} metaux={:.0} indices={:.0} | F&G={:?} VIX={:?} | veille figée : {}/13 entités",
                sc.global.unwrap_or(50.0),
                sc.crypto.unwrap_or(0.0),
                sc.forex.unwrap_or(0.0),
                sc.metaux.unwrap_or(0.0),
                sc.indices.unwrap_or(0.0),
                sc.fear_greed,
                sc.vix_brut,
                figees,
            );
        }
    });
    tracing::info!("📊 Worker sentiment composite démarré (cycle 30 min)");
}

// ── Endpoint HTTP ────────────────────────────────────────────────────────────

/// GET /api/sentiment/composite
///
/// Retourne le `SentimentScore` courant (refresh 30 min par le worker).
/// Si pas encore calculé → score neutre (50 partout).
/// Moyenne de la VEILLE par classe depuis sentiment_historique — la
/// référence du jour J, figée avant l'ouverture (décision propriétaire :
/// prior stable, pas de flip-flop intraday). Fallback : le live si la
/// veille n'a aucun snapshot (premier jour de fonctionnement).
pub async fn get_sentiment_composite(state: web::Data<AppState>) -> impl Responder {
    let hier = (Utc::now() - chrono::Duration::days(1))
        .format("%Y-%m-%d")
        .to_string();

    let lignes = sqlx::query_as::<_, (String, f64, Option<String>)>(
        "SELECT classe, score, composantes FROM sentiment_historique WHERE date = ?",
    )
    .bind(&hier)
    .fetch_all(state.db.pool())
    .await
    .unwrap_or_default();

    if !lignes.is_empty() {
        // Agrégation par classe : moyenne des scores + moyenne des
        // composantes numériques du JSON (rsi, fg, vix…).
        use std::collections::HashMap;
        let mut scores: HashMap<String, Vec<f64>> = HashMap::new();
        let mut composantes: Vec<serde_json::Value> = Vec::new();
        for (classe, score, comp) in &lignes {
            scores.entry(classe.clone()).or_default().push(*score);
            if let Some(c) = comp {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(c) {
                    composantes.push(v);
                }
            }
        }
        let moyenne = |cle: &str| -> Option<f64> {
            let vals: Vec<f64> = composantes
                .iter()
                .filter_map(|c| c.get(cle).and_then(|x| x.as_f64()))
                .collect();
            if vals.is_empty() { None } else { Some(vals.iter().sum::<f64>() / vals.len() as f64) }
        };
        let moy_classe = |cle: &str| -> Option<f64> {
            scores.get(cle).map(|v| v.iter().sum::<f64>() / v.len() as f64)
        };
        return HttpResponse::Ok().json(SentimentScore {
            global: moy_classe("global"),
            crypto: moy_classe("crypto"),
            forex: moy_classe("forex"),
            metaux: moy_classe("metaux"),
            indices: moy_classe("indices"),
            rsi_btc: moyenne("rsi_btc"),
            rsi_eth: moyenne("rsi_eth"),
            rsi_xau: moyenne("rsi_xau"),
            breadth_pct: moyenne("breadth_pct"),
            fear_greed: moyenne("fear_greed"),
            vix_score: moyenne("vix_score"),
            vix_brut: moyenne("vix_brut"),
        });
    }

    // Fallback : composite live (veille sans données — premier jour).
    let snap = state.sentiment.read().await.clone();
    match snap {
        Some(sc) => HttpResponse::Ok().json(sc),
        None => HttpResponse::Ok().json(SentimentScore {
            global: Some(50.0),
            crypto: Some(50.0),
            forex: Some(50.0),
            metaux: Some(50.0),
            indices: Some(50.0),
            ..Default::default()
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn combine_dispo_toutes_composantes() {
        // (tech=60, 0.3) + (fg=80, 0.4) → (18 + 32) / 0.7 = 50/0.7 = 71.43
        let r = combine_dispo(&[(Some(60.0), 0.3), (Some(80.0), 0.4)]).unwrap();
        assert!((r - 71.428).abs() < 0.01, "r={}", r);
    }

    #[test]
    fn combine_dispo_une_composante_absente_renormalise() {
        // fg absent → (60*0.3) / 0.3 = 60 (technique seul renormalisé)
        let r = combine_dispo(&[(Some(60.0), 0.3), (None, 0.4)]).unwrap();
        assert!((r - 60.0).abs() < 1e-9, "fallback technique renormalisé, eu {}", r);
    }

    #[test]
    fn combine_dispo_aucune_composante_none() {
        assert!(combine_dispo(&[(None, 0.3), (None, 0.4)]).is_none());
    }

    #[test]
    fn combine_dispo_seule_composante_non_nulle() {
        // Seul F&G dispo → 80*0.4/0.4 = 80
        let r = combine_dispo(&[(None, 0.3), (Some(80.0), 0.4)]).unwrap();
        assert!((r - 80.0).abs() < 1e-9);
    }
}
