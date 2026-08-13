//! Post-filtre directionnel basé sur le sentiment composite (Phase 3).
//!
//! Ajuste la force/conviction d'un signal candidat selon l'alignement de sa
//! direction avec le sentiment de sa classe d'actif. Un signal à contre-sens
//! d'un sentiment extrême est marqué `Extreme` (à ignorer).

use smc::v12::sentiment::{classe_actif, Alignement, SentimentScore, SentimentVerdict};

/// Normalise une chaîne de direction en booléen `is_long`.
///
/// Accepte "Long"/"Haussier" (insensible à la casse) → Long, sinon Short.
fn est_long(direction: &str) -> bool {
    let d = direction.trim().to_lowercase();
    d == "long" || d == "haussier" || d == "buy" || d == "bull"
}

/// Score de sentiment pour la classe de l'asset, avec fallback global puis neutre.
fn score_pour_asset(sentiment: &SentimentScore, asset: &str) -> f64 {
    let classe = classe_actif(asset);
    let par_classe = match classe {
        "crypto" => sentiment.crypto,
        "forex" => sentiment.forex,
        "metaux" => sentiment.metaux,
        "indices" => sentiment.indices,
        _ => None,
    };
    par_classe.or(sentiment.global).unwrap_or(50.0)
}

/// Filtre un signal candidat par le sentiment de sa classe d'actif.
///
/// Règles (score 0-100 de la classe, ou global fallback) :
///   - Bullish (>60) + Long  → Aligne, +15%
///   - Bullish (>60) + Short → Oppose, -20%
///   - Bearish (<40) + Short → Aligne, +15%
///   - Bearish (<40) + Long  → Oppose, -20%
///   - Extreme (<20 ou >80) à contre-sens → Extreme (signal à ignorer)
///   - Neutre (40-60) → Neutre, 0%
///
/// L'extrême *aligné* suit la règle bullish/bearish standard (+15% Aligne) :
/// seul l'extrême à contre-sens déclenche `Extreme`.
pub fn filtrer_par_sentiment(
    direction: &str,
    asset: &str,
    sentiment: &SentimentScore,
) -> SentimentVerdict {
    let score_classe = score_pour_asset(sentiment, asset);
    let is_long = est_long(direction);

    let bullish = score_classe > 60.0;
    let bearish = score_classe < 40.0;
    let extreme_bull = score_classe > 80.0;
    let extreme_bear = score_classe < 20.0;

    // 1. Extrême à contre-sens → signal à ignorer.
    if (extreme_bull && !is_long) || (extreme_bear && is_long) {
        return SentimentVerdict {
            ajustement: -100.0,
            alignement: Alignement::Extreme,
            score_classe,
        };
    }

    // 2. Alignement standard selon bullish/bearish.
    if bullish && is_long {
        return SentimentVerdict {
            ajustement: 15.0,
            alignement: Alignement::Aligne,
            score_classe,
        };
    }
    if bullish && !is_long {
        return SentimentVerdict {
            ajustement: -20.0,
            alignement: Alignement::Oppose,
            score_classe,
        };
    }
    if bearish && !is_long {
        return SentimentVerdict {
            ajustement: 15.0,
            alignement: Alignement::Aligne,
            score_classe,
        };
    }
    if bearish && is_long {
        return SentimentVerdict {
            ajustement: -20.0,
            alignement: Alignement::Oppose,
            score_classe,
        };
    }

    // 3. Neutre (40-60).
    SentimentVerdict {
        ajustement: 0.0,
        alignement: Alignement::Neutre,
        score_classe,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sc(crypto: Option<f64>, global: Option<f64>) -> SentimentScore {
        SentimentScore {
            crypto,
            global,
            ..Default::default()
        }
    }

    #[test]
    fn bullish_long_aligne() {
        let v = filtrer_par_sentiment("Long", "BTC", &sc(Some(70.0), None));
        assert_eq!(v.alignement, Alignement::Aligne);
        assert_eq!(v.ajustement, 15.0);
    }

    #[test]
    fn bullish_short_oppose() {
        let v = filtrer_par_sentiment("Short", "BTC", &sc(Some(70.0), None));
        assert_eq!(v.alignement, Alignement::Oppose);
        assert_eq!(v.ajustement, -20.0);
    }

    #[test]
    fn bearish_short_aligne() {
        let v = filtrer_par_sentiment("Short", "EURUSD", &sc(None, Some(30.0)));
        assert_eq!(v.alignement, Alignement::Aligne);
        assert_eq!(v.ajustement, 15.0);
    }

    #[test]
    fn bearish_long_oppose() {
        let v = filtrer_par_sentiment("Long", "EURUSD", &sc(None, Some(30.0)));
        assert_eq!(v.alignement, Alignement::Oppose);
        assert_eq!(v.ajustement, -20.0);
    }

    #[test]
    fn extreme_bull_short_est_extreme() {
        let v = filtrer_par_sentiment("Short", "BTC", &sc(Some(85.0), None));
        assert_eq!(v.alignement, Alignement::Extreme);
    }

    #[test]
    fn extreme_bear_long_est_extreme() {
        let v = filtrer_par_sentiment("Long", "BTC", &sc(Some(15.0), None));
        assert_eq!(v.alignement, Alignement::Extreme);
    }

    #[test]
    fn extreme_bull_long_aligne_pas_extreme() {
        // Extrême mais aligné → règle bullish standard (Aligne +15).
        let v = filtrer_par_sentiment("Long", "BTC", &sc(Some(85.0), None));
        assert_eq!(v.alignement, Alignement::Aligne);
        assert_eq!(v.ajustement, 15.0);
    }

    #[test]
    fn neutre_zero() {
        let v = filtrer_par_sentiment("Long", "BTC", &sc(Some(50.0), None));
        assert_eq!(v.alignement, Alignement::Neutre);
        assert_eq!(v.ajustement, 0.0);
    }

    #[test]
    fn direction_francaise_haussier_long() {
        let v = filtrer_par_sentiment("Haussier", "BTC", &sc(Some(70.0), None));
        assert_eq!(v.alignement, Alignement::Aligne);
    }

    #[test]
    fn classe_metaux_utilise_score_metaux() {
        let mut s = SentimentScore::default();
        s.metaux = Some(75.0);
        s.crypto = Some(10.0); // ne doit pas être utilisé pour XAUUSD
        let v = filtrer_par_sentiment("Long", "XAUUSD", &s);
        assert_eq!(v.alignement, Alignement::Aligne);
        assert!((v.score_classe - 75.0).abs() < 1e-9);
    }

    #[test]
    fn fallback_global_puis_neutre() {
        // Pas de score classe ni global → 50 neutre.
        let v = filtrer_par_sentiment("Long", "BTC", &sc(None, None));
        assert_eq!(v.alignement, Alignement::Neutre);
        assert!((v.score_classe - 50.0).abs() < 1e-9);
    }
}
