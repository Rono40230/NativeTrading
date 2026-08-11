use crate::ollama::types::{MODELE_DEFAUT, OLLAMA_URL};
use chrono::{Datelike, Timelike};
use common::{Candle, TradingError};
use db::straddle::NouveauCreneau;
use serde::Deserialize;

// ── Types ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct StatSlot {
    pub jour: u8,  // 0=Lundi...6=Dimanche
    pub heure: u8, // 0–23 UTC
    pub atr_ratio: f64,
    pub nb_occurrences: u32,
    pub pct_depasse_seuil: f64, // fraction : 0.0–1.0
}

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

// ── Calcul des stats ATR par créneau ─────────────────────────────────────────

/// Calcule l'ATR(1) bougie par bougie (True Range).
fn true_range(prev_close: f64, candle: &Candle) -> f64 {
    let hl = candle.high - candle.low;
    let hc = (candle.high - prev_close).abs();
    let lc = (candle.low - prev_close).abs();
    hl.max(hc).max(lc)
}

/// Calcule l'ATR moyen global des bougies.
fn atr_global(candles: &[Candle]) -> f64 {
    if candles.len() < 2 {
        return 0.0;
    }
    let trs: Vec<f64> = candles
        .windows(2)
        .map(|w| true_range(w[0].close, &w[1]))
        .collect();
    trs.iter().sum::<f64>() / trs.len() as f64
}

/// Retourne les statistiques ATR par (jour_semaine × heure_UTC).
pub fn calculer_stats(candles: &[Candle]) -> (Vec<StatSlot>, f64) {
    if candles.len() < 2 {
        return (vec![], 0.0);
    }

    let atr_ref = atr_global(candles);
    if atr_ref == 0.0 {
        return (vec![], 0.0);
    }

    // Accumuler par (jour, heure)
    let mut sommes: std::collections::HashMap<(u8, u8), (f64, u32, u32)> =
        std::collections::HashMap::new();

    for w in candles.windows(2) {
        let c = &w[1];
        let tr = true_range(w[0].close, c);
        let jour = c.timestamp.weekday().num_days_from_monday() as u8; // 0=Lundi
        let heure = c.timestamp.hour() as u8;
        let e = sommes.entry((jour, heure)).or_insert((0.0, 0, 0));
        e.0 += tr;
        e.1 += 1;
        if tr > atr_ref * 1.4 {
            e.2 += 1;
        }
    }

    let seuil_min = if candles.len() >= 1000 {
        4u32
    } else if candles.len() >= 400 {
        3u32
    } else {
        2u32 // peu de données : accepter 2 occurrences minimales
    };
    let mut stats: Vec<StatSlot> = sommes
        .into_iter()
        .filter(|(_, (_, nb, _))| *nb >= seuil_min)
        .map(|((jour, heure), (somme, nb, depasse))| StatSlot {
            jour,
            heure,
            atr_ratio: (somme / nb as f64) / atr_ref,
            nb_occurrences: nb,
            pct_depasse_seuil: depasse as f64 / nb as f64,
        })
        .collect();

    stats.sort_by(|a, b| {
        b.atr_ratio
            .partial_cmp(&a.atr_ratio)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    (stats, atr_ref)
}

// ── Formatage contexte LLM ───────────────────────────────────────────────────

pub fn formater_contexte_straddle(
    asset: &str,
    periode_mois: u32,
    stats: &[StatSlot],
    atr_ref: f64,
    nb_bougies: usize,
    annonces_imminentes: &[serde_json::Value],
) -> String {
    let jours = [
        "Lundi", "Mardi", "Mercredi", "Jeudi", "Vendredi", "Samedi", "Dimanche",
    ];

    let mut ctx = format!(
        "Asset: {asset}\nPériode analysée: {periode_mois} mois (~{nb_bougies} bougies H1)\n\
         ATR global de référence: {atr_ref:.5}\n\n\
         Top 35 créneaux par ratio ATR (ATR créneau / ATR global)\n\
         Jour       | Heure UTC | Ratio ATR | Fréq. > 1.4× | Occurrences\n\
         -----------|-----------|-----------|--------------|------------\n"
    );

    for s in stats.iter().take(35) {
        ctx.push_str(&format!(
            "{:<10} | {:>5}h UTC | {:>8.2}× | {:>11.0}% | {:>11}\n",
            jours[s.jour as usize % 7],
            s.heure,
            s.atr_ratio,
            s.pct_depasse_seuil * 100.0,
            s.nb_occurrences,
        ));
    }
    if !annonces_imminentes.is_empty() {
        ctx.push_str("\n\nAnnonces économiques HIGH impact dans les 2h :\n");
        for a in annonces_imminentes {
            let heure = a["date_heure"].as_str().unwrap_or("?");
            let devise = a["devise"].as_str().unwrap_or("?");
            let titre = a["titre"].as_str().unwrap_or("?");
            let prevision = a["prevision"].as_str().unwrap_or("n/a");
            ctx.push_str(&format!(
                "  - {heure} | {devise} | {titre} | Prévis.: {prevision}\n"
            ));
        }
    }
    ctx
}

// ── Appel LLM ────────────────────────────────────────────────────────────────

pub const PROMPT_ANALYSE_STRADDLE: &str = r#"Tu es un expert en trading quantitatif spécialisé dans les stratégies de volatilité bidirectionnelle (Straddle).

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
    annonces_imminentes: &[serde_json::Value],
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

    let contexte = formater_contexte_straddle(
        asset,
        periode_mois,
        &stats,
        atr_ref,
        candles.len(),
        annonces_imminentes,
    );
    let prompt = format!(
        "{}\n\n## DONNÉES\n{contexte}",
        crate::prompt_effectif("straddle_analyse")
    );

    let url = std::env::var("OLLAMA_URL").unwrap_or_else(|_| OLLAMA_URL.to_string());

    let corps = serde_json::json!({
        "model": MODELE_DEFAUT,
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
        .map_err(|e| TradingError::Api(format!("Ollama injoignable: {e}")))?;

    if !reponse.status().is_success() {
        return Err(TradingError::Api(format!(
            "Ollama HTTP {}",
            reponse.status()
        )));
    }

    let data: super::ReponseOllama = reponse
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
