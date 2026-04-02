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
    /// Ratio corps/amplitude de la bougie de signal (0.0–1.0)
    pub ratio_corps: f64,
    /// Tendance préalable confirmée (EMA20 > EMA50)
    pub tendance_haussiere: bool,
    /// Bougies consécutives en compression avant la bougie de signal
    pub nb_bougies_compression: usize,
    /// Range de la zone de consolidation (measured move pour TP1)
    pub hauteur_base: f64,
}

// ── Prompt ────────────────────────────────────────────────────────────────────

pub const PROMPT_FILTRE_ROCKET: &str = r#"Tu es un trader quantitatif expert en crypto, spécialisé dans la stratégie "Rockets".

## DÉFINITION DE LA STRATÉGIE ROCKETS
La stratégie Rockets capture les mouvements explosifs après une compression de volatilité.
Elle repose sur 3 phases successives :

**Phase "prelancement"** (pré-lancement) : L'actif entre en compression — range serré, ATR ratio < 0.80,
volume se contractant. C'est l'énergie qui s'accumule avant le lancement. Plus la compression est longue
et serrée, plus le breakout potentiel est violent.

**Phase "breakout"** : Le prix casse la résistance supérieure de la compression avec conviction —
volume nettement supérieur à la moyenne (ratio_volume > 1.5×), ATR ratio > 1.0 (volatilité en expansion),
bougie de breakout avec momentum (change1h > 0). RSI idéal entre 50 et 75 (momentum sain, pas suracheté).

**Critères de qualité d'un bon setup** :
- Volume ratio ≥ 2.0× = setup fort | 1.5–2.0× = acceptable | < 1.5× = signal faible
- RSI entre 55–75 au breakout = idéal | RSI > 85 = surachat extrême → invalider
- ATR ratio > 1.2 = bonne expansion de volatilité
- Change 1h > 2% = momentum réel | < 0.5% = breakout mou
- `tendance_haussiere=true` (EMA20 > EMA50) = tendance haussière préalable confirmée → +10 conviction
- `nb_bougies_compression ≥ 5` = compression significative (+5) | ≥ 10 = forte (+10) | < 3 = négligeable

**Critères d'invalidation** :
- RSI > 85 : surachat extrême, risque de retournement immédiat
- Série de SL récents sur ce ticker = contexte défavorable
- Phase historiquement à winrate < 40% sur ce ticker = éviter
- Score < 40 : setup de mauvaise qualité
- Volume ratio < 1.3× sur un breakout = fort risque de faux breakout
- Ratio corps/mèche < 0.3 : longue mèche de rejet, clôture loin du haut → invalider ou dégrader conviction
- Ratio corps/mèche > 0.7 : corps fort sans rejet → signal de qualité ✅
- `tendance_haussiere=false` sur un breakout → dégrader conviction de −20 (signal à contre-tendance)
- `nb_bougies_compression < 3` en phase "prelancement" → pas de vraie compression → invalider
- Consolidation chaotique : `nb_bougies_compression` faible + `ratio_corps` < 0.4 = structure instable → dégrader
- Faux breakout : si le prix actuel est inférieur au niveau de cassure (target20) → invalider

## COEFFICIENTS ATR ACTUELS
SL = entrée − 1×ATR14 | TP1 = entrée + hauteur_base (measured move) si hauteur_base > ATR14, sinon + 1×ATR14 | TP2 = entrée + 2×ATR14 | TP3 = entrée + 20×ATR14
Si les données historiques montrent que ces niveaux sont trop serrés ou trop larges sur ce ticker,
suggère un SL ou TP1 ajusté. Le measured move (hauteur_base = range de consolidation) est plus fidèle
à la stratégie Rockets originale.

## FORMAT DE RÉPONSE
Réponds UNIQUEMENT en JSON valide, sans texte avant ou après :
{
  "valide": true | false,
  "conviction": 0-100,
  "raison": "explication courte et factuelle (max 120 caractères)",
  "ajustements": {
    "sl_suggere": <float ou null>,
    "tp1_suggere": <float ou null>
  }
}

## PHILOSOPHIE : QUALITÉ > QUANTITÉ
Tu es conservateur. Il vaut MIEUX passer 0 signal que valider 1 mauvais signal.
En cas de doute → mettre valide=false. Ne valide que ce qui te semble SOLIDE.

## BARÈME CONVICTION
- 80–100 : setup excellent, tous les critères alignés → valide=true
- 65–79  : bon setup, quelques critères légèrement en dessous → valide=true
- < 65   : setup insuffisant ou incertain → valide=false IMPÉRATIF

Si la conviction serait < 65 même avec valide=true, retourne valide=false directement.
Si pas d'historique sur ce ticker, évalue uniquement sur les critères techniques actuels.
Ne suggère sl_suggere ou tp1_suggere que si l'ajustement est justifié par des données concrètes."#;

// ── Formatage du contexte ─────────────────────────────────────────────────────

fn formater_contexte(candidat: &SignalCandidat, historique: &[RocketSignal]) -> String {
    let mut ctx = format!(
        "=== SIGNAL CANDIDAT : {} ===\n\
        Phase: {} | Score: {}/100\n\
        Prix entrée: {:.6} | SL: {:.6} | TP1: {:.6}\n\
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

// ── Appel LLM avec timeout 5s ─────────────────────────────────────────────────

pub async fn filtrer_signal(
    candidat: &SignalCandidat,
    historique: &[RocketSignal],
) -> Result<FiltreReponse, TradingError> {
    let contexte = formater_contexte(candidat, historique);
    let prompt = format!("{}\n\n{contexte}", crate::prompts_handler::prompt_effectif("rockets_filtre"));

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
