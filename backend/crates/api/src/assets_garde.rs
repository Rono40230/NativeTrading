//! Garde-fou d'alimentation des assets (incident 07-09/10).
//!
//! Un asset armé dans une stratégie (SMC, KDJ, straddle) DOIT être nourri
//! en bougies — or le drapeau `assets.actif` pilote les workers (Bybit/EA)
//! et rien ne le couplait à l'armement : le 07/10, 15 assets retirés de la
//! liste ont coupé le flux de 5 des 6 assets KDJ (plus aucune bougie, plus
//! aucun signal — découvert via 3 trades orphelins). Ce garde réactive
//! automatiquement tout asset armé trouvé inactif, à chaque tick (60 s) :
//! l'ajout/l'armement d'un asset garantit son alimentation, par n'importe
//! quel chemin (UI, API, SQL). Il ne désactive JAMAIS — les assets
//! d'observation non armés restent sous contrôle du propriétaire.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use sqlx::Row;

/// Union des assets armés à garder alimentés — SMC : au moins un TF armé ;
/// KDJ : périmètre explicite (None = clé absente, legacy « tous armés » —
/// rien à garantir nommément) ; straddle : périmètre explicite.
pub(crate) fn assets_a_garder(
    smc: &HashMap<String, HashSet<String>>,
    kdj: Option<&HashSet<String>>,
    straddle: &[String],
) -> Vec<String> {
    let mut tous: HashSet<String> = smc
        .iter()
        .filter(|(_, tfs)| !tfs.is_empty())
        .map(|(a, _)| a.clone())
        .collect();
    if let Some(k) = kdj {
        tous.extend(k.iter().cloned());
    }
    tous.extend(straddle.iter().cloned());
    let mut v: Vec<String> = tous.into_iter().collect();
    v.sort();
    v
}

/// Réactive les assets armés trouvés inactifs (idempotent, log par geste).
pub async fn reconcilier(db: &Arc<db::Database>) {
    let smc = crate::reglages_smc::lire_couples_armes(db).await;
    let kdj = crate::kdj_handlers::assets_armes_kdj(db).await;
    let straddle = crate::runtime_perimetre::lire_perimetre_straddle(db).await;
    let voulus = assets_a_garder(&smc, kdj.as_ref(), &straddle);
    if voulus.is_empty() {
        return;
    }
    let marques: Vec<String> = voulus.iter().map(|a| format!("'{a}'")).collect();
    let sql = format!(
        "SELECT id FROM assets WHERE actif = 0 AND id IN ({})",
        marques.join(",")
    );
    let Ok(morts) = sqlx::query(&sql).fetch_all(db.pool()).await else {
        return;
    };
    for r in morts {
        let id: String = r.try_get("id").unwrap_or_default();
        if id.is_empty() {
            continue;
        }
        let _ = sqlx::query("UPDATE assets SET actif = 1 WHERE id = ?")
            .bind(&id)
            .execute(db.pool())
            .await;
        tracing::info!("🩹 Asset {id} réactivé automatiquement (armé dans une stratégie) — le flux de bougies reprend en ≤ 60 s");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tfs(v: &[&str]) -> HashSet<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn union_des_trois_perimetres_deduplique_et_trie() {
        let mut smc = HashMap::new();
        smc.insert("XAUUSD".into(), tfs(&["M5", "M15"]));
        smc.insert("EURUSD".into(), tfs(&[])); // configuré mais désarmé
        let kdj = tfs(&["BNB", "XAUUSD"]); // doublon voulu
        let straddle = vec!["DAX".to_string()];
        let v = assets_a_garder(&smc, Some(&kdj), &straddle);
        assert_eq!(v, vec!["BNB".to_string(), "DAX".to_string(), "XAUUSD".to_string()]);
    }

    #[test]
    fn kdj_absent_legacy_najoute_rien() {
        let mut smc = HashMap::new();
        smc.insert("BTC".into(), tfs(&["M5"]));
        assert_eq!(assets_a_garder(&smc, None, &[]), vec!["BTC".to_string()]);
    }

    #[test]
    fn smc_sans_tf_arme_nest_pas_garde() {
        let mut smc = HashMap::new();
        smc.insert("EURJPY".into(), tfs(&[]));
        assert!(assets_a_garder(&smc, None, &[]).is_empty());
    }
}
