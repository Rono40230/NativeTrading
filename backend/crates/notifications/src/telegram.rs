//! Infrastructure Telegram : envoi HTTP et lecture des tokens depuis la DB.
//!
//! La logique de formatage et d'envoi effective est dans `telegram_worker`.
//! Ce module n'expose que les primitives partagées.

/// Envoie un message texte brut via l'API Telegram (parse_mode HTML).
/// Retourne une erreur si l'envoi échoue — le caller décide du retry.
pub async fn post_message(token: &str, chat_id: &str, texte: &str) -> anyhow::Result<()> {
    let url = format!("https://api.telegram.org/bot{}/sendMessage", token);
    let client = &*crate::HTTP_CLIENT;
    let resp = client
        .post(&url)
        .json(&serde_json::json!({
            "chat_id": chat_id,
            "text": texte,
            "parse_mode": "HTML"
        }))
        .send()
        .await?;
    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("Telegram API: {}", body);
    }
    Ok(())
}

// ── Primitives partagées ──────────────────────────────────────────────────────

/// Lit les tokens Telegram depuis un SqlitePool brut (pour les workers sans Database wrapper).
pub async fn lire_tokens_pool(pool: &sqlx::SqlitePool) -> (String, String) {
    let token = sqlx::query_scalar::<_, String>(
        "SELECT valeur FROM configuration WHERE cle = 'telegram_bot_token'",
    )
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .filter(|s| !s.is_empty())
    .unwrap_or_else(|| std::env::var("TELEGRAM_BOT_TOKEN").unwrap_or_default());

    let chat_id = sqlx::query_scalar::<_, String>(
        "SELECT valeur FROM configuration WHERE cle = 'telegram_chat_id'",
    )
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .filter(|s| !s.is_empty())
    .unwrap_or_else(|| std::env::var("TELEGRAM_CHAT_ID").unwrap_or_default());

    (token, chat_id)
}

/// Formate un prix : 2 décimales si ≥ 100, sinon 5.
pub fn fmt(prix: f64, reference: f64) -> String {
    if reference >= 100.0 {
        format!("{:.2}", prix)
    } else {
        format!("{:.5}", prix)
    }
}
