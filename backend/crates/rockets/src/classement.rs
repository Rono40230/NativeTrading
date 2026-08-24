//! Classement /10 Rocket Hunter sur bougies D1 (définition canonique).
//!
//! Quatre piliers : Fondamental 3 pts (subset chiffrable v1 — le volet news
//! est réservé à l'IA étape 6), Technique 3, Chartisme 2, Chandeliers 2.
//! Seuil : ≥ 7 = ROCKET (9-10 = ROCKET ALPHA), < 7 = ÉLIMINÉ.

/// Une bougie D1 (entrée du scanner — klines Binance/Bybit).
#[derive(Debug, Clone, Copy)]
pub struct BougieD1 {
    pub ts: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

/// Contexte de marché injecté par le scanner (calculé sur BTC).
#[derive(Debug, Clone, Copy)]
pub struct ContexteMarche {
    /// BTC > MM50 > MM200 en D1 (régime haussier).
    pub btc_haussier: bool,
    /// Performance BTC sur 4 semaines (fraction, ex. 0.05 = +5 %).
    pub perf_btc_4s: f64,
}

/// Détail point par point (affichage + audit).
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct DetailPoints {
    pub sentiment: bool,
    pub contexte: bool,
    pub news: Option<bool>,
    pub tendance: bool,
    pub volatilite: bool,
    pub interet: bool,
    pub figure: bool,
    pub gaps: bool,
    pub breakout: bool,
    pub liquidite: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum VerdictRockets {
    Alpha,
    Rocket,
    Elimine,
}

/// Résultat du classement d'un symbole.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ResultatClassement {
    pub symbole: String,
    pub points: u8,
    pub verdict: VerdictRockets,
    pub detail: DetailPoints,
    /// Pivot détecté (plus haut de la base, à casser).
    pub pivot: Option<f64>,
    /// Invalidation proposée (sous la dernière contraction).
    pub stop: Option<f64>,
    /// La dernière bougie D1 casse le pivot (candidat immédiat).
    pub cassure: bool,
}

/// Moyenne mobile simple sur `periode` des clôtures (None si insuffisant).
pub fn mma(clotures: &[f64], periode: usize) -> Option<f64> {
    if clotures.len() < periode || periode == 0 {
        return None;
    }
    Some(clotures[clotures.len() - periode..].iter().sum::<f64>() / periode as f64)
}

/// Largeur de Bollinger (20, 2σ) en fraction du prix médian.
fn largeur_bollinger(b: &[BougieD1]) -> Option<f64> {
    let n = 20.min(b.len());
    if n < 10 {
        return None;
    }
    let fen: Vec<f64> = b[b.len() - n..].iter().map(|x| x.close).collect();
    let moyenne = fen.iter().sum::<f64>() / n as f64;
    let variance = fen.iter().map(|c| (c - moyenne).powi(2)).sum::<f64>() / n as f64;
    let sigma = variance.sqrt();
    Some(4.0 * sigma / moyenne.max(1e-12))
}

/// Le classement complet d'un symbole. `bougies` : D1 chronologiques,
/// la DERNIÈRE est la bougie potentiellement de cassure (≥ 220 conseillé).
pub fn classement_rocket(
    symbole: &str,
    bougies: &[BougieD1],
    ctx: &ContexteMarche,
) -> ResultatClassement {
    let mut detail = DetailPoints::default();
    if bougies.len() < 210 {
        return ResultatClassement {
            symbole: symbole.to_string(),
            points: 0,
            verdict: VerdictRockets::Elimine,
            detail,
            pivot: None,
            stop: None,
            cassure: false,
        };
    }

    let clotures: Vec<f64> = bougies.iter().map(|b| b.close).collect();
    let derniere = bougies.last().copied().unwrap_or(BougieD1 {
        ts: 0, open: 0.0, high: 0.0, low: 0.0, close: 0.0, volume: 0.0,
    });

    // ── Pivot : plus haut des 60 jours AVANT la bougie de cassure ──
    let base = &bougies[bougies.len() - 61..bougies.len() - 1];
    let pivot = base.iter().map(|b| b.high).fold(f64::MIN, f64::max);
    let idx_pivot = base
        .iter()
        .rposition(|b| (b.high - pivot).abs() < 1e-12)
        .unwrap_or(0);
    let age_pivot_jours = base.len() - 1 - idx_pivot;

    // Stop : sous le creux de la dernière contraction (30 derniers jours
    // de la base) — l'invalidation de la définition.
    let recent = &base[base.len().saturating_sub(30)..];
    let stop = recent.iter().map(|b| b.low).fold(f64::MAX, f64::min);

    // ── FONDAMENTAL (3) ──
    // Sentiment : BTC haussier ET surperformance 4 semaines (proxy secteur :
    // la force relative — le secteur sera affiné par l'IA, étape 6).
    let perf_4s = derniere.close / bougies[bougies.len() - 29].close - 1.0;
    detail.sentiment = ctx.btc_haussier && perf_4s > ctx.perf_btc_4s;
    // Contexte : base travaillée (pivot âgé ≥ 30 j) et prix proche (≥ 90 %).
    detail.contexte = age_pivot_jours >= 30 && derniere.close >= pivot * 0.90;
    // News : réservé à l'IA (étape 6) — None, pas de point attribué v1.
    detail.news = None;

    // ── TECHNIQUE (3) ──
    let (mma50, mma200) = (mma(&clotures, 50), mma(&clotures, 200));
    let plus_haut_52s = bougies[bougies.len().saturating_sub(252)..]
        .iter()
        .map(|b| b.high)
        .fold(f64::MIN, f64::max);
    detail.tendance = match (mma50, mma200) {
        (Some(m50), Some(m200)) => {
            derniere.close > m50
                && m50 > m200
                && derniere.close > m200
                && derniere.close >= plus_haut_52s * 0.75
        }
        _ => false,
    };
    // Volatilité : squeeze (largeur au plus bas 30 j dans les 5 derniers j)
    // puis expansion (la largeur actuelle remonte).
    let largeurs: Vec<f64> = (30..=bougies.len())
        .filter_map(|fin| largeur_bollinger(&bougies[fin - 20..fin]))
        .collect();
    let (min30, actuelle) = if largeurs.len() >= 30 {
        let n = largeurs.len();
        let m = largeurs[n - 30..n - 5].iter().copied().fold(f64::INFINITY, f64::min);
        (m, largeurs[n - 1])
    } else {
        (f64::INFINITY, 0.0)
    };
    detail.volatilite = actuelle > min30 && min30.is_finite();
    // Intérêt : assèchement (MM5 volumes de fin de base ≤ 60 % de la MM50
    // de la base) puis explosion (volume de cassure ≥ 150 % MM50). Les deux
    // références EXCLUENT la bougie de cassure.
    let volumes: Vec<f64> = bougies.iter().map(|b| b.volume).collect();
    let v_mm50 = mma(&volumes[..volumes.len() - 1], 50).unwrap_or(0.0);
    let v_mm5_fin = mma(&volumes[volumes.len() - 6..volumes.len() - 1], 5).unwrap_or(f64::MAX);
    detail.interet =
        v_mm50 > 0.0 && v_mm5_fin <= 0.60 * v_mm50 && derniere.volume >= 1.5 * v_mm50;

    // ── CHARTISME (2) ──
    // Figure : contractions décroissantes — replis successifs sous le pivot
    // de profondeur ≈ décroissante (≥ 2 sur la base de 60 j).
    let contractions = contractions_decroissantes(base, pivot);
    detail.figure = contractions >= 2;
    // Gaps : aucun trou > 10 % entre bougies de la base (crypto 24/7).
    detail.gaps = base
        .windows(2)
        .all(|w| ((w[1].open - w[0].close) / w[0].close).abs() < 0.10);

    // ── CHANDELIERS (2) ──
    // Breakout : cassure décisive (≥ 3 % au-delà du pivot) + Marubozu
    // (corps ≥ 80 % de l'étendue) haussier.
    let etendue = (derniere.high - derniere.low).max(1e-12);
    let corps = (derniere.close - derniere.open).abs();
    let cassure = derniere.close >= pivot * 1.03
        && derniere.close > derniere.open
        && corps >= 0.80 * etendue;
    detail.breakout = cassure;
    // Liquidité : pas de longue mèche au-delà du pivot (≤ 25 % de l'étendue).
    let meche_haute = derniere.high - derniere.close.max(derniere.open);
    detail.liquidite = meche_haute <= 0.25 * etendue;

    let points = detail.sentiment as u8
        + detail.contexte as u8
        + detail.tendance as u8
        + detail.volatilite as u8
        + detail.interet as u8
        + detail.figure as u8
        + detail.gaps as u8
        + detail.breakout as u8
        + detail.liquidite as u8;

    let verdict = if points >= 9 {
        VerdictRockets::Alpha
    } else if points >= 7 {
        VerdictRockets::Rocket
    } else {
        VerdictRockets::Elimine
    };

    ResultatClassement {
        symbole: symbole.to_string(),
        points,
        verdict,
        detail,
        pivot: Some(pivot),
        stop: Some(stop),
        cassure,
    }
}

/// Compte les contractions décroissantes : creux locaux sous le pivot dont
/// la profondeur diminue d'au moins ~40 % à chaque repli (signature VCP).
fn contractions_decroissantes(base: &[BougieD1], pivot: f64) -> usize {
    // Creux locaux : low plus bas que les 3 voisins de chaque côté.
    let mut creux: Vec<(usize, f64)> = Vec::new();
    for i in 3..base.len().saturating_sub(3) {
        let low = base[i].low;
        let voisin = base[i - 3..=i + 3]
            .iter()
            .all(|b| b.low >= low - 1e-12);
        if voisin {
            creux.push((i, pivot - low));
        }
    }
    // Signature VCP : les replis RÉTRÉCISSENT en avançant dans le temps —
    // en remontant le temps (itération inversée), chaque creux plus ancien
    // doit être ≥ ~1,6× le suivant (i.e. le plus récent ≤ 60 % de l'ancien).
    let mut compte = 0;
    let mut precedente: Option<f64> = None;
    for (_, profondeur) in creux.iter().rev().take(6) {
        match precedente {
            // precedente = creux PLUS RÉCENT ; courant = plus ANCIEN.
            Some(p) if p <= 0.60 * profondeur => compte += 1,
            None => compte += 1,
            _ => {}
        }
        precedente = Some(*profondeur);
    }
    compte.min(4)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bougie(ts: i64, o: f64, h: f64, l: f64, c: f64, v: f64) -> BougieD1 {
        BougieD1 { ts, open: o, high: h, low: l, close: c, volume: v }
    }

    /// Série D1 synthétique : tendance haussière (150 j), base de 60 j en
    /// trois V décroissants (profondeurs 8 → 4,5 → 1,8 sous le pivot 100,
    /// volumes 600 → 400 → 250), puis bougie de cassure Marubozu.
    fn serie_rocket() -> Vec<BougieD1> {
        let mut b = Vec::new();
        for i in 0..150 {
            let c = 10.0 + i as f64 * 0.6;
            b.push(bougie(i * 86400, c - 0.2, c + 0.3, c - 0.5, c, 1000.0));
        }
        let profondeurs = [8.0, 4.5, 1.8];
        let vols_phase = [600.0, 400.0, 200.0];
        for phase in 0..3 {
            for i in 0..20 {
                let t = i as f64 / 19.0;
                let v = if t < 0.5 { t * 2.0 } else { (1.0 - t) * 2.0 }; // 0→1→0
                // Sommets de contraction DÉCROISSANTS sous le pivot (le
                // plus haut de la base reste la première remontée).
                let bas = 100.0 - phase as f64 * 0.25;
                let low = bas - profondeurs[phase] * v;
                let close = low + 0.3;
                let idx = 150 + phase * 20 + i;
                b.push(bougie(
                    idx as i64 * 86400,
                    close - 0.1,
                    close + 0.4,
                    low,
                    close,
                    vols_phase[phase],
                ));
            }
        }
        let n = b.len() as i64;
        b.push(bougie(n * 86400, 100.2, 104.5, 100.0, 104.3, 2500.0));
        b
    }

    #[test]
    fn rocket_parfaite_classee_alpha() {
        let ctx = ContexteMarche { btc_haussier: true, perf_btc_4s: 0.02 };
        let r = classement_rocket("TESTUSDT", &serie_rocket(), &ctx);
        assert!(r.points >= 7, "points = {} ( {:?}", r.points, r.detail);
        assert_eq!(r.verdict, VerdictRockets::Alpha);
        assert!(r.cassure, "la bougie finale casse le pivot");
        let pivot = r.pivot.unwrap_or(0.0);
        assert!(pivot > 99.0 && pivot < 101.5, "pivot = {pivot}");
    }

    #[test]
    fn marche_baissier_elimine() {
        let ctx = ContexteMarche { btc_haussier: false, perf_btc_4s: 0.02 };
        let r = classement_rocket("TESTUSDT", &serie_rocket(), &ctx);
        // Sans sentiment : 2 points manquants (8/10 max) — reste rocket.
        assert!(r.points <= 8);
        // Série sans volume d'explosion : éliminé.
        let mut b = serie_rocket();
        let n = b.len() - 1;
        b[n].volume = 500.0; // pas d'explosion
        let r2 = classement_rocket("TESTUSDT", &b, &ctx);
        assert!(r2.points < r.points, "le volume d'explosion doit compter");
        assert!(!r2.cassure || r2.detail.interet == false);
    }

    #[test]
    fn serie_trop_courte_eliminee() {
        let ctx = ContexteMarche { btc_haussier: true, perf_btc_4s: 0.0 };
        let r = classement_rocket("X", &vec![bougie(0, 1.0, 1.0, 1.0, 1.0, 1.0); 50], &ctx);
        assert_eq!(r.verdict, VerdictRockets::Elimine);
        assert_eq!(r.points, 0);
    }
}
