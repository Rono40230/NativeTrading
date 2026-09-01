//! Étape 6 — les rôles IA de la verticale Rockets (analyste qwen3:32b) :
//!   1. « catalyseur news » — évalue les dépêches d'un candidat (point 1/10) ;
//!   2. « ranker de faux pivots » — seconde opinion avant chaque signal.
//! Extrait de rockets_verticale (limite 600 lignes).

use sqlx::Row;

use crate::rockets_verticale::klines_d1;

// ── Rôle « catalyseur news » : l'analyste lit la presse du candidat ────────

/// Une dépêche récente pour le contexte de l'analyste.
pub(crate) struct Depesche {
    titre: String,
    resume: String,
    date: String,
}

/// Récupère les dépêches des 15 derniers jours mentionnant la base du token
/// (marquage assets_concernes ou texte dans titre/résumé), max 6.
pub(crate) async fn depeches_pour(db: &db::Database, symbole: &str) -> Vec<Depesche> {
    let base = symbole.trim_end_matches("USDT").to_lowercase();
    if base.is_empty() {
        return Vec::new();
    }
    let Ok(rows) = sqlx::query(
        "SELECT titre, COALESCE(NULLIF(resume_fr, ''), titre) AS resume, publie_le
         FROM presse_articles
         WHERE publie_le >= datetime('now', '-15 days')
         ORDER BY publie_le DESC LIMIT 400",
    )
    .fetch_all(db.pool())
    .await
    else {
        return Vec::new();
    };
    rows.iter()
        .filter_map(|r| {
            let titre: String = r.get("titre");
            let resume: String = r.get("resume");
            let date: String = r.get("publie_le");
            let concerne = titre.to_lowercase().contains(&base)
                || resume.to_lowercase().contains(&base);
            concerne.then(|| Depesche { titre, resume, date })
        })
        .take(6)
        .collect()
}

/// Réponse structurée de l'analyste.
struct VerdictNews {
    verdict: String,
    conviction: i64,
    justification: String,
    /// Date de résultats (actions US) si une dépêche la mentionne — validée.
    earnings_le: Option<String>,
}

/// Extrait le premier objet JSON valide de la réponse (qwen3 peut
/// l'entourer de texte ou de balises de réflexion).
pub(crate) fn extraire_json(reponse: &str) -> Option<serde_json::Value> {
    let debut = reponse.find('{')?;
    let fin = reponse.rfind('}')?;
    serde_json::from_str::<serde_json::Value>(&reponse[debut..=fin]).ok()
}

/// Interroge l'analyste pour UN candidat.
async fn evaluer_un(db: &db::Database, symbole: &str, points_base: u8) -> Option<VerdictNews> {
    let depeches = depeches_pour(db, symbole).await;
    let texte_depeches = if depeches.is_empty() {
        "(aucune dépêche spécifique trouvée dans la revue de presse)".to_string()
    } else {
        depeches
            .iter()
            .map(|d| format!("- [{}] {} — {}", d.date, d.titre, d.resume))
            .collect::<Vec<_>>()
            .join("\n")
    };
    // Prompt = override de la page Prompts IA, sinon défaut canonique.
    let reglages = llm::charger_overrides();
    let defauts = llm::defaults();
    let role: String = reglages
        .get("rockets_catalyseur")
        .cloned()
        .or_else(|| defauts.get("rockets_catalyseur").map(|d| d.to_string()))
        .unwrap_or_default();
    let prompt = format!(
        "{}\n\nCANDIDAT : {} (classement chiffrable : {}/9)\nDÉPÊCHES DES 15 DERNIERS JOURS :\n{}\n\nÉvalue le catalyseur news et réponds en JSON.",
        role, symbole, points_base, texte_depeches,
    );
    let reponse = llm::ollama::interroger_avec_modele_smc(&prompt).await.ok()?;
    let json = extraire_json(&reponse)?;
    Some(VerdictNews {
        verdict: json.get("verdict").and_then(|v| v.as_str()).unwrap_or("NEUTRE").to_uppercase(),
        conviction: json.get("conviction").and_then(|v| v.as_i64()).unwrap_or(0).clamp(0, 100),
        justification: json
            .get("justification")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        earnings_le: json
            .get("earnings_date")
            .and_then(|v| v.as_str())
            .and_then(crate::rockets_actions_news::valider_date_earnings),
    })
}

/// Passe l'analyste sur tous les candidats du jour (max 8) et met à jour
/// le point News + le classement total. Dégradation douce : Ollama absent
/// → news « non évalué », le classement reste sur les chiffrables.
pub(crate) async fn evaluer_news(db: &std::sync::Arc<db::Database>) {
    let Ok(candidats) = sqlx::query(
        "SELECT symbole, points_base FROM rockets_candidats WHERE elimine_le IS NULL ORDER BY points_base DESC LIMIT 8",
    )
    .fetch_all(db.pool())
    .await
    else {
        return;
    };
    for c in &candidats {
        let symbole: String = c.get("symbole");
        let points_base: Option<i64> = c.try_get("points_base").ok().flatten();
        let points_base = points_base.unwrap_or(0).clamp(0, 9) as u8;
        match evaluer_un(db, &symbole, points_base).await {
            Some(v) => {
                let news_points: i64 = if v.verdict == "POUR" && v.conviction >= 60 { 1 } else { 0 };
                let total = (points_base as i64 + news_points).min(10);
                let verdict = if total >= 9 {
                    "Alpha"
                } else if total >= 7 {
                    "Rocket"
                } else {
                    "Elimine"
                };
                let _ = sqlx::query(
                    "UPDATE rockets_candidats
                     SET news_verdict = ?, news_conviction = ?, news_justification = ?,
                         news_points = ?, points = ?, verdict = ?,
                         earnings_le = COALESCE(?, earnings_le)
                     WHERE symbole = ?",
                )
                .bind(&v.verdict)
                .bind(v.conviction)
                .bind(&v.justification)
                .bind(news_points)
                .bind(total)
                .bind(verdict)
                .bind(&v.earnings_le)
                .bind(&symbole)
                .execute(db.pool())
                .await;
                tracing::info!(
                    "🚀 Rockets news {} : {} ({}/100) — point {}",
                    symbole, v.verdict, v.conviction, news_points
                );
            }
            None => {
                let _ = sqlx::query(
                    "UPDATE rockets_candidats SET news_verdict = 'non évalué', news_points = 0 WHERE symbole = ?",
                )
                .bind(&symbole)
                .execute(db.pool())
                .await;
            }
        }
    }
}

// ── Rôle « ranker de faux pivots » : seconde opinion avant le signal ──────

/// L'analyste relit le dossier d'une cassure et rend sa conviction (0-100).
/// Retourne (conviction, raison) — None si l'analyste est indisponible.
pub(crate) async fn ranker_cassure(db: &std::sync::Arc<db::Database>, symbole: &str, points_total: u8) -> Option<(i64, String)> {
    let detail: String = sqlx::query_scalar::<_, String>(
        "SELECT detail FROM rockets_candidats WHERE symbole = ?",
    )
    .bind(symbole)
    .fetch_one(db.pool())
    .await
    .ok()?;
    let news: Option<String> = sqlx::query_scalar::<_, Option<String>>(
        "SELECT news_verdict || ' ' || COALESCE(news_conviction, 0) || '/100' FROM rockets_candidats WHERE symbole = ?",
    )
    .bind(symbole)
    .fetch_one(db.pool())
    .await
    .ok()
    .flatten();
    // Les 12 dernières bougies D1 confirmées, en compact.
    let bougies = klines_d1(symbole, 13).await;
    let dernieres: Vec<String> = bougies
        .iter()
        .take(12)
        .map(|b| {
            format!(
                "{}: O{:.4} H{:.4} L{:.4} C{:.4} V{:.0}",
                chrono::DateTime::from_timestamp(b.ts, 0)
                    .map(|d| d.format("%m-%d").to_string())
                    .unwrap_or_default(),
                b.open, b.high, b.low, b.close, b.volume
            )
        })
        .collect();
    let reglages = llm::charger_overrides();
    let defauts = llm::defaults();
    let role: String = reglages
        .get("rockets_ranker")
        .cloned()
        .or_else(|| defauts.get("rockets_ranker").map(|d| d.to_string()))
        .unwrap_or_default();
    let prompt = format!(
        "{}\n\nCANDIDAT À LA CASSURE : {} — classement {}/10.\nCritères : {}\nAvis news : {}\n12 dernières bougies D1 :\n{}\n\nCassure réelle ou fausse ? Réponds en JSON.",
        role,
        symbole,
        points_total,
        detail,
        news.unwrap_or_else(|| "non évalué".into()),
        dernieres.join("\n"),
    );
    let reponse = llm::ollama::interroger_avec_modele_smc(&prompt).await.ok()?;
    let json = extraire_json(&reponse)?;
    Some((
        json.get("conviction").and_then(|v| v.as_i64()).unwrap_or(0).clamp(0, 100),
        json.get("raison").and_then(|v| v.as_str()).unwrap_or("").to_string(),
    ))
}
