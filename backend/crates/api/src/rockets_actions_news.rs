//! Catalyseur news actions US (étape D, 01/09) — collecte Yahoo Finance RSS.
//!
//! Pour chaque candidat action ACTIF (≤ 20), on tire le flux RSS dédié au
//! ticker (gratuit, sans clé) et on range les dépêches dans le pipeline
//! presse existant : `presse_articles` avec `assets_concernes=["TICKER"]`
//! (l'analyste news les retrouve via ce marquage — les titres US parlent
//! de « Apple », pas de « AAPL »). Déduplication par hash de titre.
//! L'analyste `rockets_catalyseur` fait le reste : verdict + point news,
//! et date de résultats (earnings) si une dépêche la mentionne.

use actix_web::{web, HttpResponse, Responder};
use sqlx::Row;
use std::sync::Arc;

use crate::state::AppState;
use db::Database;

/// Convertit une date RSS RFC 2822 (« Mon, 01 Sep 2026 14:30:00 +0000 »)
/// au format TEXT de presse_articles (« 2026-09-01 14:30:00 »). Pur (testé).
pub fn convertir_date_rss(pubdate: &str) -> String {
    chrono::DateTime::parse_from_rfc2822(pubdate)
        .map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_else(|_| chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string())
}

/// Valide une date ISO renvoyée par l'analyste (« 2026-09-15 »), sinon None.
pub fn valider_date_earnings(d: &str) -> Option<String> {
    let d = d.trim();
    if d.len() == 10 && chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d").is_ok() {
        Some(d.to_string())
    } else {
        None
    }
}

/// Collecte les dépêches Yahoo RSS des candidats actions actifs.
/// Retour (tickers visités, dépêches insérées).
pub async fn collecter(db: &Arc<Database>) -> (usize, usize) {
    let Ok(candidats) = sqlx::query(
        "SELECT DISTINCT symbole FROM rockets_candidats
         WHERE univers = 'action' AND elimine_le IS NULL
         ORDER BY points DESC LIMIT 20",
    )
    .fetch_all(db.pool())
    .await
    else {
        return (0, 0);
    };
    let tickers: Vec<String> = candidats.iter().map(|r| r.get::<String, _>("symbole")).collect();

    let client = &*crate::http_client::HTTP_CLIENT;
    let mut inserees = 0usize;
    for t in &tickers {
        let url = format!(
            "https://feeds.finance.yahoo.com/rss/2.0/headline?s={t}&region=US&lang=en-US"
        );
        // Yahoo rejette les clients sans User-Agent navigateur (404 nu).
        let Ok(rep) = client
            .get(&url)
            .header("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/128.0 Safari/537.36")
            .send()
            .await
        else { continue };
        let Ok(xml) = rep.text().await else { continue };
        for item in news::news_rss::extraire_items_rss(&xml) {
            let hash = news::news_traduction::hash_titre(&item.titre);
            let insere = sqlx::query(
                "INSERT OR IGNORE INTO presse_articles
                 (hash_titre, titre, url, source_nom, publie_le, score, theme,
                  assets_concernes, impact, statut_traduction, resume_source, ajoute_le)
                 VALUES (?, ?, ?, 'Yahoo Finance', ?, 0, 'actions_us', ?, 'faible',
                         'non_tente', ?, strftime('%s','now'))",
            )
            .bind(&hash)
            .bind(&item.titre)
            .bind(&item.lien)
            .bind(convertir_date_rss(&item.date_rss))
            .bind(format!("[\"{t}\"]"))
            .bind(&item.resume)
            .execute(db.pool())
            .await
            .map(|r| r.rows_affected())
            .unwrap_or(0);
            inserees += insere as usize;
        }
        // Respiration entre tickers.
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    }
    if inserees > 0 {
        tracing::info!("🚀 News actions : {inserees} dépêches Yahoo rangées ({} tickers)", tickers.len());
    }
    (tickers.len(), inserees)
}

/// POST /api/rockets/actions/news/collecter — collecte manuelle.
pub async fn post_collecter(state: web::Data<AppState>) -> impl Responder {
    let (tickers, inserees) = collecter(&state.db).await;
    HttpResponse::Ok().json(serde_json::json!({ "tickers": tickers, "depeches": inserees }))
}

#[cfg(test)]
mod tests {
    use super::{convertir_date_rss, valider_date_earnings};

    #[test]
    fn date_rfc2822_convertie() {
        assert_eq!(
            convertir_date_rss("Tue, 01 Sep 2026 14:30:00 +0000"),
            "2026-09-01 14:30:00"
        );
    }

    #[test]
    fn date_invalide_tombe_sur_maintenant() {
        let now = chrono::Utc::now().format("%Y-%m-%d").to_string();
        assert!(convertir_date_rss("n'importe quoi").starts_with(&now));
    }

    #[test]
    fn date_earnings_validee() {
        assert_eq!(valider_date_earnings("2026-09-15"), Some("2026-09-15".into()));
        assert_eq!(valider_date_earnings(" 2026-09-15 "), Some("2026-09-15".into()));
        assert_eq!(valider_date_earnings("15/09/2026"), None);
        assert_eq!(valider_date_earnings(""), None);
        assert_eq!(valider_date_earnings("bientôt"), None);
    }
}
