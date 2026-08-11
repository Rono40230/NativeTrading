//! Infrastructure Telegram : envoi HTTP et lecture des tokens depuis la DB.
//!
//! La logique de formatage et d'envoi effective est dans `telegram_worker`.
//! Ce module n'expose que les primitives partagées.
use db::Database;

/// Envoie un message texte brut via l'API Telegram (parse_mode HTML).
/// Retourne une erreur si l'envoi échoue — le caller décide du retry.
pub async fn post_message(token: &str, chat_id: &str, texte: &str) -> anyhow::Result<()> {
    let url = format!("https://api.telegram.org/bot{}/sendMessage", token);
    let client = &*crate::http_client::HTTP_CLIENT;
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

// ── SUPPRIMÉ (conservé pour éviter les erreurs de compilation) ────────────────
// Les fonctions notifier_telegram, notifier_telegram_rocket, RocketTelegramData
// ont été remplacées par telegram_worker.rs (worker centralisé).

#[allow(dead_code)]
pub struct RocketTelegramData {
    pub ticker:              String,
    pub phase:               String,
    pub score_composite:     i64,
    pub entree_limite:       f64,
    pub entree_stop:         f64,
    pub type_entree_ideal:   String,
    pub sl:                  f64,
    pub tp1:                 f64,
    pub tp2:                 f64,
    pub niveau_invalidation: f64,
    pub trailing_coeff:      f64,
    pub llm_raison:          Option<String>,
}

/// Envoie une notification Telegram dédiée aux signaux Rockets.
/// Spawn une tâche détachée — ne bloque jamais le caller.
#[allow(dead_code)]
pub fn notifier_telegram_rocket(data: RocketTelegramData, token: String, chat_id: String) {
    if token.is_empty() || chat_id.is_empty() {
        return;
    }
    tokio::spawn(async move {
        if let Err(e) = envoyer_rocket(&token, &chat_id, &data).await {
            tracing::warn!("Telegram Rocket {}: {}", data.ticker, e);
        }
    });
}

#[allow(dead_code)]
async fn envoyer_rocket(token: &str, chat_id: &str, d: &RocketTelegramData) -> anyhow::Result<()> {
    let ref_prix = d.entree_limite.min(d.entree_stop);
    let phase_label = match d.phase.as_str() {
        "breakout"    => "Breakout",
        "prelancement" => "Pré-lancement",
        "compression" => "Compression",
        other         => other,
    };
    let entree_ideal_label = if d.type_entree_ideal == "limite" { "Limite" } else { "Stop" };

    let raison = d.llm_raison.as_deref().unwrap_or("—");

    let texte = format!(
        "🚀 <b>Rocket — {ticker} · {phase}</b>\n\
        Score : <b>{score}/100</b>\n\
        \n\
        <code>\
Entrée Limite  : {el}\n\
Entrée Stop    : {es}\n\
Entrée idéale  : {ei}\n\
SL             : {sl}\n\
TP1            : {tp1}\n\
TP2            : {tp2}\n\
Invalidation   : {inv}\n\
\n\
Trailing TP3   : {trail}×</code>\n\
📝 {raison}",
        ticker = d.ticker,
        phase  = phase_label,
        score  = d.score_composite,
        el     = fmt(d.entree_limite, ref_prix),
        es     = fmt(d.entree_stop,   ref_prix),
        ei     = entree_ideal_label,
        sl     = fmt(d.sl,  ref_prix),
        tp1    = fmt(d.tp1, ref_prix),
        tp2    = fmt(d.tp2, ref_prix),
        inv    = fmt(d.niveau_invalidation, ref_prix),
        trail  = d.trailing_coeff,
        raison = raison,
    );

    let url = format!("https://api.telegram.org/bot{}/sendMessage", token);
    let client = &*crate::http_client::HTTP_CLIENT;

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

    tracing::info!("✉️  Telegram Rocket envoyé → {}", d.ticker);
    Ok(())
}

/// Lit les tokens Telegram depuis la DB (via Database wrapper) avec repli env.
#[allow(dead_code)]
pub async fn lire_tokens_telegram(db: &Database) -> (String, String) {
    lire_tokens_pool(db.pool()).await
}

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
