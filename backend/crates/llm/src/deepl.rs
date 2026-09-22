//! Client DeepL (API gratuite) — traduction EN→FR des titres presse.
//!
//! Décision 22/09 : DeepL devient la voie principale (meilleure qualité FR,
//! noms propres préservés — hy-mt2 hallucinait les noms d'entreprises) ;
//! Ollama reste le repli si l'API échoue, le quota est épuisé ou la clé
//! n'est pas configurée. Volume ~100 k caractères/mois, très sous le
//! plafond gratuit de 500 k.

use serde::Deserialize;

use crate::HTTP_CLIENT;

const URL_API: &str = "https://api-free.deepl.com/v2/translate";

#[derive(Deserialize)]
struct ReponseDeepL {
    translations: Vec<Traduction>,
}

#[derive(Deserialize)]
struct Traduction {
    text: String,
}

/// Traduit `texte` vers le français via DeepL.
/// `None` = échec (réseau, quota, clé invalide) → l'appelant replie sur Ollama.
pub async fn traduire(cle: &str, texte: &str) -> Option<String> {
    let corps = serde_json::json!({ "text": [texte], "target_lang": "FR" });
    let res = HTTP_CLIENT
        .post(URL_API)
        .header("Authorization", format!("DeepL-Auth-Key {cle}"))
        .json(&corps)
        .send()
        .await
        .ok()?;
    if !res.status().is_success() {
        tracing::warn!("DeepL HTTP {} — repli traduction locale", res.status());
        return None;
    }
    let rep: ReponseDeepL = res.json().await.ok()?;
    let t = rep
        .translations
        .into_iter()
        .next()?
        .text
        .trim()
        .to_string();
    if t.is_empty() { None } else { Some(t) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_reponse_deepl() {
        // Champ detected_source_language inconnu du struct → ignoré par serde.
        let json = r#"{"translations":[{"detected_source_language":"EN","text":"La Fed baisse ses taux"}]}"#;
        let rep: ReponseDeepL = serde_json::from_str(json).unwrap();
        assert_eq!(rep.translations.len(), 1);
        assert_eq!(rep.translations[0].text, "La Fed baisse ses taux");
    }

    #[test]
    fn parse_reponse_vide() {
        let rep: ReponseDeepL = serde_json::from_str(r#"{"translations":[]}"#).unwrap();
        assert!(rep.translations.is_empty());
    }
}
