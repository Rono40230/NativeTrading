use crate::rockets_analyse::analyser_symbol;
use crate::rockets_sauvegarder::{calculer_niveaux, filtrer_sauvegarder_publier};
use db::rockets;
use futures_util::future::join_all;
use ml::PipelineML;
use std::sync::{Arc, OnceLock};
use std::time::Duration;
pub use strategies::rockets_indicateurs::ScanResultat;
use strategies::rockets_indicateurs::{
    est_eligible, phase_priorite, Ticker24h, BATCH_SIZE, MAX_DISPLAY, SCAN_SECS,
};
use tokio::sync::RwLock;

// ── État partagé (lecture depuis le handler HTTP) ────────────────────────────

static SCAN_RESULTS: OnceLock<Arc<RwLock<Vec<ScanResultat>>>> = OnceLock::new();
static TOTAL_CANDIDATS: OnceLock<Arc<RwLock<usize>>> = OnceLock::new();

pub fn get_scan_results() -> Arc<RwLock<Vec<ScanResultat>>> {
    SCAN_RESULTS
        .get_or_init(|| Arc::new(RwLock::new(vec![])))
        .clone()
}

pub fn get_total_candidats() -> Arc<RwLock<usize>> {
    TOTAL_CANDIDATS
        .get_or_init(|| Arc::new(RwLock::new(0)))
        .clone()
}

// ── Worker de scan ───────────────────────────────────────────────────────────

pub async fn demarrer_worker_scan(
    pool: sqlx::SqlitePool,
    pipeline_ml: Arc<RwLock<PipelineML>>,
) {
    let client = &*crate::http_client::HTTP_CLIENT;

    loop {
        if let Err(e) = executer_scan(&client, &pool, &pipeline_ml).await {
            tracing::warn!("Scan rockets erreur: {}", e);
        }
        tokio::time::sleep(Duration::from_secs(SCAN_SECS)).await;
    }
}

async fn executer_scan(
    client: &reqwest::Client,
    pool: &sqlx::SqlitePool,
    pipeline_ml: &Arc<RwLock<PipelineML>>,
) -> anyhow::Result<()> {
    use anyhow::Context;

    // Lire la config depuis la DB (paramètres ajustables par l'utilisateur)
    let cfg = rockets::lire_config(pool).await;
    tracing::debug!(
        "Config scan: score_min={} rsi_max={} ratio_vol_min={} phases={:?}",
        cfg.score_min,
        cfg.rsi_max,
        cfg.ratio_volume_min,
        cfg.phases_actives
    );

    // Binance response: Vec<{ symbol, quoteVolume, ... }>
    #[derive(serde::Deserialize)]
    struct BinanceTicker {
        symbol: String,
        #[serde(rename = "quoteVolume")]
        quote_volume: String,
    }

    let tickers: Vec<Ticker24h> = client
        .get("https://api.binance.com/api/v3/ticker/24hr")
        .send()
        .await
        .context("fetch ticker Binance")?
        .json::<Vec<BinanceTicker>>()
        .await
        .context("parse ticker Binance")?
        .into_iter()
        .map(|item| Ticker24h {
            symbol: item.symbol,
            quote_volume: item.quote_volume,
        })
        .collect();

    let vol_min = cfg.vol_marche_min;
    let candidats_bruts: Vec<String> = tickers
        .into_iter()
        .filter(|t| {
            let vol = t.quote_volume.parse::<f64>().unwrap_or(0.0);
            est_eligible(&t.symbol, vol, vol_min)
        })
        .map(|t| t.symbol[..t.symbol.len() - 4].to_string())
        .collect();

    // Filtrer les tickers blacklistés (prix figé, doublons permanents, etc.)
    let mut candidats: Vec<String> = Vec::with_capacity(candidats_bruts.len());
    for ticker in candidats_bruts {
        let blackliste: bool = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM rockets_blacklist WHERE ticker = ?",
        )
        .bind(&ticker)
        .fetch_one(pool)
        .await
        .map(|n| n > 0)
        .unwrap_or(false);
        if blackliste {
            tracing::debug!("Scan rockets: {} ignoré (blacklist)", ticker);
        } else {
            candidats.push(ticker);
        }
    }

    tracing::debug!("Scan rockets: {} candidats", candidats.len());
    *get_total_candidats().write().await = candidats.len();

    let mut resultats: Vec<ScanResultat> = Vec::new();
    for batch in candidats.chunks(BATCH_SIZE) {
        let futs = batch
            .iter()
            .map(|ticker| analyser_symbol(client, ticker.as_str(), &cfg));
        let res = join_all(futs).await;
        resultats.extend(res.into_iter().flatten());
    }

    resultats.sort_by(|a, b| {
        phase_priorite(&b.phase)
            .cmp(&phase_priorite(&a.phase))
            .then(b.score.cmp(&a.score))
    });
    // NB: on ne tronque PAS ici — le cache garde tous les résultats
    // MAX_DISPLAY est appliqué dans get_scan() au moment de servir l'UI

    // ── Passe principale : breakout / pré-lancement ──────────────────────────
    // Lire le seuil de confiance ML une seule fois (table configuration)
    let seuil_rockets: f64 = sqlx::query_scalar(
        "SELECT valeur FROM configuration WHERE cle = 'seuil_confiance_rockets'",
    )
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .and_then(|v: String| v.parse().ok())
    .unwrap_or(0.60);

    for r in resultats.iter().filter(|r| {
        cfg.phases_actives.contains(&r.phase)
            && r.score >= cfg.score_min
            && r.rsi <= cfg.rsi_max
            && r.rsi >= cfg.rsi_min
            && r.ratio_volume >= cfg.ratio_volume_min
            && r.ratio_corps >= 0.35
    }) {
        if ml_rejette_rocket(r, pipeline_ml, seuil_rockets).await {
            continue;
        }
        let niveaux = calculer_niveaux(r, &cfg);
        filtrer_sauvegarder_publier(r, &niveaux, &r.phase, "Rockets", pool, &cfg).await;
    }

    // ── Passe "Confirmé Momentum" : compression avec élan 1h ─────────────────
    const CHANGE_1H_MOMENTUM_MIN: f64 = 0.5;
    const SCORE_MOMENTUM_MIN: i64 = 15;

    for r in resultats.iter().filter(|r| {
        r.phase == "compression"
            && r.change1h >= CHANGE_1H_MOMENTUM_MIN
            && r.score >= SCORE_MOMENTUM_MIN
            && r.rsi <= cfg.rsi_max
    }) {
        if ml_rejette_rocket(r, pipeline_ml, seuil_rockets).await {
            continue;
        }
        let niveaux = calculer_niveaux(r, &cfg);
        filtrer_sauvegarder_publier(
            r,
            &niveaux,
            "momentum-compression",
            "Rockets-Momentum",
            pool,
            &cfg,
        )
        .await;
    }

    let n = resultats.len();
    *get_scan_results().write().await = resultats; // cache complet (non tronqué)
    tracing::debug!(
        "Scan rockets terminé: {} résultats en cache ({} max affichés UI)",
        n,
        MAX_DISPLAY
    );
    Ok(())
}

/// Retourne `true` si le modèle XGB Rockets fine-tuné prédit un score TP insuffisant.
/// Si le modèle n'est pas encore entraîné (< 50 trades) → laisse passer (false).
async fn ml_rejette_rocket(
    r: &ScanResultat,
    pipeline_ml: &Arc<RwLock<PipelineML>>,
    seuil: f64,
) -> bool {
    let features = match &r.features_ml {
        Some(f) => f,
        None => return false, // pas assez de bougies pour extraire les features
    };
    // Guard : features corrompues (NaN/Inf) → rejeter SANS acquérir le lock.
    // Évite les freezes sur assets à prix figé (ex. FRONT).
    if ml::features_corrompues(features) {
        tracing::warn!("ML Rockets: features corrompues pour {} — signal rejeté sans lock", r.ticker);
        return true;
    }
    let ml = pipeline_ml.read().await;
    if !ml.xgb_rockets.est_pret() {
        return false; //模型 pas encore entraîné, on laisse passer
    }
    match ml.xgb_rockets.predire_score(features) {
        Some(score) if score < seuil => {
            tracing::debug!(
                "ML Rockets rejette {} (score_tp={:.2} < seuil={:.2})",
                r.ticker, score, seuil
            );
            true
        }
        Some(_) => false,
        None => false,
    }
}
