//! Lecteurs des réglages SMC (config kv) — partagés entre l'armement du
//! runtime (runtime_tick) et le re-jeu paramétrique (smc_rejeu). Extraits de
//! runtime_tick.rs (limite 600 lignes, pre-commit).

use smc::v12::signals::Tp3Reglage;

/// TP3 réglable : mode (true = liquidité lointaine, défaut) + R fixe/repli
/// (défaut 3.0, borné 3-10 — validation > TP2 côté carte).
pub async fn lire_tp3_reglage(db: &db::Database) -> smc::v12::signals::Tp3Reglage {
    let lointaine = db
        .lire_config("smc_tp3_mode")
        .await
        .ok()
        .flatten()
        .map(|v| !v.trim().eq_ignore_ascii_case("rfixe"))
        .unwrap_or(true);
    let rfixe = db
        .lire_config("smc_tp3_rfixe")
        .await
        .ok()
        .flatten()
        .and_then(|v| v.trim().parse::<f64>().ok())
        .map(|v| v.clamp(3.0, 10.0))
        .unwrap_or(3.0);
    smc::v12::signals::Tp3Reglage { lointaine, rfixe }
}

/// Trailing stop après TP2 (défaut inactif — mesuré par le re-jeu avant
/// d'en faire éventuellement un défaut).
pub async fn lire_trailing_reglage(db: &db::Database) -> Option<f64> {
    let actif = db
        .lire_config("smc_tp3_trailing")
        .await
        .ok()
        .flatten()
        .map(|v| v.trim() == "1")
        .unwrap_or(false);
    if !actif {
        return None;
    }
    Some(
        db.lire_config("smc_tp3_trailing_r")
            .await
            .ok()
            .flatten()
            .and_then(|v| v.trim().parse::<f64>().ok())
            .map(|v| v.clamp(0.1, 1.0))
            .unwrap_or(0.5),
    )
}

pub async fn lire_tp2_reglage(db: &db::Database) -> f64 {
    db.lire_config("smc_tp2_mult")
        .await
        .ok()
        .flatten()
        .and_then(|v| v.trim().parse::<f64>().ok())
        .map(|v| v.clamp(1.0, 4.0))
        .unwrap_or(2.0)
}

pub async fn lire_tp1_reglage(db: &db::Database) -> f64 {
    db.lire_config("smc_tp1_mult")
        .await
        .ok()
        .flatten()
        .and_then(|v| v.trim().parse::<f64>().ok())
        .map(|v| v.clamp(0.2, 1.5))
        .unwrap_or(0.6)
}

