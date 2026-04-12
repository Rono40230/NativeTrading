use crate::ollama::types::{MODELE_DEFAUT, OLLAMA_URL};
use common::TradingError;
use db::rockets::RocketSignal;
use db::rockets_feedback::RocketsFeedbackRow;
use serde::{Deserialize, Serialize};
use std::time::Duration;

// ── Types publics ─────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize)]
pub struct AjustementsSl {
    pub sl_suggere: Option<f64>,
    pub tp1_suggere: Option<f64>,
    /// Coefficient trailing dynamique proposé par le LLM (borné en [1.5, 5.0])
    pub trailing_coeff_suggere: Option<f64>,
    /// Type d'entrée recommandé par le LLM : "limite", "stop", ou null si pas d'avis
    pub entry_type_suggere: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct FiltreReponse {
    pub valide: bool,
    pub conviction: i64, // 0–100
    pub raison: String,
    pub ajustements: Option<AjustementsSl>,
}

// ── Signal courant (données du scan) ─────────────────────────────────────────

pub struct SignalCandidat {
    pub ticker: String,
    pub phase: String,
    pub score: i64,
    pub prix_entree: f64,
    pub stop_loss: f64,
    pub tp1: f64,
    pub atr14: f64,
    pub atr_ratio: f64,
    pub ratio_volume: f64,
    pub rsi: f64,
    pub change1h: f64,
    /// Ratio corps/amplitude de la bougie de signal (0.0–1.0)
    pub ratio_corps: f64,
    /// Tendance préalable confirmée (EMA20 > EMA50)
    pub tendance_haussiere: bool,
    /// Bougies consécutives en compression avant la bougie de signal
    pub nb_bougies_compression: usize,
    /// Range de la zone de consolidation (measured move pour TP1)
    pub hauteur_base: f64,
    /// Entrée limite calculée algorithmiquement (pullback vers zone de consolidation)
    pub entree_limite: f64,
    /// Entrée stop calculée algorithmiquement (confirmation de cassure)
    pub entree_stop: f64,
    /// Niveau d'invalidation structurelle (setup annulé si atteint avant l'entrée)
    pub niveau_invalidation: f64,
    /// Type d'entrée recommandé par l'algo : "limite" ou "stop"
    pub type_entree_rec_algo: String,
}

// ── Formatage du contexte ─────────────────────────────────────────────────────

fn formater_contexte(candidat: &SignalCandidat, historique: &[RocketSignal]) -> String {
    let mut ctx = format!(
        "=== SIGNAL CANDIDAT : {} ===\n\
        Phase: {} | Score: {}/100\n\
        Prix entrée: {:.6} | SL: {:.6} | TP1: {:.6}\n\
        Entrée limite: {:.6} | Entrée stop: {:.6} | Invalidation: {:.6}\n\
        Type entrée algo: {} \n\
        ATR14: {:.6} | ATR ratio (accélération): {:.2}\n\
        Volume ratio: {:.2}× | RSI: {:.1} | Change 1h: {:.2}%\n\
        Ratio corps/mèche bougie: {:.2} (1.0=pleine, <0.3=rejet probable)\n\
        Tendance préalable (EMA20>EMA50): {} | Compression: {} bougies | Hauteur base (measured move): {:.6}\n\n",
        candidat.ticker,
        candidat.phase,
        candidat.score,
        candidat.prix_entree,
        candidat.stop_loss,
        candidat.tp1,
        candidat.entree_limite,
        candidat.entree_stop,
        candidat.niveau_invalidation,
        candidat.type_entree_rec_algo,
        candidat.atr14,
        candidat.atr_ratio,
        candidat.ratio_volume,
        candidat.rsi,
        candidat.change1h,
        candidat.ratio_corps,
        if candidat.tendance_haussiere { "✅ oui" } else { "❌ non" },
        candidat.nb_bougies_compression,
        candidat.hauteur_base,
    );

    if historique.is_empty() {
        ctx.push_str("=== HISTORIQUE : Aucun trade clôturé sur ce ticker ===\n");
        return ctx;
    }

    ctx.push_str(&format!(
        "=== HISTORIQUE {} ({} trades clôturés) ===\n",
        candidat.ticker,
        historique.len()
    ));

    let gains = historique
        .iter()
        .filter(|s| {
            s.verdict
                .as_deref()
                .map(|v| matches!(v, "TP1" | "TP2" | "TP3" | "confirme"))
                .unwrap_or(false)
        })
        .count();
    let winrate = gains * 100 / historique.len();

    let rs: Vec<f64> = historique
        .iter()
        .filter_map(|s| {
            let pv = s.prix_verdict?;
            let risk = s.prix_entree - s.stop_loss;
            if risk <= 0.0 {
                return None;
            }
            Some((pv - s.prix_entree) / risk)
        })
        .collect();
    let r_moyen = if rs.is_empty() {
        0.0
    } else {
        rs.iter().sum::<f64>() / rs.len() as f64
    };

    ctx.push_str(&format!(
        "Winrate: {}% | R moyen: {:.2}R\n",
        winrate, r_moyen
    ));

    ctx.push_str("Derniers résultats :\n");
    for s in historique.iter().take(5) {
        let verdict = s.verdict.as_deref().unwrap_or("?");
        let r_str = s
            .prix_verdict
            .and_then(|pv| {
                let risk = s.prix_entree - s.stop_loss;
                if risk > 0.0 {
                    Some(format!("{:.2}R", (pv - s.prix_entree) / risk))
                } else {
                    None
                }
            })
            .unwrap_or_else(|| "?R".to_string());
        ctx.push_str(&format!(
            "  • {} | phase={} score={} RSI={:.0} vol={:.1}× → {} ({})\n",
            s.ticker, s.phase, s.score, s.rsi, s.ratio_volume, verdict, r_str
        ));
    }

    ctx
}

// ── Appel LLM avec timeout 90s ───────────────────────────────────────────────

/// Construit le bloc "leçons passées" injecté dans le prompt few-shot.
fn construire_few_shot(feedbacks: &[RocketsFeedbackRow]) -> String {
    if feedbacks.is_empty() {
        return String::new();
    }
    let mut bloc = String::from("=== LEÇONS PASSÉES (signaux similaires clôturés) ===\n");
    for fb in feedbacks {
        let resultat = if fb.gagnant == Some(1) {
            "✅ GAGNANT"
        } else {
            "❌ PERDANT"
        };
        let pnl = fb.pnl_r.map(|r| format!("{:.2}R", r)).unwrap_or_default();
        bloc.push_str(&format!(
            "  • {} | score={} conviction={} RSI={:.0} vol={:.1}× → {} {}\n",
            fb.verdict.as_deref().unwrap_or("?"),
            fb.score_scan,
            fb.conviction_llm,
            fb.rsi,
            fb.ratio_volume,
            resultat,
            pnl,
        ));
    }
    bloc
}

pub async fn filtrer_signal(
    candidat: &SignalCandidat,
    historique: &[RocketSignal],
    feedbacks: &[RocketsFeedbackRow],
) -> Result<FiltreReponse, TradingError> {
    let mut contexte = formater_contexte(candidat, historique);
    let few_shot = construire_few_shot(feedbacks);
    if !few_shot.is_empty() {
        contexte.push_str(&few_shot);
    }
    let prompt = format!(
        "{}\n\n{contexte}",
        crate::prompts_handler::prompt_effectif("rockets_filtre")
    );

    let modele = std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| MODELE_DEFAUT.to_string());
    let url = std::env::var("OLLAMA_URL").unwrap_or_else(|_| OLLAMA_URL.to_string());

    let corps = serde_json::json!({
        "model": modele,
        "messages": [{"role": "user", "content": prompt}],
        "stream": false,
        "options": { "temperature": 0.1, "num_predict": 256 }
    });

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(90))
        .build()
        .map_err(|e| TradingError::Api(e.to_string()))?;

    let reponse = client
        .post(&url)
        .json(&corps)
        .send()
        .await
        .map_err(|e| TradingError::Api(format!("Ollama timeout: {}", e)))?;

    if !reponse.status().is_success() {
        return Err(TradingError::Api(format!(
            "Ollama HTTP {}",
            reponse.status()
        )));
    }

    #[derive(Deserialize)]
    struct OllamaResp {
        message: OllamaMsg,
    }
    #[derive(Deserialize)]
    struct OllamaMsg {
        content: String,
    }

    let data: OllamaResp = reponse
        .json()
        .await
        .map_err(|e| TradingError::Api(format!("JSON Ollama: {}", e)))?;

    let texte = data.message.content;
    let debut = texte.find('{').unwrap_or(0);
    let fin = texte.rfind('}').map(|i| i + 1).unwrap_or(texte.len());

    serde_json::from_str::<FiltreReponse>(&texte[debut..fin])
        .map_err(|e| TradingError::Api(format!("JSON filtre non parsable: {}", e)))
}
