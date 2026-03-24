use chrono::{Datelike, Timelike};
use common::Candle;
use db::straddle::PrecisionM5;

// ── Analyse de précision M5 ──────────────────────────────────────────────────
//
// Pour un créneau H1 donné (ex: mardi 14h–16h), on analyse les bougies M5
// históriques à l'intérieur de ce créneau pour trouver :
// - timing_optimal : à quelle minute le pic de volatilité se produit typiquement
// - fenetre_entree : plage de ±2 bougies M5 autour du pic médian
// - whipsaw_minutes : durée du faux mouvement avant le pic (bougies faibles)

/// Parse "14:00" → (14, 0)
fn parse_hhmm(s: &str) -> Option<(u32, u32)> {
    let mut parts = s.splitn(2, ':');
    let h: u32 = parts.next()?.parse().ok()?;
    let m: u32 = parts.next()?.parse().ok()?;
    Some((h, m))
}

fn true_range(prev_close: f64, c: &Candle) -> f64 {
    let hl = c.high - c.low;
    let hc = (c.high - prev_close).abs();
    let lc = (c.low - prev_close).abs();
    hl.max(hc).max(lc)
}

/// Analyse les bougies M5 pour extraire la précision d'entrée sur un créneau.
/// `jour_semaine` : 0=Lundi…6=Dimanche, None=tous les jours
/// `heure_debut` / `heure_fin` : format "HH:MM" UTC
pub fn analyser_precision(
    candles_m5: &[Candle],
    jour_semaine: Option<i64>,
    heure_debut: &str,
    heure_fin: &str,
) -> Option<PrecisionM5> {
    if candles_m5.len() < 2 {
        return None;
    }

    let (h_debut, m_debut) = parse_hhmm(heure_debut)?;
    let (h_fin, m_fin) = parse_hhmm(heure_fin)?;

    // Convertir en minutes depuis minuit pour la comparaison
    let debut_min = h_debut * 60 + m_debut;
    let fin_min = h_fin * 60 + m_fin;

    // Calculer les TR avec la bougie précédente
    let trs: Vec<(i64, u8, f64)> = candles_m5 // (minute_slot, jour, tr)
        .windows(2)
        .filter_map(|w| {
            let prev = &w[0];
            let c = &w[1];
            let jour = c.timestamp.weekday().num_days_from_monday() as i64;
            // Filtre jour de la semaine
            if let Some(j) = jour_semaine {
                if jour != j {
                    return None;
                }
            }
            let heure = c.timestamp.hour();
            let minute = c.timestamp.minute();
            let slot_min = heure * 60 + minute;
            if slot_min < debut_min || slot_min >= fin_min {
                return None;
            }
            let tr = true_range(prev.close, c);
            // Slot = offset en minutes depuis le début du créneau, arrondi à 5
            let offset = (slot_min - debut_min) as i64;
            Some((offset, jour as u8, tr))
        })
        .collect();

    if trs.len() < 5 {
        return None;
    }

    // Grouper les bougies par occurrence du créneau (identifiée par la date)
    // Pour chaque occurrence : trouver le slot avec le TR maximum
    let mut pics_par_occurrence: std::collections::HashMap<String, (i64, f64)> =
        std::collections::HashMap::new();

    for w in candles_m5.windows(2) {
        let prev = &w[0];
        let c = &w[1];
        let jour = c.timestamp.weekday().num_days_from_monday() as i64;
        if let Some(j) = jour_semaine {
            if jour != j {
                continue;
            }
        }
        let heure = c.timestamp.hour();
        let minute = c.timestamp.minute();
        let slot_min = heure * 60 + minute;
        if slot_min < debut_min || slot_min >= fin_min {
            continue;
        }
        let tr = true_range(prev.close, c);
        let offset = (slot_min - debut_min) as i64;
        // Clé = date (YYYY-MM-DD)
        let date_key = c.timestamp.format("%Y-%m-%d").to_string();
        let e = pics_par_occurrence.entry(date_key).or_insert((offset, 0.0));
        if tr > e.1 {
            *e = (offset, tr);
        }
    }

    if pics_par_occurrence.is_empty() {
        return None;
    }

    let nb_occurrences = pics_par_occurrence.len() as i64;

    // Médiane du timing du pic
    let mut offsets_pic: Vec<i64> = pics_par_occurrence.values().map(|(o, _)| *o).collect();
    offsets_pic.sort_unstable();
    let median_offset = offsets_pic[offsets_pic.len() / 2];

    // ATR moyen au moment du pic (bougies dans ±5 minutes autour du pic médian)
    let atr_pic: f64 = {
        let vals: Vec<f64> = trs
            .iter()
            .filter(|(o, _, _)| (o - median_offset).abs() <= 10)
            .map(|(_, _, tr)| *tr)
            .collect();
        if vals.is_empty() {
            0.0
        } else {
            vals.iter().sum::<f64>() / vals.len() as f64
        }
    };

    // Whipsaw estimation : ATR moyen des bougies AVANT le pic médian
    let _atr_avant_pic: f64 = {
        let vals: Vec<f64> = trs
            .iter()
            .filter(|(o, _, _)| *o < median_offset)
            .map(|(_, _, tr)| *tr)
            .collect();
        if vals.is_empty() {
            0.0
        } else {
            vals.iter().sum::<f64>() / vals.len() as f64
        }
    };

    // Nombre de slots M5 (1 slot = 5 min) consécutifs avant le pic où ATR < 0.8 × atr_pic
    // = durée typique du whipsaw/range d'entrée à éviter
    let whipsaw_minutes: i64 = if atr_pic > 0.0 {
        // Chercher depuis combien de minutes avant le pic l'ATR est faible
        let seuil = atr_pic * 0.6;
        let mut count = 0i64;
        let mut offset_check = median_offset - 5;
        while offset_check >= 0 {
            let tr_slot: f64 = trs
                .iter()
                .filter(|(o, _, _)| (o - offset_check).abs() <= 2)
                .map(|(_, _, tr)| *tr)
                .sum::<f64>();
            let n = trs
                .iter()
                .filter(|(o, _, _)| (o - offset_check).abs() <= 2)
                .count();
            if n == 0 || tr_slot / n as f64 > seuil {
                break;
            }
            count += 5;
            offset_check -= 5;
        }
        count
    } else {
        0
    };

    // Construire les timestamps de résultat
    let h_pic = (debut_min + median_offset as u32) / 60;
    let m_pic = (debut_min + median_offset as u32) % 60;
    let timing_optimal = format!("{:02}:{:02}", h_pic, m_pic);

    let fenetre_debut_min = (debut_min as i64 + median_offset - 5).max(0) as u32;
    let fenetre_fin_min = debut_min + (median_offset as u32) + 10;
    let fenetre_entree = format!(
        "{:02}:{:02}–{:02}:{:02}",
        fenetre_debut_min / 60,
        fenetre_debut_min % 60,
        fenetre_fin_min / 60,
        fenetre_fin_min % 60
    );

    Some(PrecisionM5 {
        timing_optimal,
        fenetre_entree,
        whipsaw_minutes,
        nb_occurrences,
        atr_pic,
    })
}
