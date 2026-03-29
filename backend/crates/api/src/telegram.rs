//! Envoi de notifications Telegram sur chaque nouveau signal validé.
//!
//! Lit `TELEGRAM_BOT_TOKEN` et `TELEGRAM_CHAT_ID` depuis les variables
//! d'environnement (chargées via `telegram.env` au démarrage).
//!
//! Appelé en tâche Tokio detachée depuis `signal_filtre.rs` → aucun impact
//! sur la latence du signal engine si Telegram est lent ou indisponible.
use common::Signal;

/// Envoie un message Telegram pour le signal donné.
/// Spawn une tâche détachée — ne bloque jamais le caller.
pub fn notifier_telegram(signal: Signal) {
    let token = std::env::var("TELEGRAM_BOT_TOKEN").unwrap_or_default();
    let chat_id = std::env::var("TELEGRAM_CHAT_ID").unwrap_or_default();

    if token.is_empty() || chat_id.is_empty() {
        return; // Telegram non configuré — silencieux
    }

    tokio::spawn(async move {
        if let Err(e) = envoyer(&token, &chat_id, &signal).await {
            tracing::warn!(
                "Telegram: échec envoi signal {}/{}: {}",
                signal.asset.as_str(),
                signal.timeframe.as_str(),
                e
            );
        }
    });
}

async fn envoyer(token: &str, chat_id: &str, signal: &Signal) -> anyhow::Result<()> {
    let dir_str = format!("{:?}", signal.direction).to_uppercase();
    let est_long = dir_str.contains("LONG");
    let strat_lower = signal.strategie.to_lowercase();
    let est_straddle = strat_lower.contains("straddle");

    // Emoji selon la stratégie
    let emoji_strat = if strat_lower.contains("rocket") {
        "🚀"
    } else if est_straddle {
        "🌪️"
    } else {
        "📊"
    };

    let action = if est_straddle {
        format!(
            "🌪️ 2 positions simultanées sur <b>{}</b>",
            signal.asset.as_str()
        )
    } else if est_long {
        format!("📈 J'achète <b>{}</b>", signal.asset.as_str())
    } else {
        format!("📉 Je vends <b>{}</b>", signal.asset.as_str())
    };

    let ref_prix = signal.prix_entree;

    // Lignes TP sans balises imbriquées (on est déjà dans <code>)
    let tps = signal
        .take_profit
        .iter()
        .enumerate()
        .map(|(i, &tp)| {
            let pct = (tp - ref_prix) / ref_prix * 100.0;
            let signe = if pct >= 0.0 { "+" } else { "" };
            format!(
                "🎯 TP{}      {}  ({}{:.2}%)",
                i + 1,
                fmt(tp, ref_prix),
                signe,
                pct
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let sl_pct = (signal.stop_loss - ref_prix) / ref_prix * 100.0;
    let sl_signe = if sl_pct >= 0.0 { "+" } else { "" };

    let texte = format!(
        "{emoji} <b>Stratégie {strategie}</b>\n\
        ⭐ Score <b>{score:.0}/100</b>\n\
        \n\
        {action}\n\
        \n\
        <code>\
📍 Entrée   {entree}\n\
🛑 Stop     {sl}  ({sl_signe}{sl_pct:.2}%)\n\
{tps}</code>",
        emoji = emoji_strat,
        strategie = signal.strategie,
        score = signal.score,
        action = action,
        entree = fmt(ref_prix, ref_prix),
        sl = fmt(signal.stop_loss, ref_prix),
        sl_signe = sl_signe,
        sl_pct = sl_pct,
        tps = tps,
    );

    let url = format!("https://api.telegram.org/bot{}/sendMessage", token);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

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
        anyhow::bail!("Telegram API erreur: {}", body);
    }

    tracing::info!(
        "✉️  Telegram signal envoyé → {}/{}",
        signal.asset.as_str(),
        signal.timeframe.as_str()
    );
    Ok(())
}

/// Formate un prix : 2 décimales si ≥ 100, sinon 5
fn fmt(prix: f64, reference: f64) -> String {
    if reference >= 100.0 {
        format!("{:.2}", prix)
    } else {
        format!("{:.5}", prix)
    }
}
