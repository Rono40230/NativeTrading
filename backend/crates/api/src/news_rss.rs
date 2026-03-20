use reqwest::Client;

pub struct ArticleRss {
    pub titre: String,
    pub lien: String,
    pub date_rss: String,
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

            Some(ArticleRss { titre, lien, date_rss: date })
        })
        .collect()
}

/// Télécharge un flux RSS et retourne ses items. Dégradation silencieuse en cas d'erreur.
pub async fn fetch_rss(client: &Client, url: &str) -> Vec<ArticleRss> {
    let res = client
        .get(url)
        .header("Accept", "application/rss+xml, application/xml, text/xml")
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
