use reqwest::Client;

pub struct ArticleRss {
    pub titre: String,
    pub lien: String,
    pub date_rss: String,
    /// Résumé brut du flux (`<description>` ou `<content:encoded>`).
    /// Socle d'affichage de la modal : les sites rendus en JavaScript ne
    /// donnent rien au scraper, mais le RSS a TOUJOURS une description.
    pub resume: String,
}

/// Extrait le texte d'une balise XML, avec support CDATA.
fn extraire_balise(xml: &str, tag: &str) -> Option<String> {
    // CDATA en priorité : <tag><![CDATA[...]]>
    let cdata_open = format!("<{tag}><![CDATA[");
    if let Some(s) = xml.find(&cdata_open) {
        let debut = s + cdata_open.len();
        if let Some(end) = xml[debut..].find("]]>") {
            return Some(xml[debut..debut + end].trim().to_string());
        }
    }

    // Balise standard : <tag>...</tag>
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = xml.find(&open)? + open.len();
    let end = xml[start..].find(&close)?;
    Some(xml[start..start + end].trim().to_string())
}

/// Découpe le XML RSS en items et extrait titre + lien + date.
pub fn extraire_items_rss(xml: &str) -> Vec<ArticleRss> {
    xml.split("<item>")
        .skip(1)
        .filter_map(|bloc| {
            let fin = bloc.find("</item>")?;
            let item = &bloc[..fin];

            let titre = extraire_balise(item, "title")?;
            // Certains flux utilisent <guid> comme lien
            let lien = extraire_balise(item, "link")
                .or_else(|| extraire_balise(item, "guid"))
                .unwrap_or_default();
            let date = extraire_balise(item, "pubDate")
                .or_else(|| extraire_balise(item, "dc:date"))
                .unwrap_or_default();
            // Résumé : `<description>` (quasi universel, 100-200 chars) puis
            // `<content:encoded>` (flux riches) — sinon vide (dégradation
            // assumée, la modal retombera sur le scraper).
            let resume = extraire_balise(item, "description")
                .or_else(|| extraire_balise(item, "content:encoded"))
                .unwrap_or_default();

            Some(ArticleRss {
                titre,
                lien,
                date_rss: date,
                resume,
            })
        })
        .collect()
}

/// Télécharge un flux RSS et retourne ses items. Dégradation silencieuse en cas d'erreur.
pub async fn fetch_rss(client: &Client, url: &str) -> Vec<ArticleRss> {
    let res = client
        .get(url)
        .header("Accept", "application/rss+xml, application/xml, text/xml")
        .header(
            reqwest::header::USER_AGENT,
            "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36",
        )
        .send()
        .await;

    match res {
        Ok(r) if r.status().is_success() => match r.text().await {
            Ok(xml) => extraire_items_rss(&xml),
            Err(e) => {
                tracing::debug!("Lecture RSS {url}: {e}");
                vec![]
            }
        },
        Ok(r) => {
            tracing::debug!("RSS {url} → HTTP {}", r.status());
            vec![]
        }
        Err(e) => {
            tracing::debug!("Fetch RSS {url}: {e}");
            vec![]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extraire_items_recupere_le_resume() {
        let xml = r#"<rss><channel>
            <item><title>A</title><link>https://x.fr/a</link><pubDate>Sat, 15 Aug 2026 10:00:00 GMT</pubDate>
            <description><![CDATA[Résumé de l'article A.]]></description></item>
            <item><title>B</title><link>https://x.fr/b</link>
            <description>Résumé B standard</description></item>
            <item><title>C</title><link>https://x.fr/c</link>
            <content:encoded><![CDATA[<p>Contenu riche C</p>]]></content:encoded></item>
            <item><title>D</title><link>https://x.fr/d</link></item>
        </channel></rss>"#;
        let items = extraire_items_rss(xml);
        assert_eq!(items.len(), 4);
        assert_eq!(items[0].resume, "Résumé de l'article A.");
        assert_eq!(items[1].resume, "Résumé B standard");
        // Pas de <description> → fallback <content:encoded>
        assert_eq!(items[2].resume, "<p>Contenu riche C</p>");
        // Ni l'un ni l'autre → vide
        assert_eq!(items[3].resume, "");
    }
}
