//! Configuration des workers d'ingestion — lue depuis la table `configuration`,
//! pilotable depuis la vue Données de l'UI (aucune valeur hardcodée).
//!
//! Clés gérées :
//! - `worker_timeframes`       : JSON array `["M5","M15","H1","D1"]`
//! - `worker_historique_mois`  : profondeur de backfill en mois (1..=24)
//! - `worker_actif_bybit`      : "0" → le worker Bybit skippe ses sessions
//! - `worker_actif_ig`         : "0" → le worker IG skippe ses cycles

use std::sync::Arc;

use common::Timeframe;
use db::Database;

/// Clé de configuration : timeframes communs aux workers (JSON array).
pub const CLE_TIMEFRAMES: &str = "worker_timeframes";
/// Clé de configuration : profondeur d'historique en mois.
pub const CLE_HISTORIQUE_MOIS: &str = "worker_historique_mois";
/// Clé de configuration : worker Bybit activé ("0" = désactivé).
pub const CLE_ACTIF_BYBIT: &str = "worker_actif_bybit";
/// Clé de configuration : worker IG activé ("0" = désactivé).
pub const CLE_ACTIF_IG: &str = "worker_actif_ig";

/// Timeframes par défaut si la clé est absente ou illisible en DB.
pub const TIMEFRAMES_DEFAUT: &[Timeframe] =
    &[Timeframe::M5, Timeframe::M15, Timeframe::H1, Timeframe::D1];

/// Profondeur d'historique par défaut (mois).
pub const HISTORIQUE_MOIS_DEFAUT: i64 = 6;

/// Ordre canonique des timeframes — garantit des topics/planifications stables
/// quel que soit l'ordre sérialisé dans la configuration.
const ORDRE_CANONIQUE: &[Timeframe] = &[
    Timeframe::M1,
    Timeframe::M5,
    Timeframe::M15,
    Timeframe::M30,
    Timeframe::H1,
    Timeframe::H4,
    Timeframe::D1,
    Timeframe::W1,
];

/// Parse une valeur de configuration `'["M5","H1"]'` en timeframes dédoublonnés
/// et triés dans l'ordre canonique. Les entrées inconnues sont ignorées ;
/// retourne les défauts si rien n'est reconnu. Fonction pure → testable.
pub fn parse_timeframes(valeur: &str) -> Vec<Timeframe> {
    let brute: Vec<String> = serde_json::from_str(valeur).unwrap_or_default();
    let mut retenus: Vec<Timeframe> = ORDRE_CANONIQUE
        .iter()
        .copied()
        .filter(|tf| brute.iter().any(|s| s == tf.as_str()))
        .collect();
    if retenus.is_empty() {
        retenus = TIMEFRAMES_DEFAUT.to_vec();
    }
    retenus
}

/// Sérialise des timeframes en valeur de configuration `'["M5","H1"]'`.
pub fn serialise_timeframes(timeframes: &[Timeframe]) -> String {
    serde_json::to_string(
        &timeframes
            .iter()
            .map(|tf| tf.as_str())
            .collect::<Vec<&str>>(),
    )
    .unwrap_or_else(|_| serde_json::to_string(&["M5", "M15", "H1", "D1"]).unwrap_or_default())
}

/// Lit les timeframes des workers. Toute erreur DB retombe sur les défauts —
/// un worker ne doit jamais crasher pour un problème de configuration.
pub async fn lire_timeframes(db: &Arc<Database>) -> Vec<Timeframe> {
    match db.lire_config(CLE_TIMEFRAMES).await {
        Ok(Some(valeur)) => parse_timeframes(&valeur),
        _ => TIMEFRAMES_DEFAUT.to_vec(),
    }
}

/// Lit un flag d'activation worker. Absent ou illisible → actif (opt-out) ;
/// la valeur "0" (trim) est le seul moyen de désactiver.
pub async fn lire_actif(db: &Arc<Database>, cle: &str) -> bool {
    match db.lire_config(cle).await {
        Ok(Some(valeur)) => valeur.trim() != "0",
        _ => true,
    }
}

/// Lit la profondeur d'historique en mois, bornée 1..=24 pour rester
/// compatible avec les quotas IG et la taille des requêtes.
pub async fn lire_historique_mois(db: &Arc<Database>) -> i64 {
    match db.lire_config(CLE_HISTORIQUE_MOIS).await {
        Ok(Some(valeur)) => valeur
            .trim()
            .parse::<i64>()
            .unwrap_or(HISTORIQUE_MOIS_DEFAUT)
            .clamp(1, 24),
        _ => HISTORIQUE_MOIS_DEFAUT,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_timeframes_nominal_et_ordre_canonique() {
        // Ordre d'entrée inversé + doublon + valeur inconnue → trié, dédoublonné.
        let tfs = parse_timeframes(r#"["H1","M5","H1","M15","XYZ","D1"]"#);
        assert_eq!(
            tfs,
            vec![Timeframe::M5, Timeframe::M15, Timeframe::H1, Timeframe::D1]
        );
    }

    #[test]
    fn parse_timeframes_vide_ou_invalide_retombe_sur_defauts() {
        assert_eq!(parse_timeframes("[]"), TIMEFRAMES_DEFAUT.to_vec());
        assert_eq!(parse_timeframes("pas du tout json"), TIMEFRAMES_DEFAUT.to_vec());
        assert_eq!(parse_timeframes(r#"["ABC"]"#), TIMEFRAMES_DEFAUT.to_vec());
    }

    #[test]
    fn serialise_puis_parse_fait_un_tour_complet() {
        let tfs = vec![Timeframe::M1, Timeframe::H4, Timeframe::W1];
        assert_eq!(parse_timeframes(&serialise_timeframes(&tfs)), tfs);
    }
}
