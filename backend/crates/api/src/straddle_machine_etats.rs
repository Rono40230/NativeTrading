//! Machine à états pour le suivi progressif des jambes Straddle.
//! Gère la progression du SL (Break-Even → TP1 → TP2) et détecte
//! les verdicts terminaux (SL touché, TP3 atteint).

/// Résultat d'une machine à états pour une jambe.
pub(crate) struct EtatJambe<'a> {
    pub sl_courant: f64,
    pub tps_done: Vec<&'a str>,
    pub verdict: Option<(&'a str, f64)>,
    pub etat_change: bool,
}

/// Machine à états unifiée pour une jambe (LONG ou SHORT).
/// `is_long = true` : TP touché quand high >= tp, SL quand low <= sl.
/// `is_long = false`: TP touché quand low <= tp, SL quand high >= sl.
#[allow(clippy::too_many_arguments)]
pub(crate) fn jouer_machine_etats<'a>(
    bougies: &[&common::Candle],
    sl_origine: f64,
    prix_entree: f64,
    tps: &'a [f64],
    is_long: bool,
    vente_partielle: bool,
    id: &str,
    jambe: &str,
) -> Option<EtatJambe<'a>> {
    let (tp1, tp2, tp3) = match tps {
        [a, b, c, ..] => (*a, Some(*b), Some(*c)),
        [a, b] => (*a, Some(*b), None),
        [a] => (*a, None, None),
        _ => return None,
    };
    let tp_labels = ["tp1", "tp2", "tp3"];
    let mut sl_courant = sl_origine;
    let mut tps_done: Vec<&'a str> = Vec::with_capacity(3);
    let mut verdict = None;
    let mut etat_change = false;

    'b: for bougie in bougies {
        let sl_touche = if is_long {
            bougie.low <= sl_courant
        } else {
            bougie.high >= sl_courant
        };
        if sl_touche {
            verdict = Some(("sl", sl_courant));
            break 'b;
        }
        let tp1_touche = if is_long {
            bougie.high >= tp1
        } else {
            bougie.low <= tp1
        };
        if !tps_done.contains(&"tp1") && tp1_touche {
            tps_done.push(tp_labels[0]);
            sl_courant = prix_entree;
            etat_change = true;
            log_tp(id, jambe, "TP1", tp1, vente_partielle);
        }
        if let Some(tp2_val) = tp2 {
            let tp2_touche = if is_long {
                bougie.high >= tp2_val
            } else {
                bougie.low <= tp2_val
            };
            if tps_done.contains(&"tp1") && !tps_done.contains(&"tp2") && tp2_touche {
                tps_done.push(tp_labels[1]);
                sl_courant = tp1;
                etat_change = true;
                log_tp(id, jambe, "TP2", tp2_val, vente_partielle);
            }
        }
        if let Some(tp3_val) = tp3 {
            let tp3_touche = if is_long {
                bougie.high >= tp3_val
            } else {
                bougie.low <= tp3_val
            };
            if tps_done.contains(&"tp2") && tp3_touche {
                verdict = Some(("tp3", tp3_val));
                break 'b;
            }
        }
    }
    Some(EtatJambe {
        sl_courant,
        tps_done,
        verdict,
        etat_change,
    })
}

fn log_tp(id: &str, jambe: &str, tp: &str, prix: f64, vente_partielle: bool) {
    if vente_partielle {
        tracing::info!(
            "📋 Straddle {} jambe {} {} partiel ⅓ @ {:.5}",
            id,
            jambe,
            tp,
            prix
        );
    } else {
        tracing::info!(
            "📋 Straddle {} jambe {} {} atteint, SL progresse (Option 2) @ {:.5}",
            id,
            jambe,
            tp,
            prix
        );
    }
}
