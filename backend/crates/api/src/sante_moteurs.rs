//! Étape 4 (roadmap audit 05/10) — télémétrie de silence des moteurs.
//!
//! Leçon KDJ : une stratégie armée peut rester muette des semaines sans
//! que rien ne le signale (moteur endormi par un warm-up jamais franchi).
//! Cet endpoint répond à la question que personne ne posait : « ce moteur
//! est armé — depuis combien de temps n'a-t-il rien émis ? ». Par
//! stratégie : armement, jours de silence, signaux 7 j, seuil d'alerte.
//! Une stratégie armée et muette au-delà de son seuil passe en alerte.

use actix_web::{web, HttpResponse, Responder};
use chrono::Utc;
use sqlx::Row;

use crate::state::AppState;
use db::Database;

/// Seuil d'alerte par stratégie (jours de silence tolérés, propriétaire
/// 05/10). SMC vit au quotidien ; le straddle événementiel tire quelques
/// fois par semaine ; rockets est rare par construction (cassure D1) ;
/// KDJ retient des retournements confirmés.
fn seuil_alerte(strategie: &str) -> f64 {
    match strategie {
        "SMC" => 3.0,
        "straddle" => 2.0,
        "rockets" => 15.0,
        "kdj_halftrend" => 30.0,
        _ => 7.0,
    }
}

/// Dernier signal et volume 7 j d'une stratégie (une requête commune).
async fn stats_signaux(db: &Database, strategie: &str) -> (Option<i64>, i64) {
    let row = sqlx::query(
        "SELECT MAX(cree_le) AS dernier,
                SUM(CASE WHEN cree_le >= ? THEN 1 ELSE 0 END) AS semaine
         FROM signaux WHERE strategie = ?",
    )
    .bind(Utc::now().timestamp() - 7 * 86_400)
    .bind(strategie)
    .fetch_one(db.pool())
    .await
    .ok();
    let dernier = row
        .as_ref()
        .and_then(|r| r.try_get::<Option<i64>, _>("dernier").ok().flatten());
    let semaine = row
        .and_then(|r| r.try_get::<Option<i64>, _>("semaine").ok().flatten())
        .unwrap_or(0);
    (dernier, semaine)
}

/// Ligne de santé d'une stratégie : armement + silence + alerte.
async fn ligne(db: &Database, strategie: &str, armee: bool, detail: String) -> serde_json::Value {
    let (dernier, semaine) = stats_signaux(db, strategie).await;
    let maintenant = Utc::now().timestamp();
    let jours_silence = dernier.map(|d| ((maintenant - d) as f64 / 86_400.0 * 10.0).round() / 10.0);
    let seuil = seuil_alerte(strategie);
    // Armée et muette : au-delà du seuil, ou depuis toujours.
    let alerte = armee
        && jours_silence.is_none_or(|j| j > seuil);
    serde_json::json!({
        "strategie": strategie,
        "armee": armee,
        "detail_armement": detail,
        "dernier_signal_ts": dernier,
        "jours_silence": jours_silence,
        "signaux_7j": semaine,
        "seuil_alerte_jours": seuil,
        "alerte": alerte,
    })
}

/// GET /api/sante/moteurs — le pouls des quatre stratégies.
pub async fn get_sante_moteurs(state: web::Data<AppState>) -> impl Responder {
    let db = &state.db;

    // SMC : couples armés (outil Paramètres › SMC).
    let couples = crate::reglages_smc::lire_couples_armes(db).await;
    let n_smc: usize = couples.values().map(|tfs| tfs.len()).sum();
    let smc = ligne(db, "SMC", n_smc > 0, format!("{n_smc} couple(s) armé(s)")).await;

    // Straddle : créneaux événements armés sur le périmètre.
    let armes: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM creneaux_evenements WHERE arme = 1")
        .fetch_one(db.pool())
        .await
        .unwrap_or(0);
    let straddle = ligne(
        db,
        "straddle",
        armes > 0,
        format!("{armes} créneau(x) événement armé(s)"),
    )
    .await;

    // Rockets : verticale toujours lancée au boot (scanner quotidien + gestion 30 min).
    let rockets = ligne(db, "rockets", true, "scanner quotidien + gestion 30 min".into()).await;

    // KDJ : assets armés (None = tous les H1).
    let kdj = crate::kdj_handlers::assets_armes_kdj(db).await;
    let detail_kdj = match &kdj {
        None => "tous les assets H1".to_string(),
        Some(ensemble) if ensemble.is_empty() => "aucun asset (désarmé)".to_string(),
        Some(ensemble) => format!("{} asset(s) H1", ensemble.len()),
    };
    let kdj_arme = !matches!(&kdj, Some(e) if e.is_empty());
    let kdj = ligne(db, "kdj_halftrend", kdj_arme, detail_kdj).await;

    HttpResponse::Ok().json(serde_json::json!({ "moteurs": [smc, straddle, rockets, kdj] }))
}

/// Sources CONTINUES (alerte à 5 min sans bougie M1) vs ÉPISODIQUES — le
/// comblement Binance n'écrit QUE lors d'un trou (nuits, pannes : décision
/// owner 15/08) : son âge mesure « le temps depuis le dernier trou comblé »,
/// pas une santé — informatif, jamais en alerte.
fn source_episodique(source: &str) -> bool {
    matches!(source, "binance")
}

/// GET /api/sante/sources — fraîcheur des sources de prix (M1).
/// Étape 10 (roadmap audit 05/10) : le retard d'une source continue doit
/// se VOIR (une flux mort devient pastille rouge) ; le comblement reste
/// informatif.
pub async fn get_sante_sources(state: web::Data<AppState>) -> impl Responder {
    let maintenant = Utc::now().timestamp();
    let rows = sqlx::query(
        "SELECT source, COUNT(DISTINCT asset) AS actifs, MAX(timestamp) AS derniere
         FROM bougies
         WHERE timeframe = 'M1' AND source IN ('mt5', 'bybit_ws', 'binance')
         GROUP BY source",
    )
    .fetch_all(state.db.pool())
    .await
    .unwrap_or_default();
    let sources: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            let source: String = r.get("source");
            let derniere: i64 = r.get("derniere");
            let age_min = ((maintenant - derniere) as f64 / 60.0).round();
            let episodique = source_episodique(&source);
            serde_json::json!({
                "source": source,
                "actifs": r.get::<i64, _>("actifs"),
                "age_min": age_min,
                "episodique": episodique,
                "vivante": !episodique && age_min <= 5.0,
            })
        })
        .collect();
    HttpResponse::Ok().json(serde_json::json!({ "sources": sources }))
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn db_test() -> Database {
        let db = Database::new(":memory:").await.expect("DB mémoire");
        db.run_migrations().await.expect("migrations");
        db
    }

    async fn inserer_signal(db: &Database, strategie: &str, age_jours: i64) {
        sqlx::query(
            "INSERT INTO signaux (id, asset, timeframe, direction, score, prix_entree, stop_loss,
             take_profit, strategie, statut, cree_le)
             VALUES (?, 'XAUUSD', 'M1', 'Long', 5, 100.0, 99.0, '[]', ?, 'Fermé', ?)",
        )
        .bind(format!("test-{strategie}-{age_jours}"))
        .bind(strategie)
        .bind(Utc::now().timestamp() - age_jours * 86_400)
        .execute(db.pool())
        .await
        .expect("signal");
    }

    /// Le silence se calcule depuis le dernier signal ; une stratégie armée
    /// sans AUCUN signal est en alerte (le cas KDJ : muette depuis toujours).
    /// Le seuil par défaut des stratégies inconnues est 7 jours.
    #[tokio::test]
    async fn silence_et_alerte() {
        let db = db_test().await;
        inserer_signal(&db, "SMC", 1).await;          // 1 j → pas d'alerte
        inserer_signal(&db, "straddle", 5).await;      // 5 j > seuil 2 → alerte
        // rockets : rien.

        let (dernier, semaine) = stats_signaux(&db, "SMC").await;
        assert!(dernier.is_some());
        assert_eq!(semaine, 1);
        let l = ligne(&db, "SMC", true, "test".into()).await;
        assert_eq!(l["alerte"], false, "1 j de silence < seuil 3");
        assert!((l["jours_silence"].as_f64().unwrap() - 1.0).abs() < 0.2);

        let l = ligne(&db, "straddle", true, "test".into()).await;
        assert_eq!(l["alerte"], true, "5 j > seuil 2");

        let l = ligne(&db, "rockets", true, "test".into()).await;
        assert_eq!(l["alerte"], true, "armée et muette depuis toujours");

        let l = ligne(&db, "kdj_halftrend", false, "désarmé".into()).await;
        assert_eq!(l["alerte"], false, "désarmée : jamais en alerte");
    }
}
