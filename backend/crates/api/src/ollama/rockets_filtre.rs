use crate::ollama::types::{MODELE_DEFAUT, OLLAMA_URL};
use common::TradingError;
use db::rockets::RocketSignal;
use db::rockets_feedback::RocketsFeedbackRow;
use serde::{Deserialize, Serialize};

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
    /// Ratio volume compression vs baseline (VCP) : <0.75 = assèchement valide
    pub volume_seche: f64,
    /// Score 0.0–1.0 progressivité des contractions (VCP Minervini) : >0.7 = valide
    pub contraction_qualite: f64,
    /// ATR 50 périodes (référence long terme)
    pub atr50: f64,
    /// Série ordonnée des amplitudes (high−low) des bougies de compression (la plus ancienne en premier).
    /// Décroissante = VCP authentique. Vide si < 2 bougies en compression.
    pub swing_amplitudes: Vec<f64>,
    /// Session de marché active : "london" | "ny" | "asia" | "off"
    pub session: String,
    /// Contexte marché global 48h : (nb_trades, win_rate 0.0–1.0, pnl_moyen_r)
    pub tendance_marche_48h: (i64, f64, f64),
}

// ── Formatage du contexte (délégué à rockets_contexte.rs) ────────────────────

use crate::ollama::rockets_contexte::{construire_few_shot, formater_contexte};

// ── Appel LLM avec timeout 90s ───────────────────────────────────────────────

pub async fn filtrer_signal(
    candidat: &SignalCandidat,
    historique: &[RocketSignal],
    feedbacks: &[RocketsFeedbackRow],
    lecons_systemiques: &str,
) -> Result<FiltreReponse, TradingError> {
    let mut contexte = formater_contexte(candidat, historique);
    let few_shot = construire_few_shot(feedbacks);
    if !few_shot.is_empty() {
        contexte.push_str(&few_shot);
    }
    if !lecons_systemiques.is_empty() {
        contexte.push('\n');
        contexte.push_str(lecons_systemiques);
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
        "options": { "temperature": 0.1, "num_predict": 400, "num_gpu": 99, "num_ctx": 4096 }
    });

    let _permit = super::OLLAMA_SEMAPHORE.acquire().await.ok();
    let client = &*super::OLLAMA_HTTP_CLIENT;

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

    if let Ok(reponse) = serde_json::from_str::<FiltreReponse>(&texte[debut..fin]) {
        return Ok(reponse);
    }
    // JSON complet non parsable — tenter extraction partielle (tronqué à num_predict)
    if let Some(partiel) = extraire_reponse_partielle(&texte[debut..fin]) {
        tracing::warn!(ticker = %candidat.ticker, "JSON Ollama tronqué — conviction extraite: {}", partiel.conviction);
        return Ok(partiel);
    }

    // Retry avec prompt minimaliste si le JSON est malformé
    tracing::warn!(
        ticker = %candidat.ticker,
        "JSON Ollama malformé — retry prompt simplifié"
    );
    let prompt_retry = format!(
        "Réponds UNIQUEMENT avec ce JSON exact, sans aucun autre texte :\n\
        {{\"valide\": true, \"conviction\": 60, \"raison\": \"...\"}}\n\n\
        Signal : {} phase={} score={} RSI={:.0} atr_ratio={:.2}\n\
        Question : ce signal Rockets vaut-il la peine d'être tradé ?",
        candidat.ticker, candidat.phase, candidat.score, candidat.rsi, candidat.atr_ratio
    );
    let corps_retry = serde_json::json!({
        "model": modele,
        "messages": [{"role": "user", "content": prompt_retry}],
        "stream": false,
        "options": { "temperature": 0.0, "num_predict": 64, "num_gpu": 99, "num_ctx": 1024 }
    });
    let reponse_retry = client
        .post(&url)
        .json(&corps_retry)
        .send()
        .await
        .map_err(|e| TradingError::Api(format!("Ollama retry timeout: {}", e)))?;

    let data_retry: OllamaResp = reponse_retry
        .json()
        .await
        .map_err(|e| TradingError::Api(format!("JSON retry Ollama: {}", e)))?;

    let texte_retry = data_retry.message.content;
    let debut_r = texte_retry.find('{').unwrap_or(0);
    let fin_r = texte_retry.rfind('}').map(|i| i + 1).unwrap_or(texte_retry.len());

    if let Ok(r) = serde_json::from_str::<FiltreReponse>(&texte_retry[debut_r..fin_r]) {
        return Ok(r);
    }
    // Dernier recours : extraction partielle sur le retry
    extraire_reponse_partielle(&texte_retry[debut_r..fin_r])
        .ok_or_else(|| TradingError::Api("JSON filtre non parsable après retry".into()))
}

/// Extrait valide + conviction d'un JSON partiellement tronqué.
/// Utilisé quand num_predict coupe le JSON en plein milieu de la raison.
fn extraire_reponse_partielle(texte: &str) -> Option<FiltreReponse> {
    let valide = texte.contains("\"valide\": true") || texte.contains("\"valide\":true");
    let conviction_pos = texte.find("\"conviction\":")
        .or_else(|| texte.find("\"conviction\": "))?;
    let apres = texte[conviction_pos + 13..].trim_start();
    let conviction: i64 = apres
        .split(|c: char| !c.is_ascii_digit())
        .next()?
        .parse()
        .ok()?;
    if conviction == 0 {
        return None; // pas d'info utile
    }
    Some(FiltreReponse {
        valide,
        conviction,
        raison: "[Réponse LLM tronquée — conviction extraite]".into(),
        ajustements: None,
    })
}
