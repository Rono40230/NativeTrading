//! Analyse LLM périodique des performances SMC Directionnel.
//!
//! Calqué sur `rockets_analyse.rs` — déclenché sur demande via `/api/smc/analyse-llm`.
//! Analyse les signaux SMC clôturés et produit des recommandations d'optimisation.
use crate::ollama::types::{MODELE_SMC, OLLAMA_URL};
use common::TradingError;
use serde::{Deserialize, Serialize};

// ── Types publics ─────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone)]
pub struct Recommandation {
    pub r#type: String,
    pub description: String,
    pub impact_estime: String,
    pub priorite: String, // "haute" | "moyenne" | "faible"
}

#[derive(Serialize, Deserialize)]
pub struct AnalyseSMCReponse {
    pub synthese: String,
    pub recommandations: Vec<Recommandation>,
    pub meilleur_setup: Option<String>,
    pub pire_setup: Option<String>,
}

// ── Signal SMC simplifié pour analyse ────────────────────────────────────────

pub struct SignalSMCClotl {
    pub asset: String,
    pub timeframe: String,
    pub direction: String,
    pub _score: f64,
    pub statut: String,
    pub verdict: Option<String>,
    pub llm_conviction: Option<i64>,
    pub _cree_le: i64,
}

// ── Prompt ────────────────────────────────────────────────────────────────────

pub const PROMPT_ANALYSE_SMC: &str = r#"Tu es un expert en trading algorithmique SMC/ICT quantitatif.

## CONTEXTE DE LA STRATÉGIE SMC DIRECTIONNEL
La stratégie SMC Directionnel génère des signaux directionnels basés sur la confluence de :
- Structure de marché ICT (HH/HL, LH/LL)
- Order Blocks non mitigés
- IFVG (Imbalance / Fair Value Gap)
- Niveaux Fibonacci (38.2–61.8%)
- Kill Zone ICT active (London 07h-10h UTC, New York 13h30-16h30 UTC) — prérequis BLOQUANT
- Liquidity Sweep confirmé — prérequis BLOQUANT
- Score SMC ≥ 60 + ML ≥ 0.60 — prérequis BLOQUANT
- Filtre LLM : conviction ≥ 65 avant sauvegarde

## STRUCTURE DE RÉPONSE (JSON uniquement, sans texte autour)
{
  "synthese": "résumé en 2-3 phrases de la performance globale",
  "recommandations": [
    {
      "type": "seuil_score|filtre_kill_zone|filtre_sweep|filtre_ml|assets_privilégier|timeframes|filtre_llm|autre",
      "description": "recommandation concrète et actionnable",
      "impact_estime": "estimation chiffrée si possible ex: +12% winrate",
      "priorite": "haute|moyenne|faible"
    }
  ],
  "meilleur_setup": "description du setup le plus profitable (asset, TF, direction, conditions)",
  "pire_setup": "description du setup à éviter absolument"
}

## RÈGLES D'ANALYSE
- Produis entre 3 et 6 recommandations classées par priorité
- Analyse la performance par asset, timeframe, direction et niveau de conviction LLM
- Identifie les patterns de SL récurrents (sur-optimisation des seuils ?)
- Pour le filtre LLM : les trades avec conviction ≥ 80 ont-ils un meilleur winrate que ceux à 65-79 ?
- Base-toi uniquement sur les données fournies, pas sur des hypothèses générales

## PHILOSOPHIE
Un signal SMC évité = 1 SL évité. Mieux vaut 10 signaux précis que 30 signaux médiocres."#;

// ── Agrégation ────────────────────────────────────────────────────────────────

fn est_gain(verdict: &str) -> bool {
    matches!(verdict, "TP1" | "TP2" | "TP3")
}

fn formater_contexte(signaux: &[SignalSMCClotl]) -> String {
    let total = signaux.len();
    if total == 0 {
        return "Aucun signal SMC clôturé disponible pour analyse.".to_string();
    }

    let fermes: Vec<&SignalSMCClotl> = signaux.iter().filter(|s| s.statut == "Fermé").collect();

    let gains = fermes
        .iter()
        .filter(|s| s.verdict.as_deref().map(est_gain).unwrap_or(false))
        .count();
    let sl_count = fermes
        .iter()
        .filter(|s| s.verdict.as_deref() == Some("SL"))
        .count();
    let winrate = if !fermes.is_empty() {
        gains * 100 / fermes.len()
    } else {
        0
    };

    let mut ctx = format!(
        "=== MÉTRIQUES GLOBALES SMC ({total} signaux, {} clôturés) ===\n\
        Winrate : {winrate}% | Gains : {gains} | SL : {sl_count} | Expirés : {expires}\n\n",
        fermes.len(),
        expires = fermes.len() - gains - sl_count,
    );

    // Par asset
    let mut assets: Vec<String> = signaux.iter().map(|s| s.asset.clone()).collect();
    assets.sort();
    assets.dedup();
    ctx.push_str("=== PAR ASSET ===\n");
    for asset in &assets {
        let groupe: Vec<&SignalSMCClotl> = signaux.iter().filter(|s| &s.asset == asset).collect();
        let ferms: Vec<&&SignalSMCClotl> = groupe.iter().filter(|s| s.statut == "Fermé").collect();
        let g = ferms
            .iter()
            .filter(|s| s.verdict.as_deref().map(est_gain).unwrap_or(false))
            .count();
        let wr = if !ferms.is_empty() {
            g * 100 / ferms.len()
        } else {
            0
        };
        ctx.push_str(&format!(
            "{asset}: {} signaux, winrate={wr}%\n",
            groupe.len()
        ));
    }

    // Par timeframe
    let mut tfs: Vec<String> = signaux.iter().map(|s| s.timeframe.clone()).collect();
    tfs.sort();
    tfs.dedup();
    ctx.push_str("\n=== PAR TIMEFRAME ===\n");
    for tf in &tfs {
        let groupe: Vec<&SignalSMCClotl> = signaux.iter().filter(|s| &s.timeframe == tf).collect();
        let ferms: Vec<&&SignalSMCClotl> = groupe.iter().filter(|s| s.statut == "Fermé").collect();
        let g = ferms
            .iter()
            .filter(|s| s.verdict.as_deref().map(est_gain).unwrap_or(false))
            .count();
        let wr = if !ferms.is_empty() {
            g * 100 / ferms.len()
        } else {
            0
        };
        ctx.push_str(&format!("{tf}: {} signaux, winrate={wr}%\n", groupe.len()));
    }

    // Par conviction LLM
    ctx.push_str("\n=== PAR CONVICTION LLM ===\n");
    for (min, max) in [(65i64, 74i64), (75, 84), (85, 100)] {
        let groupe: Vec<&SignalSMCClotl> = signaux
            .iter()
            .filter(|s| {
                s.llm_conviction
                    .map(|c| c >= min && c <= max)
                    .unwrap_or(false)
            })
            .collect();
        if groupe.is_empty() {
            continue;
        }
        let ferms: Vec<&&SignalSMCClotl> = groupe.iter().filter(|s| s.statut == "Fermé").collect();
        let g = ferms
            .iter()
            .filter(|s| s.verdict.as_deref().map(est_gain).unwrap_or(false))
            .count();
        let wr = if !ferms.is_empty() {
            g * 100 / ferms.len()
        } else {
            0
        };
        ctx.push_str(&format!(
            "Conviction {min}-{max}: {} signaux, winrate={wr}%\n",
            groupe.len()
        ));
    }
    let sans_llm = signaux
        .iter()
        .filter(|s| s.llm_conviction.is_none())
        .count();
    if sans_llm > 0 {
        ctx.push_str(&format!("Sans LLM (avant filtre) : {sans_llm} signaux\n"));
    }

    // Par direction
    ctx.push_str("\n=== PAR DIRECTION ===\n");
    for dir in &["Long", "Short"] {
        let groupe: Vec<&SignalSMCClotl> = signaux.iter().filter(|s| s.direction == *dir).collect();
        let ferms: Vec<&&SignalSMCClotl> = groupe.iter().filter(|s| s.statut == "Fermé").collect();
        let g = ferms
            .iter()
            .filter(|s| s.verdict.as_deref().map(est_gain).unwrap_or(false))
            .count();
        let wr = if !ferms.is_empty() {
            g * 100 / ferms.len()
        } else {
            0
        };
        ctx.push_str(&format!("{dir}: {} signaux, winrate={wr}%\n", groupe.len()));
    }

    ctx
}

// ── Appel LLM ────────────────────────────────────────────────────────────────

pub async fn analyser_strategie(
    signaux: &[SignalSMCClotl],
    contexte_backtest: Option<&str>,
) -> Result<AnalyseSMCReponse, TradingError> {
    let mut contexte = formater_contexte(signaux);
    if let Some(ctx) = contexte_backtest {
        contexte.push_str(ctx);
    }
    // /no_think : l'analyse de performances n'a pas besoin du reasoning chain-of-thought,
    // on gagne en vitesse avec le mode non-thinking de Qwen3.
    let prompt = format!(
        "{}\n\n{contexte}\n/no_think",
        crate::prompts_handler::prompt_effectif("smc_analyse")
    );

    let modele = std::env::var("OLLAMA_MODEL_SMC")
        .unwrap_or_else(|_| MODELE_SMC.to_string());
    let url = std::env::var("OLLAMA_URL").unwrap_or_else(|_| OLLAMA_URL.to_string());

    let corps = serde_json::json!({
        "model": modele,
        "messages": [{"role": "user", "content": prompt}],
        "stream": false,
        "options": { "temperature": 0.7, "num_predict": 1024, "num_gpu": 99, "num_ctx": 8192 }
    });

    let _permit = super::OLLAMA_SEMAPHORE.acquire().await.ok();
    let client = &*super::OLLAMA_HTTP_CLIENT;

    let reponse = client
        .post(&url)
        .json(&corps)
        .send()
        .await
        .map_err(|e| TradingError::Api(format!("Ollama injoignable (smc_analyse): {}", e)))?;

    if !reponse.status().is_success() {
        return Err(TradingError::Api(format!(
            "Ollama HTTP {} (smc_analyse)",
            reponse.status()
        )));
    }

    let data: super::ReponseOllama = reponse
        .json()
        .await
        .map_err(|e| TradingError::Api(format!("Réponse Ollama invalide: {}", e)))?;

    // Filtrer les balises <think>...</think> (Qwen3 peut les produire même avec /no_think)
    let texte = super::filtrer_think(data.message.content);
    let debut = texte.find('{').unwrap_or(0);
    let fin = texte.rfind('}').map(|i| i + 1).unwrap_or(texte.len());
    serde_json::from_str::<AnalyseSMCReponse>(&texte[debut..fin]).map_err(|e| {
        TradingError::Api(format!(
            "JSON LLM SMC analyse non parsable: {} — début: {}",
            e,
            &texte[debut..texte.len().min(debut + 200)]
        ))
    })
}
