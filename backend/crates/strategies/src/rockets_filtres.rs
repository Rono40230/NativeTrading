//! Filtres qualitatifs professionnels pour la stratégie Rockets.
//!
//! Inspiré du VCP (Volatility Contraction Pattern — Mark Minervini) :
//! - Assèchement du volume en compression → signal de distribution absente
//! - Progressivité des contractions → setup VCP authentique
//! - ATR long terme → distinguer expansion réelle vs. bruit normal

// ── ATR référence long terme ─────────────────────────────────────────────────

/// ATR Wilder sur 50 périodes — référence long terme.
/// Permet de distinguer une vraie expansion de volatilité (ATR14 > ATR50 × 1.2)
/// d'un simple bruit aléatoire. Nécessite >= 52 bougies (50 TRs + 1 init).
pub fn calc_atr50(highs: &[f64], lows: &[f64], closes: &[f64]) -> f64 {
    let n = highs.len().min(lows.len()).min(closes.len());
    if n < 2 {
        return 0.0;
    }
    let trs: Vec<f64> = (1..n)
        .map(|i| {
            let p = closes[i - 1];
            [
                highs[i] - lows[i],
                (highs[i] - p).abs(),
                (lows[i] - p).abs(),
            ]
            .into_iter()
            .fold(f64::NEG_INFINITY, f64::max)
        })
        .collect();

    const P: usize = 50;
    if trs.len() >= P {
        let mut val = trs[..P].iter().sum::<f64>() / P as f64;
        for &tr in &trs[P..] {
            val = (val * (P as f64 - 1.0) + tr) / P as f64;
        }
        val
    } else if !trs.is_empty() {
        trs.iter().sum::<f64>() / trs.len() as f64
    } else {
        0.0
    }
}

// ── Assèchement du volume (VCP key signal) ───────────────────────────────────

/// Ratio entre le volume moyen pendant la compression et le volume moyen sur 20p avant.
///
/// < 0.75 = volume qui se "sèche" correctement → signal VCP valide, +15 pts score  
/// < 0.55 = assèchement fort → setup VCP premium  
/// > 1.0  = volume toujours présent = compression technique seulement, pas institutionnelle
pub fn calc_volume_seche(
    volumes: &[f64],
    nb_compression: usize,
    lookback: usize,
) -> f64 {
    let n = volumes.len();
    if n < 2 || nb_compression == 0 {
        return 1.0;
    }
    // Bougies de compression = les [nb_compression] dernières (hors bougie courante)
    let comp_end = n.saturating_sub(1);
    let comp_start = comp_end.saturating_sub(nb_compression);
    if comp_end <= comp_start {
        return 1.0;
    }
    let vol_compression = volumes[comp_start..comp_end].iter().sum::<f64>()
        / (comp_end - comp_start) as f64;

    // Baseline = [lookback] bougies précédant la zone de compression
    let base_end = comp_start;
    let base_start = base_end.saturating_sub(lookback);
    if base_end <= base_start {
        return 1.0;
    }
    let vol_base = volumes[base_start..base_end].iter().sum::<f64>()
        / (base_end - base_start) as f64;

    if vol_base > 0.0 {
        vol_compression / vol_base
    } else {
        1.0
    }
}

// ── Qualité de contraction VCP ────────────────────────────────────────────────

/// Score de progressivité des contractions (pattern VCP de Minervini).
///
/// Chaque bougie de compression doit avoir un range ≤ 90% du range de la précédente.
/// Retourne 0.0 (aucune progressivité) → 1.0 (contractions uniformément décroissantes).
///
/// > 0.70 = VCP authentique → +10 pts score  
/// > 0.90 = parfait → +10 pts supplémentaires (via double condition)
pub fn calc_contraction_qualite(
    highs: &[f64],
    lows: &[f64],
    nb_compression: usize,
) -> f64 {
    let swings = calc_swing_amplitudes(highs, lows, nb_compression);
    if swings.len() < 2 {
        return 0.0;
    }
    let progressif = swings
        .windows(2)
        .filter(|w| w[1] <= w[0] * 0.92) // chaque swing ≤ 92% du précédent
        .count();
    progressif as f64 / (swings.len() - 1) as f64
}

/// Retourne la série ordonnée des amplitudes de swing (high−low) sur la fenêtre de
/// compression. La suite décroissante est la signature d'un VCP authentique.
/// Utilisée pour enrichir le contexte LLM avec la structure temporelle réelle.
pub fn calc_swing_amplitudes(
    highs: &[f64],
    lows: &[f64],
    nb_compression: usize,
) -> Vec<f64> {
    if nb_compression < 2 {
        return vec![];
    }
    let n = highs.len();
    let start = n.saturating_sub(nb_compression + 1);
    (start..n.saturating_sub(1))
        .map(|i| highs[i] - lows[i])
        .collect()
}
