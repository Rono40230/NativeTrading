use common::{Asset, Timeframe};

/// Parse un nom d'asset case-insensitive. La légitimité d'un ticker vient
/// de la table `assets` — aucune liste codée : tout ticker formel est
/// accepté (majuscules, lettres/chiffres, 2-20 caractères).
pub fn parse_asset(s: &str) -> Option<Asset> {
    let t = s.trim().to_uppercase();
    if t.len() >= 2 && t.len() <= 20 && t.chars().all(|c| c.is_ascii_alphanumeric()) {
        Some(Asset::from(t))
    } else {
        None
    }
}

/// Normalise un score LLM vers l'échelle 0-10.
/// Si le LLM retourne 0-1 au lieu de 0-10, multiplie par 10.
/// Résultat clampé entre 0.0 et 10.0.
pub fn normaliser_score_llm(score: f64) -> f64 {
    let normalise = if score > 0.0 && score <= 1.0 {
        score * 10.0
    } else {
        score
    };
    normalise.clamp(0.0, 10.0)
}

#[cfg(test)]
mod tests {
    use super::normaliser_score_llm;

    #[test]
    fn score_sur_echelle_0_10_inchange() {
        assert_eq!(normaliser_score_llm(7.5), 7.5);
    }

    #[test]
    fn score_sur_echelle_0_1_normalise_vers_0_10() {
        assert_eq!(normaliser_score_llm(0.75), 7.5);
    }

    #[test]
    fn score_negatif_clamp_a_zero() {
        assert_eq!(normaliser_score_llm(-1.0), 0.0);
    }

    #[test]
    fn score_superieur_10_clamp_a_10() {
        assert_eq!(normaliser_score_llm(12.0), 10.0);
    }
}

/// Parse un timeframe avec M15 comme valeur par défaut.
pub fn parse_timeframe(s: &str) -> Timeframe {
    match s {
        "M1" => Timeframe::M1,
        "M5" => Timeframe::M5,
        "M10" => Timeframe::M10,
        "M15" => Timeframe::M15,
        "M30" => Timeframe::M30,
        "H1" => Timeframe::H1,
        "H4" => Timeframe::H4,
        "D1" => Timeframe::D1,
        "W1" => Timeframe::W1,
        _ => Timeframe::M15,
    }
}
