use crate::ollama::types::{MODELE_DEFAUT, OLLAMA_URL};
use common::TradingError;
use db::rockets::RocketSignal;
use serde::{Deserialize, Serialize};
use std::time::Duration;

// ── Types publics ─────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize)]
pub struct AjustementsSl {
    pub sl_suggere: Option<f64>,
    pub tp1_suggere: Option<f64>,
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
}

// ── Prompt ────────────────────────────────────────────────────────────────────

const PROMPT_FILTRE_ROCKET: &str = r#"Tu es un trader quantitatif expert en crypto. Tu dois évaluer si un signal de trading
de type "Rocket" mérite d'être suivi, en croisant les données actuelles avec l'historique du ticker.

Réponds UNIQUEMENT en JSON valide, sans texte avant ou après :
{
  "valide": true | false,
  "conviction": 0-100,
  "raison": "explication courte (max 120 caractères)",
  "ajustements": {
    "sl_suggere": <float ou null>,
    "tp1_suggere": <float ou null>
  }
}

Règles de validation :
- valide=true si le setup est cohérent avec les patterns gagnants historiques du ticker
- valide=false si : RSI>85 sur breakout; série de SL récents; phase historiquement perdante; score<40
- conviction reflète la qualité du setup (80+ = excellent, 60-79 = bon, 40-59 = acceptable, <40 = rejeter)
- Suggère un sl_suggere ou tp1_suggere uniquement si l'ajustement est clairement justifié par l'historique
- Si pas d'historique sur ce ticker, évalue uniquement sur les critères techniques actuels"#;

// ── Formatage du contexte ─────────────────────────────────────────────────────

fn formater_contexte(candidat: &SignalCandidat, historique: &[RocketSignal]) -> String {
    let mut ctx = format!(
        "=== SIGNAL CANDIDAT : {} ===\n\
        Phase: {} | Score: {}/100\n\
        Prix entrée: {:.6} | SL: {:.6} | TP1: {:.6}\n\
        ATR14: {:.6} | ATR ratio (accélération): {:.2}\n\
        Volume ratio: {:.2}× | RSI: {:.1} | Change 1h: {:.2}%\n\n",
        candidat.ticker,
        candidat.phase,
        candidat.score,
        candidat.prix_entree,
        candidat.stop_loss,
        candidat.tp1,
        candidat.atr14,
        candidat.atr_ratio,
        candidat.ratio_volume,
        candidat.rsi,
        candidat.change1h,
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

// ── Appel LLM avec timeout 5s ─────────────────────────────────────────────────

pub async fn filtrer_signal(
    candidat: &SignalCandidat,
    historique: &[RocketSignal],
) -> Result<FiltreReponse, TradingError> {
    let contexte = formater_contexte(candidat, historique);
    let prompt = format!("{PROMPT_FILTRE_ROCKET}\n\n{contexte}");

    let modele = std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| MODELE_DEFAUT.to_string());
    let url = std::env::var("OLLAMA_URL").unwrap_or_else(|_| OLLAMA_URL.to_string());

    let corps = serde_json::json!({
        "model": modele,
        "messages": [{"role": "user", "content": prompt}],
        "stream": false,
        "options": { "temperature": 0.1, "num_predict": 256 }
    });

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
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
