use crate::ollama::types::{MODELE_DEFAUT, OLLAMA_URL};
use common::TradingError;
use db::rockets::{RocketSignal, RocketsConfig};
use serde::{Deserialize, Serialize};
use std::time::Duration;

// ── Types publics ─────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone)]
pub struct Recommandation {
    pub r#type: String,
    pub description: String,
    pub impact_estime: String,
    pub priorite: String, // "haute" | "moyenne" | "faible"
}

#[derive(Serialize, Deserialize)]
pub struct AnalyseReponse {
    pub synthese: String,
    pub recommandations: Vec<Recommandation>,
    pub meilleur_setup: Option<String>,
    pub pire_setup: Option<String>,
}

// ── Prompt ────────────────────────────────────────────────────────────────────

const PROMPT_ANALYSE_ROCKETS: &str = r#"Tu es un expert en trading algorithmique quantitatif spécialisé en crypto.
Analyse les métriques de performance d'une stratégie de type "Rocket" (breakout/compression/prelancement).

Réponds UNIQUEMENT en JSON valide, sans texte avant ou après, avec cette structure exacte :
{
  "synthese": "résumé en 2-3 phrases de la performance globale et des points clés",
  "recommandations": [
    {
      "type": "seuil_score|filtre_phase|trailing_stop|filtre_rsi|filtre_volume|mode_entree|autre",
      "description": "description concrète et actionnable de la recommandation",
      "impact_estime": "estimation chiffrée si possible ex: +8% winrate",
      "priorite": "haute|moyenne|faible"
    }
  ],
  "meilleur_setup": "description du setup le plus profitable identifié",
  "pire_setup": "description du setup à éviter en priorité"
}

Produis entre 3 et 6 recommandations. Priorise les actions à fort impact sur le winrate et le R moyen.
Base-toi uniquement sur les données fournies, pas sur des hypothèses générales.
Si la config actuelle te semble déjà bien calibrée sur un point, dis-le explicitement plutôt que de répéter la même recommandation."#;

// ── Agrégation ────────────────────────────────────────────────────────────────

struct MetriquesPhase {
    phase: String,
    total: usize,
    gains: usize,
    sl: usize,
    r_moyen: f64,
    score_moyen: f64,
    score_gagnants: f64,
    rsi_moyen: f64,
    vol_moyen: f64,
}

fn r_realise(s: &RocketSignal) -> Option<f64> {
    let pv = s.prix_verdict?;
    let risk = s.prix_entree - s.stop_loss;
    if risk <= 0.0 {
        return None;
    }
    Some((pv - s.prix_entree) / risk)
}

fn est_gain(verdict: &str) -> bool {
    matches!(verdict, "TP1" | "TP2" | "TP3" | "confirme")
}

fn agreger_par_phase(signaux: &[RocketSignal]) -> Vec<MetriquesPhase> {
    let mut phases: Vec<String> = signaux.iter().map(|s| s.phase.clone()).collect();
    phases.sort();
    phases.dedup();

    phases
        .into_iter()
        .map(|phase| {
            let groupe: Vec<&RocketSignal> =
                signaux.iter().filter(|s| s.phase == phase).collect();
            let total = groupe.len();
            let gains = groupe
                .iter()
                .filter(|s| s.verdict.as_deref().map(est_gain).unwrap_or(false))
                .count();
            let sl = groupe
                .iter()
                .filter(|s| s.verdict.as_deref() == Some("invalide"))
                .count();
            let rs: Vec<f64> = groupe.iter().filter_map(|s| r_realise(s)).collect();
            let r_moyen = if rs.is_empty() {
                0.0
            } else {
                rs.iter().sum::<f64>() / rs.len() as f64
            };
            let score_moyen =
                groupe.iter().map(|s| s.score as f64).sum::<f64>() / total.max(1) as f64;
            let gagnants: Vec<&RocketSignal> = groupe
                .iter()
                .filter(|s| s.verdict.as_deref().map(est_gain).unwrap_or(false))
                .copied()
                .collect();
            let score_gagnants = if gagnants.is_empty() {
                0.0
            } else {
                gagnants.iter().map(|s| s.score as f64).sum::<f64>() / gagnants.len() as f64
            };
            let rsi_moyen =
                groupe.iter().map(|s| s.rsi).sum::<f64>() / total.max(1) as f64;
            let vol_moyen =
                groupe.iter().map(|s| s.ratio_volume).sum::<f64>() / total.max(1) as f64;

            MetriquesPhase {
                phase,
                total,
                gains,
                sl,
                r_moyen,
                score_moyen,
                score_gagnants,
                rsi_moyen,
                vol_moyen,
            }
        })
        .collect()
}

fn formater_contexte(signaux: &[RocketSignal], cfg: &RocketsConfig) -> String {
    let total = signaux.len();
    let gains = signaux
        .iter()
        .filter(|s| s.verdict.as_deref().map(est_gain).unwrap_or(false))
        .count();
    let sl_count = signaux
        .iter()
        .filter(|s| s.verdict.as_deref() == Some("invalide"))
        .count();
    let winrate = if total > 0 {
        gains * 100 / total
    } else {
        0
    };
    let rs: Vec<f64> = signaux.iter().filter_map(r_realise).collect();
    let r_global = if rs.is_empty() {
        0.0
    } else {
        rs.iter().sum::<f64>() / rs.len() as f64
    };

    let phases_str = cfg.phases_actives.join(", ");
    let mut ctx = format!(
        "=== CONFIG ACTIVE DU SCANNER ===\n\
        score_min={score_min} | phases_actives=[{phases}]\n\
        rsi_min={rsi_min} | rsi_max={rsi_max} | ratio_volume_min={vol_ratio:.1} | vol_marche_min={vol_min:.0}M$\n\n\
        === MÉTRIQUES GLOBALES ({total} trades clôturés) ===\n\
        Winrate global : {winrate}% | R moyen : {r_global:.2}R\n\
        Gagnants : {gains} | SL : {sl_count} | Autres : {autres}\n\
        Coefficient ATR actuel : SL=1×ATR, TP1=1×ATR, TP2=2×ATR, TP3=20×ATR\n\n",
        score_min = cfg.score_min,
        phases = phases_str,
        rsi_min = cfg.rsi_min,
        rsi_max = cfg.rsi_max,
        vol_ratio = cfg.ratio_volume_min,
        vol_min = cfg.vol_marche_min / 1_000_000.0,
        autres = total - gains - sl_count,
    );

    ctx.push_str("=== PAR PHASE ===\n");
    for m in agreger_par_phase(signaux) {
        let wr = if m.total > 0 {
            m.gains * 100 / m.total
        } else {
            0
        };
        ctx.push_str(&format!(
            "Phase '{phase}' : {total} trades, winrate={wr}%, R moyen={r:.2}, \
            score moyen={sm:.0} (gagnants={sg:.0}), RSI moyen={rsi:.0}, vol_ratio moyen={vol:.1}\n",
            phase = m.phase,
            total = m.total,
            r = m.r_moyen,
            sm = m.score_moyen,
            sg = m.score_gagnants,
            rsi = m.rsi_moyen,
            vol = m.vol_moyen,
        ));
        ctx.push_str(&format!("  SL touchés : {}\n", m.sl));
    }

    // Corrélations score/winrate par tranche
    ctx.push_str("\n=== PAR TRANCHE DE SCORE ===\n");
    for (min, max) in [(15, 39), (40, 59), (60, 79), (80, 100)] {
        let t: Vec<&RocketSignal> = signaux
            .iter()
            .filter(|s| s.score >= min && s.score <= max)
            .collect();
        if t.is_empty() {
            continue;
        }
        let g = t
            .iter()
            .filter(|s| s.verdict.as_deref().map(est_gain).unwrap_or(false))
            .count();
        let wr = g * 100 / t.len();
        ctx.push_str(&format!(
            "Score {min}-{max} : {} trades, winrate={wr}%\n",
            t.len()
        ));
    }

    ctx
}

// ── Appel LLM ────────────────────────────────────────────────────────────────

pub async fn analyser_strategie(signaux: &[RocketSignal], cfg: &RocketsConfig) -> Result<AnalyseReponse, TradingError> {
    let contexte = formater_contexte(signaux, cfg);
    let prompt = format!("{PROMPT_ANALYSE_ROCKETS}\n\n{contexte}");

    let modele = std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| MODELE_DEFAUT.to_string());
    let url = std::env::var("OLLAMA_URL").unwrap_or_else(|_| OLLAMA_URL.to_string());

    let corps = serde_json::json!({
        "model": modele,
        "messages": [{"role": "user", "content": prompt}],
        "stream": false,
        "options": { "temperature": 0.3, "num_predict": 1024 }
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
        .map_err(|e| TradingError::Api(format!("Ollama injoignable: {}", e)))?;

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
        .map_err(|e| TradingError::Api(format!("Réponse Ollama invalide: {}", e)))?;

    let texte = data.message.content;
    let debut = texte.find('{').unwrap_or(0);
    let fin = texte.rfind('}').map(|i| i + 1).unwrap_or(texte.len());
    serde_json::from_str::<AnalyseReponse>(&texte[debut..fin])
        .map_err(|e| TradingError::Api(format!("JSON LLM non parsable: {} — texte: {}", e, &texte[..texte.len().min(300)])))
}
