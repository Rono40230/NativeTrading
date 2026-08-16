use reqwest::Client;

/// SSRF protection : HTTPS uniquement, adresses IP internes bloquées (RFC 1918).
pub fn est_url_externe_sure(url: &str) -> bool {
    if !url.starts_with("https://") {
        return false;
    }
    let apres_scheme = url.trim_start_matches("https://");
    let host = apres_scheme
        .split('/')
        .next()
        .unwrap_or("")
        .split(':')
        .next()
        .unwrap_or("");

    if host.is_empty() || host == "localhost" || host.ends_with(".local") {
        return false;
    }
    for prefix in &["127.", "10.", "192.168.", "169.254."] {
        if host.starts_with(prefix) {
            return false;
        }
    }
    // 172.16.0.0/12
    if let Some(rest) = host.strip_prefix("172.") {
        if let Some(n) = rest.split('.').next().and_then(|s| s.parse::<u8>().ok()) {
            if (16..=31).contains(&n) {
                return false;
            }
        }
    }
    true
}

// ── Nettoyage HTML ───────────────────────────────────────────────────────────

fn supprimer_balises(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let mut in_tag = false;
    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                result.push(' ');
            }
            _ if !in_tag => result.push(ch),
            _ => {}
        }
    }
    result
}

fn decoder_entites(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&#x27;", "'")
        .replace("&#x2019;", "'")
        .replace("&#x2018;", "'")
        .replace("&#x201C;", "\"")
        .replace("&#x201D;", "\"")
        .replace("&nbsp;", " ")
        .replace("&#39;", "'")
}

fn nettoyer(html: &str) -> String {
    let sans_balises = supprimer_balises(html);
    let sans_entites = decoder_entites(&sans_balises);
    let mut res = String::new();
    let mut prev_space = true;
    for ch in sans_entites.chars() {
        if ch.is_whitespace() {
            if !prev_space {
                res.push(' ');
                prev_space = true;
            }
        } else {
            res.push(ch);
            prev_space = false;
        }
    }
    res.trim().to_string()
}

// ── Extraction paragraphes ───────────────────────────────────────────────────

/// Extrait jusqu'à 8 paragraphes <p> significatifs du HTML.
fn extraire_paragraphes(html: &str) -> String {
    let mut paragraphes: Vec<String> = Vec::new();
    let mut pos = 0;

    while pos < html.len() && paragraphes.len() < 8 {
        let Some(rel) = html[pos..].find("<p") else {
            break;
        };
        let p_start = pos + rel;

        // Exclure <path>, <pre>, <progress>, etc.
        let suivant = html.get(p_start + 2..p_start + 3).unwrap_or("x");
        if !matches!(suivant, " " | ">" | "\t" | "\n" | "\r") {
            pos = p_start + 3;
            continue;
        }

        let Some(rel2) = html[p_start..].find('>') else {
            break;
        };
        let content_start = p_start + rel2 + 1;

        let Some(rel3) = html[content_start..].find("</p>") else {
            break;
        };

        let para = nettoyer(&html[content_start..content_start + rel3]);
        if para.chars().count() > 40 {
            paragraphes.push(para);
        }
        pos = content_start + rel3 + 4;
    }

    paragraphes.join("\n\n")
}

/// Tente d'isoler la zone de contenu principal (<article>, <main>, fallback body).
fn trouver_zone<'a>(html: &'a str, balise: &str) -> Option<&'a str> {
    let open = format!("<{}", balise);
    let close = format!("</{}>", balise);
    let start = html.find(&open)?;
    let end_rel = html[start..].find(&close)?;
    Some(&html[start..start + end_rel])
}

// ── Entrée publique ──────────────────────────────────────────────────────────

/// Télécharge une page et retourne son texte lisible.
/// Retourne `None` si la page est inaccessible, sans paragraphes extractibles,
/// ou si c'est un mur de consentement cookies/privauté (pas un article).
pub async fn recuperer_contenu_article(client: &Client, url: &str) -> Option<String> {
    let html = client
        .get(url)
        .header("Accept", "text/html,application/xhtml+xml")
        .header("Accept-Language", "fr-FR,fr;q=0.9,en;q=0.8")
        .header(
            reqwest::header::USER_AGENT,
            "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36",
        )
        .send()
        .await
        .ok()?
        .text()
        .await
        .ok()?;

    let zone = trouver_zone(&html, "article")
        .or_else(|| trouver_zone(&html, "main"))
        .unwrap_or(&html);

    let texte = extraire_paragraphes(zone);
    if texte.len() < 100 {
        return None;
    }
    // Mur de consentement cookies/privauté : les sites modernes (Yahoo,
    // Reuters…) servent ces pages aux requêtes sans JavaScript. Le texte
    // ressemble à une politique de confidentialité, pas à un article —
    // on le rejette plutôt que de le servir comme contenu.
    if est_mur_consentement(&texte) {
        return None;
    }
    Some(texte)
}

/// Détecte une page de consentement cookies/privauté (fr ou en) : une
/// densité anormale de termes juridiques sur un court texte. Un vrai
/// article peut mentionner les cookies une fois — pas quinze fois.
fn est_mur_consentement(texte: &str) -> bool {
    let t = texte.to_lowercase();
    let marqueurs = [
        "cookies", "cookie", "consentement", "consent", "privauté", "privacy",
        "confidentialité", "données personnelles", "personal data",
        "paramètres de confidentialité", "privacy settings",
        "refuser tout", "reject all", "gérer les paramètres",
    ];
    let occurrences: usize = marqueurs.iter()
        .map(|m| t.matches(m).count())
        .sum();
    // Un article de presse ne contient pas 8+ occurrences de termes de
    // consentement dans les ~2000 premiers caractères.
    occurrences >= 8
}

#[cfg(test)]
mod tests_consentement {
    use super::est_mur_consentement;

    #[test]
    fn page_cookies_yahoo_detectee() {
        let texte = "Votre vie privée est importante pour nous. Chez Yahoo, nous utilisons des cookies. Les cookies permettent de stocker et lire des informations. Consultez notre politique relative aux cookies. Si vous ne souhaitez pas que nos partenaires utilisent des cookies et vos données personnelles, cliquez sur Refuser tout. Vous pouvez révoquer votre consentement ou modifier vos choix à tout moment en cliquant sur Paramètres de confidentialité et de cookies. Découvrez comment nous utilisons vos données personnelles dans notre Politique de confidentialité.";
        assert!(est_mur_consentement(texte), "doit détecter le mur de consentement Yahoo");
    }

    #[test]
    fn vrai_article_non_detecte() {
        let texte = r#"Bitcoin surged past 60000 on Friday as institutional investors continued to accumulate. The rally comes amid growing expectations of Fed rate cuts. Analysts at Goldman Sachs noted that ETF inflows have accelerated, with BlackRock IBIT reaching record volumes. Meanwhile, ether also gained, trading above 3400."#;
        assert!(!est_mur_consentement(texte), "un vrai article ne doit pas être filtré");
    }
}
