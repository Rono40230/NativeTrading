//! Analyse stratégique Rockets — V2, recâblée le 05/09 sur les vraies
//! données de la verticale (§10) : signaux officiels clôturés (table
//! `signaux`, verdicts SL/TS avec R réalisé), candidats du scanner
//! (`rockets_candidats`, classement par palier et univers) et réglages
//! (`rockets_params`). L'ancienne version lisait la table v1
//! `rockets_signaux` (0 ligne — le bouton 📊 Analyse répondait toujours
//! « pas assez de trades »).

use crate::ollama::types::{MODELE_DEFAUT, OLLAMA_URL};
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
pub struct AnalyseReponse {
    pub synthese: String,
    pub recommandations: Vec<Recommandation>,
    pub meilleur_setup: Option<String>,
    pub pire_setup: Option<String>,
}

// ── Prompt ────────────────────────────────────────────────────────────────────

pub const PROMPT_ANALYSE_ROCKETS: &str = r#"Tu es un expert en trading algorithmique quantitatif, stratégie "Rockets" (VCP × Rocket Hunter, classement /10).

## CONTEXTE DE LA STRATÉGIE ROCKETS (moteur réel)
Rockets détecte les décollages de prix par un classement /10 à quatre piliers :
- Fondamental (3) : sentiment = force relative (battre la référence BTC/QQQ sur 4 semaines, sans veto macro — 05/09), contexte (pivot âgé ≥ 30 j, prix ≥ 90 % du pivot), news catalyseur (IA).
- Technique (3) : tendance (prix > MM50 > MM200, proche du haut 52 s.), volatilité (squeeze Bollinger puis expansion), intérêt (volumes asséchés puis explosion).
- Chartisme (2) : VCP (contractions décroissantes), pas de gros gaps.
- Pilotage (2) : cassure du pivot 60 j, liquidité (mèche haute ≤ 25 % de l'étendue).
Verdicts : Alpha ≥ 9/9, Rocket ≥ 7/9 ; candidats ≥ 5 suivis. Deux univers : crypto (top 300 Binance) et actions US (périmètre liquide 450, narratifs prioritaires — 05/09).
Gestion : invalidation −1R sous la dernière contraction ; R1 → vendre 50 % puis trailing % (défaut 5 %). Les réglages actuels (seuil de suivi, trailing, plafond de position) sont indiqués dans le contexte.
Tes recommandations doivent comparer les réglages actuels aux performances observées.

## STRUCTURE DE RÉPONSE (JSON uniquement, sans texte autour)
{
  "synthese": "résumé en 2-3 phrases de la performance globale",
  "recommandations": [
    {
      "type": "seuil_classement|filtre_univers|trailing|invalidation|conviction_min|mode_entree|autre",
      "description": "recommandation concrète et actionnable",
      "impact_estime": "estimation chiffrée si possible ex: +8% winrate",
      "priorite": "haute|moyenne|faible"
    }
  ],
  "meilleur_setup": "description précise du setup le plus profitable (univers, classement, critères)",
  "pire_setup": "description précise du setup à éviter (univers, classement, critères)"
}

## RÈGLES D'ANALYSE
- Produis entre 3 et 6 recommandations, classées par impact décroissant
- Compare chaque réglage actuel (config fournie) aux performances observées — dis explicitement si un réglage est déjà bien calibré
- Analyse la performance par univers (crypto vs actions), par palier de classement (5-6, 7-8, 9) et par verdict
- Pour le trailing et l'invalidation : les trades gagnants/perdants suggèrent-ils de les ajuster ?
- Base-toi uniquement sur les données fournies, pas sur des hypothèses générales"#;

// ── Appel LLM ────────────────────────────────────────────────────────────────

pub async fn analyser_strategie(contexte: &str) -> Result<AnalyseReponse, TradingError> {
    // La définition de la stratégie ancre l'analyste (constitution 26/08).
    let prompt = format!(
        "{}\n\n{}\n\n{contexte}",
        crate::prompt_effectif("rockets_definition"),
        crate::prompt_effectif("rockets_analyse")
    );

    let modele = std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| MODELE_DEFAUT.to_string());
    let url = std::env::var("OLLAMA_URL").unwrap_or_else(|_| OLLAMA_URL.to_string());

    let corps = serde_json::json!({
        "model": modele,
        "messages": [{"role": "user", "content": prompt}],
        "stream": false,
        "options": { "temperature": 0.3, "num_predict": 1024, "num_gpu": 99, "num_ctx": 8192 }
    });

    let _permit = super::OLLAMA_SEMAPHORE.acquire().await.ok();
    let client = &*super::OLLAMA_HTTP_CLIENT;

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

    let data: super::ReponseOllama = reponse
        .json()
        .await
        .map_err(|e| TradingError::Api(format!("Réponse Ollama invalide: {}", e)))?;

    let texte = data.message.content;
    let debut = texte.find('{').unwrap_or(0);
    let fin = texte.rfind('}').map(|i| i + 1).unwrap_or(texte.len());
    serde_json::from_str::<AnalyseReponse>(&texte[debut..fin]).map_err(|e| {
        TradingError::Api(format!(
            "JSON LLM non parsable: {} — texte: {}",
            e,
            &texte[..texte.len().min(300)]
        ))
    })
}
