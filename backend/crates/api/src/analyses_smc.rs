//! Enrichissements SMC du centre d'analyse — tranches de score + bloc IA
//! (rail conviction LLM, directions), extraits d'analyses.rs (limite 600
//! lignes). Même base que le reste de l'analyse : les clôtures vécues.

use crate::analyses::ClotureAnalyse;
use serde::Serialize;
use std::sync::Arc;

/// Une tranche de score SMC (bandes métier du moteur v12) — même base que
/// nb_trades : toutes les clôtures vécues, expirés compris.
#[derive(Serialize)]
pub struct TrancheScore {
    pub label: String,
    pub n: usize,
    pub tp1: usize,
    pub tp2: usize,
    pub tp3: usize,
    pub sl: usize,
    pub expire: usize,
    /// Part des clôtures gagnantes ($ > 0) — 0-1.
    pub wr: f64,
    /// R-distance moyen de la tranche.
    pub r_moyen: f64,
}

/// Un avis LLM (rail conviction) — les plus récents de la stratégie.
#[derive(Serialize)]
pub struct AvisLlm {
    pub id: String,
    pub asset: String,
    pub tf: String,
    pub conviction: i64,
    pub raison: Option<String>,
}

/// Bloc IA des pages d'analyse : conviction moyenne du rail LLM, part des
/// signaux notés, directions émises — sur l'intégralité des signaux de la
/// stratégie (pas une fenêtre front).
#[derive(Serialize)]
pub struct BlocIa {
    pub conviction_moyenne: f64,
    /// Part des signaux notés par le LLM — 0-1.
    pub taux_notes: f64,
    pub longs: usize,
    pub shorts: usize,
    pub derniers_avis: Vec<AvisLlm>,
}

impl BlocIa {
    pub(crate) fn vide() -> Self {
        Self {
            conviction_moyenne: 0.0,
            taux_notes: 0.0,
            longs: 0,
            shorts: 0,
            derniers_avis: Vec::new(),
        }
    }
}

/// Bandes de score du moteur v12 (seuil trade 7 — « seuil / confirmé / fort »).
const TRANCHES_SCORE: [(&str, f64, f64); 3] = [("6–8", 6.0, 8.0), ("9–11", 9.0, 11.0), ("12–19", 12.0, 19.0)];

/// Tranches de score + bloc IA de la page d'analyse SMC — calculés sur les
/// signaux réels (table signaux complète, pas un extrait front). Les scores
/// et avis LLM sont rapprochés des clôtures PAR ID : même base que nb_trades.
pub(crate) async fn enrichissements_smc(
    db: &Arc<db::Database>,
    clotures: &[ClotureAnalyse],
) -> (Vec<TrancheScore>, BlocIa) {
    let signaux = db.obtenir_signaux(5000).await.unwrap_or_default();
    let smc: Vec<&serde_json::Value> = signaux
        .iter()
        .filter(|s| {
            s.get("strategie")
                .and_then(|v| v.as_str())
                .map(|v| v.to_lowercase().starts_with("smc"))
                .unwrap_or(false)
        })
        .collect();

    // id → (score, conviction) pour rattacher chaque clôture à son signal.
    use std::collections::HashMap;
    let mut score_par_id: HashMap<String, f64> = HashMap::new();
    for s in &smc {
        if let (Some(id), Some(score)) = (
            s.get("id").and_then(|v| v.as_str()),
            s.get("score").and_then(|v| v.as_f64()),
        ) {
            score_par_id.insert(id.to_string(), score);
        }
    }

    let mut tranches = Vec::with_capacity(TRANCHES_SCORE.len());
    for (label, min, max) in TRANCHES_SCORE {
        let membres: Vec<&ClotureAnalyse> = clotures
            .iter()
            .filter(|c| score_par_id.get(&c.id).map(|s| *s >= min && *s <= max).unwrap_or(false))
            .collect();
        let n = membres.len();
        let somme_r: f64 = membres.iter().map(|c| c.r).sum();
        tranches.push(TrancheScore {
            label: label.to_string(),
            n,
            tp1: membres.iter().filter(|c| c.verdict.starts_with("TP1")).count(),
            tp2: membres.iter().filter(|c| c.verdict.starts_with("TP2")).count(),
            tp3: membres.iter().filter(|c| c.verdict == "TP3").count(),
            sl: membres.iter().filter(|c| c.verdict == "SL").count(),
            expire: membres.iter().filter(|c| c.verdict == "Expire").count(),
            wr: if n > 0 {
                membres.iter().filter(|c| c.dollars > 0.0).count() as f64 / n as f64
            } else {
                0.0
            },
            r_moyen: if n > 0 { somme_r / n as f64 } else { 0.0 },
        });
    }

    // Bloc IA : sur TOUS les signaux SMC émis (activité du rail conviction).
    let notes: Vec<i64> = smc
        .iter()
        .filter_map(|s| s.get("llm_conviction").and_then(|v| v.as_i64()))
        .collect();
    let longs = smc
        .iter()
        .filter(|s| s.get("direction").and_then(|v| v.as_str()).map(|d| d.eq_ignore_ascii_case("long")).unwrap_or(false))
        .count();
    let shorts = smc
        .iter()
        .filter(|s| s.get("direction").and_then(|v| v.as_str()).map(|d| d.eq_ignore_ascii_case("short")).unwrap_or(false))
        .count();
    let derniers_avis: Vec<AvisLlm> = smc
        .iter()
        .filter(|s| s.get("llm_conviction").and_then(|v| v.as_i64()).is_some())
        .take(5)
        .filter_map(|s| {
            Some(AvisLlm {
                id: s.get("id")?.as_str()?.to_string(),
                asset: s.get("asset")?.as_str()?.to_string(),
                tf: s.get("timeframe")?.as_str()?.to_string(),
                conviction: s.get("llm_conviction")?.as_i64()?,
                raison: s.get("llm_raison").and_then(|v| v.as_str()).map(|r| r.to_string()),
            })
        })
        .collect();
    let ia = BlocIa {
        conviction_moyenne: if !notes.is_empty() {
            notes.iter().sum::<i64>() as f64 / notes.len() as f64
        } else {
            0.0
        },
        taux_notes: if !smc.is_empty() {
            notes.len() as f64 / smc.len() as f64
        } else {
            0.0
        },
        longs,
        shorts,
        derniers_avis,
    };
    (tranches, ia)
}

