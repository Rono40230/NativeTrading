use sqlx::SqlitePool;

use llm::ollama;

// ── Hash DJB2 ────────────────────────────────────────────────────────────────

pub fn hash_titre(titre: &str) -> String {
    let mut h: u64 = 5381;
    for b in titre.bytes() {
        h = h.wrapping_mul(33).wrapping_add(b as u64);
    }
    format!("{h:016x}")
}

// ── Cache SQLite ─────────────────────────────────────────────────────────────

/// Retourne la traduction mise en cache, ou None si absente.
pub async fn lire_cache(pool: &SqlitePool, hash: &str) -> Option<String> {
    sqlx::query_scalar::<_, String>("SELECT titre_fr FROM news_traductions WHERE hash_titre = ?")
        .bind(hash)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
}

/// Persiste une traduction en cache (INSERT OR REPLACE).
pub async fn ecrire_cache(pool: &SqlitePool, hash: &str, titre_fr: &str) {
    let now = chrono::Utc::now().timestamp();
    if let Err(e) = sqlx::query(
        "INSERT OR REPLACE INTO news_traductions (hash_titre, titre_fr, traduit_le)
         VALUES (?, ?, ?)",
    )
    .bind(hash)
    .bind(titre_fr)
    .bind(now)
    .execute(pool)
    .await
    {
        tracing::warn!("Cache traduction écriture: {e}");
    }
}

// ── Traduction Ollama ─────────────────────────────────────────────────────────

const MODELE_TRADUCTION: &str = "qwen2.5:3b";

/// Traduit un texte anglais en français via Ollama (modèle léger 3B).
/// Retourne le texte original en cas d'échec (dégradation silencieuse).
pub async fn traduire(texte: &str) -> String {
    let prompt = format!(
        "Traduis ce titre financier en français naturel. \
        Réponds uniquement avec la traduction, sans guillemets ni explication.\n\n\
        Titre: {texte}"
    );

    let corps = serde_json::json!({
        "model": MODELE_TRADUCTION,
        "messages": [{ "role": "user", "content": prompt }],
        "stream": false
    });

    let url = std::env::var("OLLAMA_URL")
        .unwrap_or_else(|_| "http://localhost:11434/api/chat".to_string());

    let _permit = llm::OLLAMA_SEMAPHORE.acquire().await.ok();
    let client = &*llm::OLLAMA_HTTP_CLIENT;

    let res = client.post(&url).json(&corps).send().await;

    match res {
        Ok(r) if r.status().is_success() => {
            r.json::<llm::ReponseOllama>()
                .await
                .map(|r| r.message.content.trim().to_string())
                .unwrap_or_else(|_| texte.to_string())
        }
        _ => texte.to_string(),
    }
}

// ── Point d'entrée principal ─────────────────────────────────────────────────

/// Traduit un titre en utilisant le cache SQLite. Appelle Ollama uniquement
/// si le titre n'est pas encore connu. Dégradation silencieuse.
pub async fn traduire_avec_cache(pool: &SqlitePool, titre: &str) -> String {
    let hash = hash_titre(titre);

    if let Some(cached) = lire_cache(pool, &hash).await {
        return cached;
    }

    let traduit = traduire(titre).await;
    ecrire_cache(pool, &hash, &traduit).await;
    traduit
}

// ── Sentiment Ollama par article ─────────────────────────────────────────────

/// Lit le sentiment mis en cache pour un article, ou None si absent.
pub async fn lire_sentiment_cache(pool: &SqlitePool, hash: &str) -> Option<String> {
    sqlx::query_scalar::<_, String>("SELECT sentiment FROM news_sentiment WHERE hash_titre = ?")
        .bind(hash)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
}

/// Persiste un sentiment en cache.
pub async fn ecrire_sentiment_cache(pool: &SqlitePool, hash: &str, sentiment: &str) {
    let now = chrono::Utc::now().timestamp();
    if let Err(e) = sqlx::query(
        "INSERT OR REPLACE INTO news_sentiment (hash_titre, sentiment, analyse_le) VALUES (?, ?, ?)",
    )
    .bind(hash)
    .bind(sentiment)
    .bind(now)
    .execute(pool)
    .await
    {
        tracing::warn!("Cache sentiment écriture: {e}");
    }
}

/// Analyse le sentiment d'un titre financier via Ollama.
/// Retourne `"haussier"`, `"neutre"` ou `"baissier"`.
/// Dégradation silencieuse → `"neutre"` si Ollama indisponible.
pub async fn analyser_sentiment_avec_cache(pool: &SqlitePool, titre: &str) -> String {
    let hash = hash_titre(titre);

    if let Some(cached) = lire_sentiment_cache(pool, &hash).await {
        return cached;
    }

    let sentiment = analyser_sentiment(titre).await;
    ecrire_sentiment_cache(pool, &hash, &sentiment).await;
    sentiment
}

async fn analyser_sentiment(titre: &str) -> String {
    let prompt = format!(
        "En un seul mot parmi [haussier, neutre, baissier], quel est l'impact probable de ce titre \
        financier sur les prix des actifs (BTC, or, forex) ? Réponds uniquement avec un des trois mots.\n\n\
        Titre: {titre}"
    );

    let corps = serde_json::json!({
        "model": MODELE_TRADUCTION,
        "messages": [{ "role": "user", "content": prompt }],
        "stream": false
    });

    let url = std::env::var("OLLAMA_URL")
        .unwrap_or_else(|_| "http://localhost:11434/api/chat".to_string());

    let _permit = llm::OLLAMA_SEMAPHORE.acquire().await.ok();
    let client = &*llm::OLLAMA_HTTP_CLIENT;

    let res = client.post(&url).json(&corps).send().await;

    let texte = match res {
        Ok(r) if r.status().is_success() => {
            r.json::<llm::ReponseOllama>()
                .await
                .map(|r| r.message.content.trim().to_lowercase())
                .unwrap_or_default()
        }
        _ => return "neutre".to_string(),
    };

    if texte.contains("haussier") {
        "haussier".to_string()
    } else if texte.contains("baissier") {
        "baissier".to_string()
    } else {
        "neutre".to_string()
    }
}

/// Traduit un texte long (corps d'article) — sans cache (trop volumineux).
pub async fn traduire_contenu(texte: &str) -> String {
    // Tronquer à 3000 caractères pour éviter les timeouts
    let extrait = if texte.len() > 3000 {
        &texte[..3000]
    } else {
        texte
    };

    let prompt = format!(
        "Traduis ce texte financier en français naturel et fluide. \
        Réponds uniquement avec la traduction.\n\n{extrait}"
    );

    match ollama::interroger_chat_modele(&[("user".to_string(), prompt)], MODELE_TRADUCTION).await {
        Ok(t) => t,
        Err(_) => texte.to_string(),
    }
}

// ── Traduction stricte + brief LLM (revue de presse) ──────────────────────────

/// Une traduction est réussie si elle diffère de l'original (contrat :
/// `traduire` rend le texte original en cas d'échec).
pub fn traduction_reussie(original: &str, traduit: &str) -> bool {
    !traduit.trim().is_empty() && traduit.trim() != original.trim()
}

/// Traduction STRICTE pour la presse : None = échec (rien n'est caché).
/// Diffère de `traduire_avec_cache` qui cache aussi les échecs (rend
/// l'original) — inacceptable pour la règle « porte d'entrée ».
pub async fn traduire_avec_cache_strict(pool: &SqlitePool, titre: &str) -> Option<String> {
    let hash = hash_titre(titre);
    if let Some(cached) = lire_cache(pool, &hash).await {
        return Some(cached);
    }
    let traduit = traduire(titre).await;
    if traduction_reussie(titre, &traduit) {
        ecrire_cache(pool, &hash, &traduit).await;
        Some(traduit)
    } else {
        None
    }
}

/// Génération du brief : un appel Ollama, entrée compilée par l'appelant
/// (titres FR + scores + thèmes), sortie markdown brute.
pub async fn generer_brief_llm(entree: &str) -> Option<String> {
    let prompt = format!(
        "Tu es analyste financier. Rédige en français un brief matinal en markdown :\n\
         ## Contexte marché\n3 lignes sur les thèmes dominants.\n\n\
         ## Articles marquants\nPour chaque article : 2-3 lignes (impact, assets).\n\n\
         Articles des dernières 24h (score sur 100, thème) :\n{entree}"
    );
    let corps = serde_json::json!({
        "model": MODELE_TRADUCTION,
        "messages": [{ "role": "user", "content": prompt }],
        "stream": false
    });
    let url = std::env::var("OLLAMA_URL")
        .unwrap_or_else(|_| "http://localhost:11434/api/chat".to_string());
    let _permit = llm::OLLAMA_SEMAPHORE.acquire().await.ok();
    let res = llm::OLLAMA_HTTP_CLIENT.post(&url).json(&corps).send().await;
    match res {
        Ok(r) if r.status().is_success() => r
            .json::<llm::ReponseOllama>().await.ok()
            .map(|r| r.message.content.trim().to_string())
            .filter(|s| !s.is_empty()),
        _ => None,
    }
}

#[cfg(test)]
mod tests_strict {
    use super::*;

    #[test]
    fn eval_traduction() {
        assert!(traduction_reussie("Fed cuts rates", "La Fed baisse ses taux"));
        assert!(!traduction_reussie("Fed cuts rates", "Fed cuts rates"));
        assert!(!traduction_reussie("Fed cuts rates", "  "));
    }
}
