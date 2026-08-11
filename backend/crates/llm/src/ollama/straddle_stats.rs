use chrono::{Datelike, Timelike};
use common::Candle;

/// Statistiques ATR agrégées par créneau (jour × heure UTC).
#[derive(Debug, Clone)]
pub struct StatSlot {
    pub jour: u8,       // 0=Lundi…6=Dimanche
    pub heure: u8,      // 0–23 UTC
    pub atr_ratio: f64, // ATR créneau / ATR global
    pub nb_occurrences: u32,
    pub pct_depasse_seuil: f64, // fraction 0.0–1.0
}

/// ATR(1) : True Range d'une bougie par rapport à la clôture précédente.
pub fn true_range(prev_close: f64, candle: &Candle) -> f64 {
    let hl = candle.high - candle.low;
    let hc = (candle.high - prev_close).abs();
    let lc = (candle.low - prev_close).abs();
    hl.max(hc).max(lc)
}

/// ATR moyen global sur l'ensemble des bougies.
pub fn atr_global(candles: &[Candle]) -> f64 {
    if candles.len() < 2 {
        return 0.0;
    }
    let trs: Vec<f64> = candles
        .windows(2)
        .map(|w| true_range(w[0].close, &w[1]))
        .collect();
    trs.iter().sum::<f64>() / trs.len() as f64
}

/// Retourne les statistiques ATR par (jour_semaine × heure_UTC) + ATR global de référence.
pub fn calculer_stats(candles: &[Candle]) -> (Vec<StatSlot>, f64) {
    if candles.len() < 2 {
        return (vec![], 0.0);
    }
    let atr_ref = atr_global(candles);
    if atr_ref == 0.0 {
        return (vec![], 0.0);
    }

    let mut sommes: std::collections::HashMap<(u8, u8), (f64, u32, u32)> =
        std::collections::HashMap::new();

    for w in candles.windows(2) {
        let c = &w[1];
        let tr = true_range(w[0].close, c);
        let jour = c.timestamp.weekday().num_days_from_monday() as u8;
        let heure = c.timestamp.hour() as u8;
        let e = sommes.entry((jour, heure)).or_insert((0.0, 0, 0));
        e.0 += tr;
        e.1 += 1;
        if tr > atr_ref * 1.4 {
            e.2 += 1;
        }
    }

    // Seuil adaptatif selon volume de données
    let seuil_min: u32 = if candles.len() >= 1000 {
        4
    } else if candles.len() >= 400 {
        3
    } else {
        2
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

/// Formate le contexte textuel pour le LLM (top 35 créneaux).
pub fn formater_contexte_straddle(
    asset: &str,
    periode_mois: u32,
    stats: &[StatSlot],
    atr_ref: f64,
    nb_bougies: usize,
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
    ctx
}
