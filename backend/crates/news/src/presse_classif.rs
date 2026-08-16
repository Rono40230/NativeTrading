//! Classification presse v1 : attribution d'assets et d'impact par MOTS-CLÉS
//! (gratuit, immédiat — l'affinage LLM reste une évolution post-v1, cf spec).

use crate::news_rss::ArticleRss;
use crate::news_scoring::{classer_theme, jaccard_bigrammes, scorer};
use crate::news_traduction::hash_titre;

/// Mot-clé → asset du pipeline (12 assets). Ordre : spécifique avant générique.
const MOTS_ASSETS: &[(&str, &str)] = &[
    ("bitcoin", "BTC"), ("btc", "BTC"),
    ("ethereum", "ETH"), ("ether", "ETH"),
    ("gold", "XAUUSD"), ("xau", "XAUUSD"),
    ("silver", "XAGUSD"), ("xag", "XAGUSD"),
    ("euro/dollar", "EURUSD"), ("eur/usd", "EURUSD"), ("euro dollar", "EURUSD"),
    ("yen", "USDJPY"), ("japanese yen", "USDJPY"),
    ("dax", "DAX"), ("german index", "DAX"),
    ("nasdaq", "NAS100"), ("nas 100", "NAS100"),
    ("s&p", "SP500"), ("s&p 500", "SP500"),
];

/// Assets du pipeline cités dans le titre (dédoublonnés, ordre stable).
pub fn assets_concernes(titre_lower: &str) -> Vec<&'static str> {
    let mut trouves: Vec<&'static str> = Vec::new();
    for (mot, asset) in MOTS_ASSETS {
        if titre_lower.contains(mot) && !trouves.contains(asset) {
            trouves.push(asset);
        }
    }
    trouves
}

/// Impact dérivé du score (mots-clés × poids source).
pub fn impact(score: u8) -> &'static str {
    if score >= 60 { "fort" } else if score >= 35 { "moyen" } else { "faible" }
}

/// Article prêt pour l'insertion DB (cf db::presse::PresseArticle).
pub struct ArticleCollecte {
    pub hash_titre: String,
    pub titre: String,
    pub url: String,
    pub source_nom: String,
    pub publie_le: String,
    pub score: u8,
    pub theme: String,
    pub assets_concernes: String,
    pub impact: String,
}

/// Traite les items d'UN flux : dédoublonnage interne (jaccard ≥ 0.8),
/// scoring, thème, assets, impact. Fonction pure — aucune I/O.
pub fn traiter_items(items: &[ArticleRss], source_nom: &str, poids_source: u8) -> Vec<ArticleCollecte> {
    let mut retenus: Vec<ArticleCollecte> = Vec::new();
    for item in items {
        let titre_lower = item.titre.to_lowercase();
        // Doublon interne au flux ?
        if retenus.iter().any(|r| jaccard_bigrammes(&r.titre, &item.titre) >= 0.8) {
            continue;
        }
        let score = scorer(&titre_lower, poids_source, &item.date_rss);
        let assets = assets_concernes(&titre_lower);
        retenus.push(ArticleCollecte {
            hash_titre: hash_titre(&item.titre),
            titre: item.titre.clone(),
            url: item.lien.clone(),
            source_nom: source_nom.to_string(),
            publie_le: item.date_rss.clone(),
            score,
            theme: classer_theme(&titre_lower, source_nom).to_string(),
            assets_concernes: serde_json::to_string(&assets).unwrap_or_else(|_| "[]".into()),
            impact: impact(score).to_string(),
        });
    }
    retenus
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::news_rss::ArticleRss;

    fn rss(titre: &str) -> ArticleRss {
        ArticleRss { titre: titre.into(), lien: "https://x.fr".into(), date_rss: "Sat, 15 Aug 2026 10:00:00 GMT".into() }
    }

    #[test]
    fn assets_par_mots_cles() {
        let a = assets_concernes("bitcoin surges as fed cuts rates, gold slips");
        assert!(a.contains(&"BTC"));
        assert!(a.contains(&"XAUUSD"));
        // "fed"/"rates" → macro sans asset dédié : pas de faux positif
        assert!(!a.contains(&"DAX"));
        let b = assets_concernes("dax rallies with nasdaq");
        assert!(b.contains(&"DAX"));
        assert!(b.contains(&"NAS100"));
    }

    #[test]
    fn impact_par_score() {
        assert_eq!(impact(75), "fort");
        assert_eq!(impact(40), "moyen");
        assert_eq!(impact(10), "faible");
    }

    #[test]
    fn traiter_items_deduplique_et_classe() {
        let items = vec![
            rss("Fed cuts rates: bitcoin surges, gold slips"),
            rss("Fed cuts rates: bitcoin surges, gold slips"), // doublon exact
            rss("Unrelated earnings report from acme corp"),
        ];
        let res = traiter_items(&items, "Reuters Business", 40);
        // Le doublon jaccard ~1.0 est éliminé
        assert_eq!(res.len(), 2);
        let premier = res.iter().find(|a| a.titre.contains("Fed")).unwrap();
        assert_eq!(premier.source_nom, "Reuters Business");
        assert!(premier.score > 0);
        assert!(!premier.hash_titre.is_empty());
        assert!(premier.assets_concernes.contains("BTC"));
    }
}
