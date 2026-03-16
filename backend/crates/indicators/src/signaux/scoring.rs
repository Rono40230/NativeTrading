use std::collections::HashMap;

use super::types::{DirectionSignal, NiveauForce, SignalIndicateur};

/// Calcule la confluence : si ≥2 signaux de même direction sur le même timestamp,
/// tous ceux de cette direction sont promus à `Fort`.
/// Les signaux en conflit (bull ET bear) conservent leur niveau d'origine.
pub fn calculer_confluence(mut signaux: Vec<SignalIndicateur>) -> Vec<SignalIndicateur> {
    // Compter les directions par timestamp
    let mut comptes: HashMap<i64, (usize, usize)> = HashMap::new(); // (bull, bear)
    for s in &signaux {
        let entry = comptes.entry(s.timestamp).or_insert((0, 0));
        match s.direction {
            DirectionSignal::Bullish => entry.0 += 1,
            DirectionSignal::Bearish => entry.1 += 1,
            DirectionSignal::Neutre => {}
        }
    }
    // Upgrader la force des signaux avec confluence pure (pas de conflit)
    for s in &mut signaux {
        if s.direction == DirectionSignal::Neutre {
            continue;
        }
        let (bull, bear) = comptes[&s.timestamp];
        // Pas de confluence si signaux contradictoires
        if bull > 0 && bear > 0 {
            continue;
        }
        let count = match s.direction {
            DirectionSignal::Bullish => bull,
            DirectionSignal::Bearish => bear,
            DirectionSignal::Neutre => 0,
        };
        if count >= 2 {
            s.force = NiveauForce::Fort;
        }
    }
    signaux
}
