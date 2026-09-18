//! Périmètre straddle du runtime (18/09) — extrait de runtime_tick.rs.

/// Périmètre par défaut (incarne les décisions 15/09 : BTC seule crypto,
/// tier 1 + majeures). Éditable par le propriétaire via la config
/// `perimetre_straddle` (modale « Choix des Assets & créneaux »).
const PERIMETRE_STRADDLE_DEFAUT: &[&str] = &["BTC", "XAUUSD", "XAGUSD", "SP500", "NAS100", "DAX", "EURUSD", "GBPUSD", "USDJPY"];

/// Périmètre EFFECTIF du straddle : config si présente, défaut sinon.
/// Lu à chaque tick (resynchro ≤ 60 s, comme l'armement SMC — jamais de
/// relance nécessaire).
pub async fn lire_perimetre_straddle(db: &db::Database) -> Vec<String> {
    db.lire_config("perimetre_straddle")
        .await
        .ok()
        .flatten()
        .and_then(|v| serde_json::from_str::<Vec<String>>(&v).ok())
        .filter(|l| !l.is_empty())
        .unwrap_or_else(|| PERIMETRE_STRADDLE_DEFAUT.iter().map(|s| s.to_string()).collect())
}
