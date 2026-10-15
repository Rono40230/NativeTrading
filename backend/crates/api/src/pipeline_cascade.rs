//! Pipeline = référence unique des assets (doctrine owner 15/10).
//!
//! « La seule source de référence pour tous les assets affichés et analysés
//! dans l'app est 📦 Données — Pilotage du pipeline. » Conséquences :
//!   • RETRAIT du pipeline = retrait PARTOUT proprement : l'asset quitte le
//!     périmètre straddle (et ses cases de matrice), ses couples SMC, la
//!     liste KDJ — et les caches d'analyse se vident pour que briefing,
//!     volatilité et matrice reflètent l'univers SANS délai.
//!   • AJOUT au pipeline = disponible partout, ARMÉ NULLE PART (doctrine
//!     anti-auto-arm du 10/10) — l'armement reste un geste du propriétaire.
//!   • La base n'oublie jamais : les bougies/vécu restent, re-sélectionner
//!     remonte immédiatement.
//!   • Les positions ouvertes d'un asset retiré ne sont jamais bloquantes :
//!     le rattrapage (straddle 14/10, SMC 18/09) les clôture au dernier prix
//!     connu — la confirmation UI les AFFICHE avant le geste.
//!
//! Ce module porte la cascade unique, appelée par le PUT pipeline ET par le
//! garde-fou (assets_garde) inversé : « armé ⇒ alimenté seulement s'il est
//! au pipeline » — un armé hors pipeline est désarmé, plus jamais réactivé
//! en silence (le garde-fou d'avant combattait la décision du propriétaire).


use sqlx::Row;

use db::Database;

/// Ce que le retrait va désarmer + les positions concernées — pour la
/// confirmation de l'UI (lecture seule, aucun changement).
#[derive(serde::Serialize)]
pub struct ImpactRetrait {
    pub positions_ouvertes: i64,
    /// Couples SMC armés sur cet asset (TF compris).
    pub smc_couples: Vec<String>,
    /// Membre de la liste KDJ explicite ?
    pub kdj_arme: bool,
    /// Membre du périmètre straddle ?
    pub straddle: bool,
}

/// Lecture d'impact AVANT retrait (affichée par la modale de confirmation).
pub async fn impact_retrait(db: &Database, id: &str) -> ImpactRetrait {
    let positions_ouvertes = sqlx::query_scalar(
        "SELECT COUNT(*) FROM signaux
         WHERE asset = ? AND statut = 'Actif' AND rempli = 1",
    )
    .bind(id)
    .fetch_one(db.pool())
    .await
    .unwrap_or(0);

    let smc = crate::reglages_smc::lire_couples_armes(db).await;
    let mut smc_couples: Vec<String> = smc
        .get(id)
        .map(|tfs| {
            let mut v: Vec<String> = tfs.iter().cloned().collect();
            v.sort();
            v
        })
        .unwrap_or_default();
    smc_couples.sort();

    let kdj_arme = crate::kdj_handlers::assets_armes_kdj(db)
        .await
        .map(|s| s.contains(id))
        .unwrap_or(false);

    let straddle = crate::runtime_perimetre::lire_perimetre_straddle(db)
        .await
        .iter()
        .any(|a| a == id);

    ImpactRetrait { positions_ouvertes, smc_couples, kdj_arme, straddle }
}

/// La CASCADE de retrait — idempotente, journalisée. Retourne le résumé
/// (pour la réponse API et les tests).
#[derive(serde::Serialize, Default, Clone, Copy)]
pub struct RetraitRecap {
    pub smc_couples_retires: usize,
    pub kdj_retire: bool,
    pub straddle_retire: bool,
    pub cases_matrice: usize,
    pub positions_ouvertes: i64,
}

pub async fn retirer(db: &Database, id: &str) -> RetraitRecap {
    let impact = impact_retrait(db, id).await;
    let mut recap = RetraitRecap {
        positions_ouvertes: impact.positions_ouvertes,
        ..Default::default()
    };

    // ① SMC : l'asset quitte la map d'armement (ses couples ne génèrent plus).
    if !impact.smc_couples.is_empty() {
        let mut smc = crate::reglages_smc::lire_couples_armes(db).await;
        if smc.remove(id).is_some() {
            if let Ok(v) = serde_json::to_string(&smc) {
                let _ = db.ecrire_config(crate::reglages_smc::CLE_COUPLES_ARMES, &v).await;
                recap.smc_couples_retires = impact.smc_couples.len();
            }
        }
    }

    // ② KDJ : sortie de la liste explicite. Liste vide/absente = « tous » :
    // on MATÉRIALISE l'univers actifs moins l'asset retiré (sinon « tous »
    // le ressusciterait à la première évaluation).
    let kdj = crate::kdj_handlers::assets_armes_kdj(db).await;
    let kdj_absente = kdj.is_none();
    let nouvelle_liste: Vec<String> = match kdj {
        Some(explicite) if !explicite.is_empty() => {
            let mut v: Vec<String> = explicite.into_iter().filter(|a| a != id).collect();
            v.sort();
            v
        }
        _ => db
            .lister_assets()
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|a| a.id)
            .filter(|a| a != id)
            .collect(),
    };
    if impact.kdj_arme || kdj_absente {
        if let Ok(v) = serde_json::to_string(&nouvelle_liste) {
            let _ = db.ecrire_config("kdj_assets_armes", &v).await;
            recap.kdj_retire = impact.kdj_arme;
        }
    }

    // ③ Straddle : sortie du périmètre + ses cases quittent la matrice
    // (mêmes sémantiques que le PUT périmètre, owner 10/10 : le réajouter
    // re-sème/re-arme = nouveau test à zéro).
    if impact.straddle {
        let reste: Vec<String> = crate::runtime_perimetre::lire_perimetre_straddle(db)
            .await
            .into_iter()
            .filter(|a| a != id)
            .collect();
        if let Ok(v) = serde_json::to_string(&reste) {
            let _ = db.ecrire_config("perimetre_straddle", &v).await;
        }
        if let Ok(r) = sqlx::query("DELETE FROM creneaux_evenements WHERE asset = ?")
            .bind(id)
            .execute(db.pool())
            .await
        {
            recap.cases_matrice = r.rows_affected() as usize;
        }
        recap.straddle_retire = true;
    }

    // ④ Caches d'analyse vidés : briefing, patterns de volatilité (radar
    // compris), matrice de réactivité — la propagation est IMMÉDIATE, pas
    // « à l'expiration » (owner 15/10 : la différence entre automatique et
    // presque).
    crate::briefing_ia::vider_cache();
    crate::volatility_handlers::vider_cache_patterns().await;
    crate::evenements_matrice::vider_cache().await;

    tracing::info!(
        "📦 Pipeline : {id} retiré — {} couple(s) SMC, KDJ {}, straddle {}, {} case(s) matrice, {} position(s) ouverte(s) au rattrapage",
        recap.smc_couples_retires,
        if recap.kdj_retire { "retiré" } else { "—" },
        if recap.straddle_retire { "retiré" } else { "—" },
        recap.cases_matrice,
        recap.positions_ouvertes
    );
    recap
}

/// GET /api/assets/{id}/impact-retrait — la lecture AVANT le geste (la
/// modale de confirmation du Pilotage affiche désarmements + positions).
pub async fn get_impact_retrait(
    state: actix_web::web::Data<crate::state::AppState>,
    path: actix_web::web::Path<String>,
) -> impl actix_web::Responder {
    let id = path.into_inner().to_uppercase();
    let impact = impact_retrait(&state.db, &id).await;
    actix_web::HttpResponse::Ok().json(impact)
}

// ── Tests d'intégration (DB mémoire) ────────────────────────────────────────

#[cfg(test)]
mod tests_db {
    use super::*;

    async fn db_test() -> db::Database {
        let db = db::Database::new(":memory:").await.expect("DB mémoire");
        db.run_migrations().await.expect("migrations OK");
        db
    }

    /// La cascade désarme partout, garde les autres assets intacts, et la
    /// liste KDJ « tous » (absente) est matérialisée SANS l'asset retiré.
    #[tokio::test]
    async fn retrait_desarme_partout_et_materialise_kdj() {
        let db = db_test().await;
        // XAUUSD actif + armé partout ; BTC actif + armé SMC.
        for a in ["XAUUSD", "BTC"] {
            sqlx::query("INSERT OR REPLACE INTO assets (id, nom, type, source, actif, cree_le) VALUES (?, ?, 'metal', 'mt5', 1, strftime('%s','now'))")
                .bind(a).bind(a).execute(db.pool()).await.unwrap();
        }
        db.ecrire_config(
            crate::reglages_smc::CLE_COUPLES_ARMES,
            r#"{"XAUUSD":["M5","M15"],"BTC":["M5"]}"#,
        ).await.unwrap();
        db.ecrire_config("perimetre_straddle", r#"["XAUUSD","BTC"]"#).await.unwrap();
        // KDJ : clé ABSENTE = tous armés (legacy).

        let impact = impact_retrait(&db, "XAUUSD").await;
        assert_eq!(impact.positions_ouvertes, 0);
        assert_eq!(impact.smc_couples, vec!["M15", "M5"]);
        assert!(impact.straddle);

        let recap = retirer(&db, "XAUUSD").await;
        assert_eq!(recap.smc_couples_retires, 2);
        assert!(recap.straddle_retire);
        assert_eq!(recap.cases_matrice, 0, "pas de cases seedées en mémoire");

        // SMC : XAUUSD parti, BTC intact.
        let smc = crate::reglages_smc::lire_couples_armes(&db).await;
        assert!(smc.get("XAUUSD").is_none());
        assert_eq!(smc.get("BTC").map(|s| s.len()), Some(1));

        // KDJ matérialisée : contient BTC, pas XAUUSD.
        let kdj = crate::kdj_handlers::assets_armes_kdj(&db).await;
        let kdj = kdj.expect("liste matérialisée");
        assert!(!kdj.contains("XAUUSD"));
        assert!(kdj.contains("BTC"));

        // Straddle : périmètre réduit.
        let per = crate::runtime_perimetre::lire_perimetre_straddle(&db).await;
        assert_eq!(per, vec!["BTC".to_string()]);
    }

    /// Retrait d'un asset armé nulle part : aucun dégât, caches vidés quand
    /// même (l'univers des analyses change).
    #[tokio::test]
    async fn retrait_d_un_asset_non_arme_est_inoffensif() {
        let db = db_test().await;
        sqlx::query("INSERT OR REPLACE INTO assets (id, nom, type, source, actif, cree_le) VALUES ('DOGE', 'DOGE', 'crypto', 'binance', 1, strftime('%s','now'))")
            .execute(db.pool()).await.unwrap();
        let recap = retirer(&db, "DOGE").await;
        assert_eq!(recap.smc_couples_retires, 0);
        assert!(!recap.straddle_retire);
    }
}
