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

/// Normalise une date RSS en RFC 3339 : les `<pubDate>` RSS arrivent en RFC 2822
/// (« Sat, 15 Aug 2026 10:00:00 GMT ») alors que `scorer`/`bonus_temporel` et le
/// stockage DB attendent du RFC 3339. Si la chaîne est déjà RFC 3339 (ex. flux
/// `<dc:date>` ISO), elle est retournée telle quelle.
///
/// Dégradation assumée : format inconnu → chaîne originale retournée (le scorer
/// appliquera sa pénalité de date non parsable, la DB stockera la valeur brute —
/// on préfère une donnée fidèle à une date inventée).
fn normaliser_date_rss(date: &str) -> String {
    if let Ok(dt) = chrono::DateTime::parse_from_rfc2822(date) {
        return dt.to_rfc3339();
    }
    if chrono::DateTime::parse_from_rfc3339(date).is_ok() {
        return date.to_string();
    }
    date.to_string()
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
    /// Résumé du flux RSS nettoyé (sans HTML) — socle d'affichage de la
    /// modal quand le scraper échoue sur les sites rendus en JavaScript.
    pub resume: String,
}

/// Nettoie le résumé RSS brut : les `<description>` arrivent souvent en HTML
/// fragmentaire (`<a href=…>`, `&amp;`, `&#39;`…). Strip des balises par
/// parcours de caractères (équivalent d'un regex `<[^>]*>`), décodage des
/// entités courantes, espaces blancs réduits à un seul. Sans dépendance.
fn nettoyer_resume(brut: &str) -> String {
    let sans_balises: String = {
        let mut hors_balise = true;
        brut.chars()
            .filter(|&c| {
                if c == '<' {
                    hors_balise = false;
                } else if c == '>' {
                    hors_balise = true;
                    return false; // le '>' lui-même est avalé
                }
                hors_balise
            })
            .collect()
    };
    let decodee = sans_balises
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&nbsp;", " ");
    decodee.split_whitespace().collect::<Vec<_>>().join(" ")
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
        let date_iso = normaliser_date_rss(&item.date_rss);
        let score = scorer(&titre_lower, poids_source, &date_iso);
        let assets = assets_concernes(&titre_lower);
        retenus.push(ArticleCollecte {
            hash_titre: hash_titre(&item.titre),
            titre: item.titre.clone(),
            url: item.lien.clone(),
            source_nom: source_nom.to_string(),
            publie_le: date_iso,
            score,
            theme: classer_theme(&titre_lower, source_nom).to_string(),
            assets_concernes: serde_json::to_string(&assets).unwrap_or_else(|_| "[]".into()),
            impact: impact(score).to_string(),
            resume: nettoyer_resume(&item.resume),
        });
    }
    retenus
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::news_rss::ArticleRss;

    fn rss(titre: &str) -> ArticleRss {
        ArticleRss { titre: titre.into(), lien: "https://x.fr".into(), date_rss: "Sat, 15 Aug 2026 10:00:00 GMT".into(), resume: String::new() }
    }

    fn rss_dated(titre: &str, date_rss: &str) -> ArticleRss {
        ArticleRss { titre: titre.into(), lien: "https://x.fr".into(), date_rss: date_rss.into(), resume: String::new() }
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

    #[test]
    fn normaliser_date_rss_convertit_2822_vers_3339() {
        // RFC 2822 → RFC 3339, date conservée, résultat reparsable
        let iso = normaliser_date_rss("Sat, 15 Aug 2026 10:00:00 GMT");
        assert!(iso.contains("2026-08-15"), "attendu 2026-08-15 dans : {iso}");
        assert!(chrono::DateTime::parse_from_rfc3339(&iso).is_ok());
        // Déjà RFC 3339 → inchangé
        assert_eq!(normaliser_date_rss("2026-08-15T10:00:00Z"), "2026-08-15T10:00:00Z");
        // Format inconnu → original (dégradation documentée)
        assert_eq!(normaliser_date_rss("pas une date"), "pas une date");
    }

    #[test]
    fn traiter_items_bonus_fraicheur_sur_date_rss_2822() {
        // Avant correctif : date RFC 2822 brute passée au scorer (qui parse du RFC 3339)
        // → échec silencieux → pénalité -10 pour TOUT article en pubDate, bonus jamais appliqué.
        let date_fraiche = chrono::Utc::now().format("%a, %d %b %Y %H:%M:%S GMT").to_string();
        let date_ancienne =
            (chrono::Utc::now() - chrono::Duration::days(7)).format("%a, %d %b %Y %H:%M:%S GMT").to_string();

        let titre = "Unrelated earnings report from acme corp";
        let res_frais = traiter_items(&[rss_dated(titre, &date_fraiche)], "Reuters Business", 10);
        let res_ancien = traiter_items(&[rss_dated(titre, &date_ancienne)], "Reuters Business", 10);

        // Frais (bonus +10) doit STRICTEMENT dépasser ancien > 6 jours (-10) :
        // égalité = pénalité de parse toujours présente.
        assert!(
            res_frais[0].score > res_ancien[0].score,
            "frais={} ancien={} : la pénalité de parse ne doit plus s'appliquer",
            res_frais[0].score,
            res_ancien[0].score
        );
        // Stockage ISO en DB : publie_le doit être parsable en RFC 3339.
        assert!(chrono::DateTime::parse_from_rfc3339(&res_ancien[0].publie_le).is_ok());
    }

    #[test]
    fn nettoyer_resume_strip_le_html() {
        // Balises (avec attributs), entités, espaces multiples/leading
        assert_eq!(nettoyer_resume("<p>BTC &amp; gold <b>rally</b></p>"), "BTC & gold rally");
        assert_eq!(nettoyer_resume("  <a href=\"https://x.fr\">Lien</a>\n&#39;quote&#39;  "), "Lien 'quote'");
        assert_eq!(nettoyer_resume("texte &nbsp;solid&lt;&gt;"), "texte solid<>");
        // Cas limites : balise non fermée, vide
        assert_eq!(nettoyer_resume("tronqué <b brav"), "tronqué");
        assert_eq!(nettoyer_resume(""), "");
    }

    #[test]
    fn traiter_items_propage_le_resume_nettoye() {
        let items = vec![ArticleRss {
            titre: "Bitcoin surges as fed cuts rates".into(),
            lien: "https://x.fr".into(),
            date_rss: "Sat, 15 Aug 2026 10:00:00 GMT".into(),
            resume: "  <p>Bitcoin &amp; co jump&nbsp;after the cut.</p>  ".into(),
        }];
        let res = traiter_items(&items, "Reuters Business", 40);
        assert_eq!(res[0].resume, "Bitcoin & co jump after the cut.");
        // Flux sans description → chaîne vide (dégradation, pas d'invention)
        let vide = traiter_items(&[rss("Unrelated earnings report")], "Reuters Business", 40);
        assert_eq!(vide[0].resume, "");
    }
}
