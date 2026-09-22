//! Notation LLM des articles laissés à 0 par le scorer heuristique (tâche 6.4
//! de l'audit). Le scorer mots-clés ne voit pas les titres sans terme connu —
//! souvent pertinents (« Euro remains cautious as… »). Ce job de fond fait
//! noter les titres par lots de 20. Scores bornés à 1 minimum : un article
//! noté ne retourne jamais à 0 et ne repasse pas dans le backlog. L'impact
//! dérivé est recalculé avec les seuils du classifieur (fort ≥ 60 · moyen
//! ≥ 35 · faible).

use std::sync::Arc;

use db::Database;
use llm::ollama;

const TAILLE_LOT: i64 = 10;
const PAUSE_ENTRE_LOTS_SEC: u64 = 3;
const INTERVALLE_BALAYAGES_SEC: u64 = 6 * 3600;

/// Modèle de notation : qwen2.5:3b (1,9 Go). Essai qwen3:32b le 22/09 :
/// « requires more system memory (9.0 GiB) than is available » — la machine
/// tourne 24/7 VRAM pleine, un gros modèle n'est PAS fiable pour un job de
/// fond. La tâche (pertinence 1-100 d'un titre) est dans les cordes d'un 3B.
const MODELE_NOTATION: &str = "qwen2.5:3b";

#[derive(serde::Deserialize)]
struct NoteLlm {
    i: usize,
    score: i64,
}

/// Boucle de fond : balayage du backlog au boot (après pose), puis toutes
/// les 6 h pour les nouveaux articles restés à 0.
pub async fn boucle(db: Arc<Database>) {
    // Laisser le boot se poser (collecteurs, moteurs) avant d'occuper le LLM.
    tokio::time::sleep(std::time::Duration::from_secs(120)).await;
    loop {
        balayer(&db).await;
        tokio::time::sleep(std::time::Duration::from_secs(INTERVALLE_BALAYAGES_SEC)).await;
    }
}

/// Note les articles sans score par lots, jusqu'à épuisement du backlog.
/// Un lot raté (réponse LLM cabossée) est réessayé une fois puis marqué à 1 :
/// le laisser à 0 le ferait resélectionner en boucle (infinite loop).
async fn balayer(db: &Arc<Database>) {
    let mut lots = 0usize;
    let mut notes = 0usize;
    loop {
        let articles = match db.articles_presse_sans_score(TAILLE_LOT).await {
            Ok(a) => a,
            Err(e) => {
                tracing::warn!("Notation presse (lecture): {e}");
                break;
            }
        };
        if articles.is_empty() {
            break;
        }
        lots += 1;
        let scores = match noter_lot(&articles).await {
            Ok(s) => s,
            Err(premier) => match noter_lot(&articles).await {
                Ok(s) => s,
                Err(second) => {
                    tracing::warn!(
                        "Notation presse : lot raté 2× ({premier} puis {second}) — \
                         {} article(s) marqué(s) à 1 pour débloquer le balayage",
                        articles.len()
                    );
                    for (hash, _) in &articles {
                        let _ = db.noter_article_presse(hash, 1).await;
                    }
                    notes += articles.len();
                    tokio::time::sleep(std::time::Duration::from_secs(PAUSE_ENTRE_LOTS_SEC)).await;
                    continue;
                }
            },
        };
        for (idx, (hash, _)) in articles.iter().enumerate() {
            // Titre manquant dans la réponse → 1 (sans intérêt) : on ne
            // laisse jamais un article noté à 0 (re-boucle infinie).
            let score = scores
                .iter()
                .find(|n| n.i == idx)
                .map(|n| n.score.clamp(1, 100))
                .unwrap_or(1);
            match db.noter_article_presse(hash, score as u8).await {
                Ok(()) => notes += 1,
                Err(e) => tracing::warn!("Notation presse (écriture): {e}"),
            }
        }
        tokio::time::sleep(std::time::Duration::from_secs(PAUSE_ENTRE_LOTS_SEC)).await;
    }
    if notes > 0 {
        tracing::info!("📰 Notation presse : {notes} article(s) noté(s) en {lots} lot(s)");
    }
}

/// Un appel LLM pour tout le lot : numérotation des titres, réponse attendue
/// en tableau JSON strict.
async fn noter_lot(articles: &[(String, String)]) -> anyhow::Result<Vec<NoteLlm>> {
    let titres: String = articles
        .iter()
        .enumerate()
        .map(|(i, (_, t))| format!("{i}. {t}\n"))
        .collect();
    let prompt = format!(
        "Tu es analyste financier. Note la pertinence de chaque titre pour un trader \
         forex, métaux précieux (or, argent), crypto et indices : 1 (sans intérêt) à \
         100 (à lire absolument : banque centrale, macro majeure, mouvement fort d'un \
         actif suivi). Réponds UNIQUEMENT par un tableau JSON, un objet par titre, sans \
         autre texte : [{{\"i\":0,\"score\":45}},…]\n\nTitres :\n{titres}"
    );
    let corps = serde_json::json!({
        "model": MODELE_NOTATION,
        "messages": [{ "role": "user", "content": prompt }],
        "stream": false
    });
    let url = std::env::var("OLLAMA_URL")
        .unwrap_or_else(|_| "http://localhost:11434/api/chat".to_string());
    let brut = ollama::appeler_ollama(&url, &corps).await?;
    let texte = ollama::filtrer_think(brut);
    // Extraire le tableau même si le modèle l'entoure de prose.
    let debut = texte
        .find('[')
        .ok_or_else(|| anyhow::anyhow!("pas de '[' dans la réponse LLM"))?;
    let fin = texte
        .rfind(']')
        .ok_or_else(|| anyhow::anyhow!("pas de ']' dans la réponse LLM"))?;
    Ok(serde_json::from_str(&texte[debut..=fin])?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_notes_json_strict() {
        let notes: Vec<NoteLlm> = serde_json::from_str(
            r#"[{"i":0,"score":45},{"i":1,"score":90},{"i":2,"score":3}]"#,
        )
        .unwrap();
        assert_eq!(notes.len(), 3);
        assert_eq!(notes[1].score, 90);
    }

    #[test]
    fn parse_notes_avec_champs_extra() {
        // Le modèle ajoute parfois un champ (raison, confiance…) : serde ignore.
        let notes: Vec<NoteLlm> =
            serde_json::from_str(r#"[{"i":0,"score":50,"raison":"macro"}]"#).unwrap();
        assert_eq!(notes[0].score, 50);
    }
}
