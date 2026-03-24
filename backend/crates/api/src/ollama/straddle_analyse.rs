use crate::ollama::straddle_stats::{calculer_stats, formater_contexte_straddle};
use crate::ollama::types::{MODELE_DEFAUT, OLLAMA_URL};
use common::{Candle, TradingError};
use db::straddle::NouveauCreneau;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Deserialize)]
struct CreneauBrut {
    jour_semaine: Option<i64>,
    heure_debut: String,
    heure_fin: String,
    atr_moyen: Option<f64>,
    frequence: Option<f64>,
    llm_conviction: i64,
    llm_raison: String,
}

#[derive(Deserialize)]
struct OllamaResp {
    message: OllamaMsg,
}
#[derive(Deserialize)]
struct OllamaMsg {
    content: String,
}

// ── Appel LLM ────────────────────────────────────────────────────────────────

const PROMPT_ANALYSE_STRADDLE: &str = r#"Tu es un expert en trading quantitatif spécialisé dans les stratégies de volatilité bidirectionnelle (Straddle).

## MISSION
Analyser les statistiques de volatilité horaire fournies et identifier les **créneaux récurrents de forte volatilité** qui méritent d'être testés en backtest avec une stratégie bidirectionnelle (Long + Short simultané).

Un bon créneau Straddle est un moment où le marché bouge fortement dans une direction OU l'autre de façon **récurrente et prévisible**.

## CRITÈRES D'UN BON CRÉNEAU STRADDLE
- Ratio ATR ≥ 1.4× (le créneau est 40%+ plus volatile que la moyenne)
- Fréquence de dépassement ≥ 50% des occurrences
- Idéalement coïncide avec des événements calendrier récurrents :
  - Ouverture de Londres (8h–9h UTC)
  - Ouverture de New York (13h–14h UTC)
  - Publications NFP (1er vendredi du mois, 13h30 UTC)
  - Publications CPI/PPI/FOMC (13h30 ou 19h UTC)
  - Ouverture de Tokyo (0h–1h UTC)

## FORMAT DE RÉPONSE
Réponds UNIQUEMENT en JSON valide — un tableau de créneaux, sans texte avant ou après :
[
  {
    "jour_semaine": 1,       (0=Lundi...4=Vendredi, null=tous les jours)
    "heure_debut": "14:00",  (UTC)
    "heure_fin": "16:00",    (UTC)
    "atr_moyen": 0.85,       (ratio ATR observé)
    "frequence": 0.72,       (0.0–1.0)
    "llm_conviction": 78,    (0–100 — ta conviction que ce créneau mérite un test)
    "llm_raison": "Ouverture NY + corrélation publications hebdomadaires"
  }
]

## PHILOSOPHIE : QUALITÉ > QUANTITÉ
- Propose au **maximum 5 créneaux** — préfère les meilleurs
- conviction < 65 → NE PAS inclure dans les résultats
- Fréquence < 40% → invalider même si ratio ATR élevé
- Si aucun créneau ne mérite d'être testé → retourne []
- En cas de doute → ne pas inclure"#;

pub async fn analyser_creneaux(
    asset: &str,
    periode_mois: u32,
    candles: &[Candle],
) -> Result<Vec<NouveauCreneau>, TradingError> {
    if candles.len() < 10 {
        return Err(TradingError::Data(
            "Pas assez de bougies pour analyser les créneaux".into(),
        ));
    }

    let (stats, atr_ref) = calculer_stats(candles);
    if stats.is_empty() {
        return Ok(vec![]);
    }

    let contexte = formater_contexte_straddle(asset, periode_mois, &stats, atr_ref, candles.len());
    let prompt = format!("{PROMPT_ANALYSE_STRADDLE}\n\n## DONNÉES\n{contexte}");

    let url = std::env::var("OLLAMA_URL").unwrap_or_else(|_| OLLAMA_URL.to_string());

    #[derive(Serialize)]
    struct Corps<'a> {
        model: &'a str,
        messages: Vec<serde_json::Value>,
        stream: bool,
    }

    let corps = Corps {
        model: MODELE_DEFAUT,
        messages: vec![serde_json::json!({ "role": "user", "content": prompt })],
        stream: false,
    };

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(|e| TradingError::Api(e.to_string()))?;

    let reponse = client
        .post(&url)
        .json(&corps)
        .send()
        .await
        .map_err(|e| TradingError::Api(format!("Ollama injoignable: {e}")))?;

    if !reponse.status().is_success() {
        return Err(TradingError::Api(format!(
            "Ollama HTTP {}",
            reponse.status()
        )));
    }

    let data: OllamaResp = reponse
        .json()
        .await
        .map_err(|e| TradingError::Api(format!("Réponse Ollama invalide: {e}")))?;

    let texte = data.message.content;
    let debut = texte.find('[').unwrap_or(0);
    let fin = texte.rfind(']').map(|i| i + 1).unwrap_or(texte.len());

    let bruts: Vec<CreneauBrut> = serde_json::from_str(&texte[debut..fin]).map_err(|e| {
        TradingError::Api(format!(
            "JSON créneaux LLM non parsable: {e} — texte: {}",
            &texte[..texte.len().min(300)]
        ))
    })?;

    let creneaux = bruts
        .into_iter()
        .filter(|c| c.llm_conviction >= 65)
        .map(|c| NouveauCreneau {
            asset: asset.to_string(),
            jour_semaine: c.jour_semaine,
            heure_debut: c.heure_debut,
            heure_fin: c.heure_fin,
            atr_moyen: c.atr_moyen,
            frequence: c.frequence,
            llm_raison: Some(c.llm_raison),
            llm_conviction: Some(c.llm_conviction),
        })
        .collect();

    Ok(creneaux)
}
