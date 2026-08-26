//! Backfill automatique de l'historique (décision propriétaire 2026-08-15).
//!
//! Le runtime doit se comporter comme un graphique TradingView : à
//! l'ouverture, l'historique est là. Avant le cold start (replay) de chaque
//! couple (asset × TF), les bougies manquantes depuis la dernière connue
//! sont récupérées via le provider REST Bybit (crypto) — les trous
//! dus aux arrêts de l'app (nuits, week-ends, pannes) se rebouchent seuls.
//!
//! Déclenchement : enregistrement d'un couple au runtime (démarrage de
//! l'app ou nouvel asset/TF coché dans l'UI). Les micro-trous en cours de
//! session (brève coupure WS) ne déclenchent PAS de backfill — le WS
//! reprend le flux depuis l'instant présent.
//!
//! Limite v1 : un seul appel REST par couple (1 000 bougies max) — comble
//! le trou récent, suffisant pour réarmer les moteurs (le v12 a besoin de
//! ~7 jours ; 1 000 bougies M1 ≈ 17 h). La pagination complète reste à
//! venir (les assets MT5 se rebouchent par le push historique de l'EA).

use std::sync::Arc;

use chrono::Utc;
use common::{Asset, Timeframe};
use db::Database;

use crate::providers::BinanceProvider;
use crate::DataProvider;

/// Limite d'un appel REST Bybit (bougies).
pub const LIMITE_REST: i64 = 1000;

/// Calcule le nombre de bougies à récupérer pour combler jusqu'à maintenant.
///
/// - aucune donnée → la limite REST (démarrage à froid d'un couple) ;
/// - données présentes → les barres manquantes depuis la dernière connue,
///   hors bougie en cours de formation, plafonnées à la limite REST.
///
/// Fonction pure → testable.
pub fn bars_manquantes(dernier_ts: Option<i64>, maintenant: i64, tf_sec: i64) -> i64 {
    match dernier_ts {
        None => LIMITE_REST,
        Some(dernier) => {
            if dernier >= maintenant {
                return 0;
            }
            // Barres écoulées depuis la dernière connue, sans la bougie en
            // cours de formation.
            let passees = (maintenant - dernier) / tf_sec.max(1);
            (passees - 1).clamp(0, LIMITE_REST)
        }
    }
}

/// Comble l'historique d'un couple via le provider REST, si nécessaire.
/// Retourne le nombre de bougies réellement insérées (0 si à jour).
pub async fn combler_historique(
    db: &Arc<Database>,
    asset: Asset,
    tf: Timeframe,
) -> anyhow::Result<u64> {
    let maintenant = Utc::now().timestamp();
    let dernier = db
        .timestamp_derniere_bougie_chart(&asset, &tf)
        .await
        .ok()
        .flatten();
    let manquantes = bars_manquantes(dernier, maintenant, tf.minutes() as i64 * 60);
    if manquantes < 1 {
        return Ok(0);
    }

    let provider = BinanceProvider;
    let bougies = provider
        .fetch_candles(asset.clone(), tf, manquantes as usize)
        .await?;
    if bougies.is_empty() {
        return Ok(0);
    }
    let inserees = db
        .inserer_bougies_avec_source(&asset, &tf, &bougies, "binance")
        .await?;
    Ok(inserees)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn maintenant() -> i64 {
        1_786_800_000 // fixe pour des tests déterministes
    }

    #[test]
    fn aucune_donnee_demande_la_limite() {
        assert_eq!(bars_manquantes(None, maintenant(), 900), LIMITE_REST);
        assert_eq!(bars_manquantes(None, maintenant(), 86_400), LIMITE_REST);
    }

    #[test]
    fn a_jour_ne_demande_rien() {
        let m = maintenant();
        // Dernière bougie il y a moins d'une période → rien à combler.
        assert_eq!(bars_manquantes(Some(m - 60), m, 900), 0);
        assert_eq!(bars_manquantes(Some(m), m, 900), 0);
        assert_eq!(bars_manquantes(Some(m + 100), m, 900), 0);
    }

    #[test]
    fn trou_calcule_sans_la_bougie_en_cours() {
        let m = maintenant();
        // Dernière bougie il y a 10 barres M15 → 9 manquantes (la 10e est
        // la bougie en cours de formation).
        assert_eq!(bars_manquantes(Some(m - 10 * 900), m, 900), 9);
    }

    #[test]
    fn trou_geant_plafonne_a_la_limite() {
        let m = maintenant();
        // Une semaine de trou M1 → plafonné à 1000.
        assert_eq!(bars_manquantes(Some(m - 7 * 86_400), m, 60), LIMITE_REST);
    }
}
