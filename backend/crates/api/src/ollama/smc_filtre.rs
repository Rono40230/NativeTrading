//! Filtre LLM pré-sauvegarde pour les signaux SMC Directionnel.
//!
//! Utilise Qwen3-32B en mode thinking (raisonnement logique sur confluences ICT).
//! Identique dans sa philosophie au filtre Rockets : conviction ≥ 65 → valide,
//! sinon le signal est écarté avant toute insertion en base.
use crate::ollama::types::{MODELE_SMC, OLLAMA_URL};
use common::TradingError;
use serde::{Deserialize, Deserializer, Serialize};

/// Tolère que le LLM retourne un float (ex: `74.8`) pour un champ entier.
pub fn deserialiser_conviction<'de, D: Deserializer<'de>>(d: D) -> Result<i64, D::Error> {
    let v = serde_json::Value::deserialize(d)?;
    match v {
        serde_json::Value::Number(n) => Ok(n.as_f64().unwrap_or(0.0) as i64),
        _ => Ok(0),
    }
}

// ── Types publics ─────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize)]
pub struct AjustementsSmc {
    pub sl_suggere: Option<f64>,
    pub tp1_suggere: Option<f64>,
}

#[derive(Serialize, Deserialize)]
pub struct FiltreSMCReponse {
    pub valide: bool,
    #[serde(deserialize_with = "crate::ollama::smc_filtre::deserialiser_conviction")]
    pub conviction: i64, // 0–100 (le LLM peut retourner un float → on tronque)
    pub raison: String,
    pub ajustements: Option<AjustementsSmc>,
}

// ── Données du signal candidat ─────────────────────────────────────────────

pub struct SignalSMCCandidat {
    pub asset: String,
    pub timeframe: String,
    pub direction: String,
    pub score_smc: f64,
    pub confiance_ml: f64,
    pub prix_entree: f64,
    pub stop_loss: f64,
    pub tp1: f64,
    pub atr14: f64,
    pub atr_ratio: f64, // ATR courant / ATR moyen 14
    pub rsi: f64,
    pub kill_zone_active: bool,
    pub sweep_detecte: bool,
}

// ── Extrait d'historique ───────────────────────────────────────────────────

pub struct HistoriqueSMCSignal {
    pub direction: String,
    pub timeframe: String,
    pub score: f64,
    pub statut: String,
}

// ── Prompt ────────────────────────────────────────────────────────────────────

pub const PROMPT_FILTRE_SMC: &str = r#"Tu es un trader institutionnel SMC/ICT expert, spécialiste de la stratégie "SMC Directionnel".

## DÉFINITION DE LA STRATÉGIE SMC DIRECTIONNEL
La stratégie SMC Directionnel génère des signaux directionnels basés sur la confluence de :
- Structure de marché (HH/HL haussier, LH/LL baissier)
- Order Blocks non mitigés alignés avec la direction
- IFVG (Imbalance / Fair Value Gap)
- Fibonacci (niveaux 38.2–61.8%)
- Kill Zone ICT active (London 07h-10h UTC, New York 13h30-16h30 UTC)
- Liquidity Sweep confirmé (faux breakout d'un swing récent avec retour)

## CRITÈRES DE QUALITÉ
Un signal SMC valide DOIT réunir :
- kill_zone_active = true → BLOQUANT si false
- sweep_detecte = true → BLOQUANT si false
- score_smc ≥ 60 → invalider si < 60
- confiance_ml ≥ 0.60 → dégrader fortement si < 0.60
- RSI en zone saine (Long: 30–70, Short: 30–70) → invalider si extrême (>85 ou <15)
- ATR ratio > 0.8 (mouvement en cours, pas de compression) → dégrader si < 0.8

## CRITÈRES D'INVALIDATION STRICTS
- Kill Zone non active → conviction < 30, valide=false IMPÉRATIF
- Sweep non confirmé → conviction < 40, valide=false IMPÉRATIF
- Annonce HIGH impact dans moins de 60 min (FOMC, NFP, CPI…) → valide=false IMPÉRATIF
- RSI > 85 (Long) ou < 15 (Short) → surachat/survente extrême → invalider
- ATR ratio < 0.7 → compression, pas de momentum → invalider
- Score SMC < 50 → structure trop faible → invalider
- R:R < 2:1 (distance TP1 / SL) → configuration défavorable → invalider
- Winrate historique < 40% sur cet asset+timeframe → dégrader fortement

## AJUSTEMENTS SL/TP
Si l'historique montre que le SL ou TP1 sont systématiquement touchés avant l'objectif,
suggère sl_suggere et tp1_suggere en conséquence (basés sur ATR×1.5 ou ATR×2).
Sinon, laisser null.

## FORMAT DE RÉPONSE
Réponds UNIQUEMENT en JSON valide, sans texte avant ni après :
{
  "valide": true | false,
  "conviction": 0-100,
  "raison": "explication courte et factuelle (max 150 caractères)",
  "ajustements": {
    "sl_suggere": <float ou null>,
    "tp1_suggere": <float ou null>
  }
}

## PHILOSOPHIE : QUALITÉ > QUANTITÉ
Tu es conservateur. Il vaut MIEUX passer 0 signal que valider 1 mauvais signal.
En cas de doute → conviction < 70 → valide=false.

## BARÈME CONVICTION
- 80–100 : tous les critères ICT alignés, Kill Zone + Sweep + score élevé → valide=true
- 70–79  : bonne confluence, quelques critères légèrement faibles → valide=true
- < 70   : confluence insuffisante ou critères bloquants → valide=false IMPÉRATIF

Si conviction < 70, retourne valide=false directement, même si certains critères sont positifs."#;

// ── Formatage du contexte ─────────────────────────────────────────────────────

fn formater_contexte(candidat: &SignalSMCCandidat, historique: &[HistoriqueSMCSignal]) -> String {
    let mut ctx = format!(
        "=== SIGNAL CANDIDAT : {} {} ===\n\
        Direction: {} | Score SMC: {:.1}/100 | ML confiance: {:.1}%\n\
        Prix entrée: {:.5} | SL: {:.5} | TP1: {:.5}\n\
        ATR14: {:.5} | ATR ratio: {:.2} | RSI: {:.1}\n\
        Kill Zone active: {} | Sweep détecté: {}\n\n",
        candidat.asset,
        candidat.timeframe,
        candidat.direction,
        candidat.score_smc,
        candidat.confiance_ml * 100.0,
        candidat.prix_entree,
        candidat.stop_loss,
        candidat.tp1,
        candidat.atr14,
        candidat.atr_ratio,
        candidat.rsi,
        candidat.kill_zone_active,
        candidat.sweep_detecte,
    );

    if historique.is_empty() {
        ctx.push_str("=== HISTORIQUE : Aucun signal précédent sur cet asset ===\n");
        return ctx;
    }

    ctx.push_str(&format!(
        "=== HISTORIQUE {} ({} derniers signaux) ===\n",
        candidat.asset,
        historique.len()
    ));

    let succes = historique
        .iter()
        .filter(|s| matches!(s.statut.as_str(), "Fermé" | "TP1" | "TP2" | "TP3"))
        .count();
    let clotls: Vec<_> = historique.iter().filter(|s| s.statut != "Actif").collect();
    if !clotls.is_empty() {
        let wr = succes * 100 / clotls.len().max(1);
        ctx.push_str(&format!("Winrate estimé : {}%\n", wr));
    }

    ctx.push_str("Derniers résultats :\n");
    for s in historique.iter().take(5) {
        ctx.push_str(&format!(
            "  • {} {} score={:.0} → {}\n",
            s.direction, s.timeframe, s.score, s.statut
        ));
    }

    ctx
}

// ── Appel LLM ────────────────────────────────────────────────────────────────

pub async fn filtrer_signal_smc(
    candidat: &SignalSMCCandidat,
    historique: &[HistoriqueSMCSignal],
    few_shot: &str,
) -> Result<FiltreSMCReponse, TradingError> {
    let contexte = formater_contexte(candidat, historique);
    let few_shot_bloc = if few_shot.is_empty() {
        String::new()
    } else {
        format!("\n{few_shot}")
    };
    // /no_think est le soft-switch Qwen3 — pour le filtre SMC on veut le mode thinking complet.
    // On l'indique explicitement en ajoutant /think dans le prompt utilisateur.
    let prompt = format!(
        "{}\n\n{contexte}{few_shot_bloc}\n/think",
        crate::prompts_handler::prompt_effectif("smc_filtre")
    );

    let modele = std::env::var("OLLAMA_MODEL_SMC")
        .unwrap_or_else(|_| MODELE_SMC.to_string());
    let url = std::env::var("OLLAMA_URL").unwrap_or_else(|_| OLLAMA_URL.to_string());

    let corps = serde_json::json!({
        "model": modele,
        "messages": [{"role": "user", "content": prompt}],
        "stream": false,
        // temperature=0.6 recommandé pour le mode thinking Qwen3
        "options": { "temperature": 0.6, "num_predict": 800, "num_gpu": 99, "num_ctx": 8192 }
    });

    let _permit = super::OLLAMA_SEMAPHORE.acquire().await.ok();
    let client = &*super::OLLAMA_HTTP_CLIENT;

    let reponse = client
        .post(&url)
        .json(&corps)
        .send()
        .await
        .map_err(|e| TradingError::Api(format!("Ollama timeout SMC filtre: {}", e)))?;

    if !reponse.status().is_success() {
        return Err(TradingError::Api(format!(
            "Ollama HTTP {} (smc_filtre)",
            reponse.status()
        )));
    }

    let data: super::ReponseOllama = reponse
        .json()
        .await
        .map_err(|e| TradingError::Api(format!("JSON Ollama smc_filtre: {}", e)))?;

    // Filtrer le bloc <think>...</think> produit par Qwen3 en mode thinking
    let texte = super::filtrer_think(data.message.content);
    let debut = texte.find('{').unwrap_or(0);
    let fin = texte.rfind('}').map(|i| i + 1).unwrap_or(texte.len());

    if let Ok(reponse) = serde_json::from_str::<FiltreSMCReponse>(&texte[debut..fin]) {
        return Ok(reponse);
    }

    // Retry avec prompt minimaliste + /no_think (fast path Qwen3, pas besoin de reasoning ici)
    let prompt_retry = format!(
        "Réponds UNIQUEMENT avec ce JSON exact, sans aucun autre texte :\n\
        {{\"valide\": true, \"conviction\": 60, \"raison\": \"...\", \"ajustements\": null}}\n\n\
        Signal SMC : {} {} direction={} score={:.0} confiance_ml={:.2}\n\
        Question : ce signal SMC vaut-il la peine d'être tradé ? /no_think",
        candidat.asset, candidat.timeframe, candidat.direction,
        candidat.score_smc, candidat.confiance_ml
    );
    let corps_retry = serde_json::json!({
        "model": modele,
        "messages": [{"role": "user", "content": prompt_retry}],
        "stream": false,
        // /no_think → mode rapide, pas de raisonnement interne
        "options": { "temperature": 0.7, "num_predict": 64, "num_gpu": 99, "num_ctx": 1024 }
    });

    let reponse_retry = client
        .post(&url)
        .json(&corps_retry)
        .send()
        .await
        .map_err(|e| TradingError::Api(format!("Ollama retry SMC timeout: {}", e)))?;

    let data_retry: super::ReponseOllama = reponse_retry
        .json()
        .await
        .map_err(|e| TradingError::Api(format!("JSON retry SMC Ollama: {}", e)))?;

    let texte_retry = super::filtrer_think(data_retry.message.content);
    let debut_r = texte_retry.find('{').unwrap_or(0);
    let fin_r = texte_retry.rfind('}').map(|i| i + 1).unwrap_or(texte_retry.len());

    serde_json::from_str::<FiltreSMCReponse>(&texte_retry[debut_r..fin_r])
        .map_err(|e| TradingError::Api(format!("JSON smc_filtre non parsable après retry: {}", e)))
}
