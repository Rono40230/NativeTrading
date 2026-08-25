//! Crate `notifications` — envoi de notifications (Telegram, et futur email).
//! Extrait du monolithe `api` (phase 1.6a). Cluster fermé : aucune dépendance
//! vers le métier ; consommé uniquement via `notifications::telegram`
//! (l'émetteur unique est le writer des signaux officiels).
pub mod telegram;

use std::sync::LazyLock;

/// Client HTTP partagé du crate notifications (Telegram Bot API, timeout 10 s).
/// Recréé localement pour éviter une dépendance api→notifications.
pub static HTTP_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
});
