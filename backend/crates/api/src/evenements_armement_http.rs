//! Endpoints d'armement événementiel — extraits d'evenements_armement.rs
//! (limite 600 lignes). Poste de commandement (GET armement), bascule
//! individuelle, balayage complet, seuils de la boucle + tests DB.


use actix_web::{web, HttpResponse, Responder};
use chrono::{Datelike, Timelike, Utc};
use sqlx::Row;

use crate::state::AppState;
use crate::evenements::{EVENEMENTS, prochaine_occurrence};
use crate::evenements_armement::lire_seuils;

// ── Endpoints d'armement ─────────────────────────────────────────────────────

/// GET /api/evenements/armement — le poste de commandement : taxonomie +
/// état par (asset × événement) + stats de test + archive des créneaux
/// statistiques (remplacés le 28/09, lignes conservées) + seuils.
pub async fn get_armement(state: web::Data<AppState>) -> impl Responder {
    let perimetre = crate::runtime_perimetre::lire_perimetre_straddle(&state.db).await;
    let rows = sqlx::query(
        "SELECT asset, evenement, arme, arme_le, occurrences, somme_r, verdict_test, conclut_le
         FROM creneaux_evenements",
    )
    .fetch_all(state.db.pool())
    .await
    .unwrap_or_default();

    let maintenant = Utc::now();
    // Premier ts réel à venir par type calendaire (une seule lecture du
    // calendrier, classification en mémoire).
    let annonces_futures = crate::evenements_armement::annonces_high_futures(&state.db, 14 * 24 * 3600).await;
    let mut prochain_par_type: std::collections::HashMap<&'static str, i64> = std::collections::HashMap::new();
    for (ts, titre) in &annonces_futures {
        let ident = crate::evenements::classifie_annonce(titre).unwrap_or("cal_autres");
        prochain_par_type
            .entry(ident)
            .and_modify(|t| *t = (*t).min(*ts))
            .or_insert(*ts);
    }
    let mut evenements: Vec<serde_json::Value> = EVENEMENTS
        .iter()
        .map(|ev| {
            let prochain_ts_type = if ev.calendrier { prochain_par_type.get(ev.ident).copied() } else { None };
            let lignes: Vec<serde_json::Value> = rows
                .iter()
                .filter(|r| r.get::<String, _>("evenement") == ev.ident)
                .map(|r| serde_json::json!({
                    "asset": r.get::<String, _>("asset"),
                    "arme": r.get::<i64, _>("arme") == 1,
                    "occurrences": r.get::<i64, _>("occurrences"),
                    "somme_r": r.get::<f64, _>("somme_r"),
                    "verdict_test": r.try_get::<Option<String>, _>("verdict_test").ok().flatten(),
                    "conclut_le": r.try_get::<Option<i64>, _>("conclut_le").ok().flatten(),
                    "hors_perimetre": !perimetre.iter().any(|p| p == r.get::<String, _>("asset").as_str()),
                }))
                .collect();
            // Types calendrier : la prochaine occurrence est la VRAIE
            // annonce à venir du type (classée par titre) — pas une heure
            // fixe. Lecture une seule fois par requête (lazy ci-dessous).
            let prochaine = if ev.calendrier {
                prochain_ts_type.as_ref().and_then(|ts| {
                    chrono::DateTime::from_timestamp(*ts, 0)
                })
            } else {
                prochaine_occurrence(ev, maintenant)
            };
            let paris = prochaine.map(|p| p.with_timezone(&chrono_tz::Europe::Paris));
            serde_json::json!({
                "ident": ev.ident,
                "nom": ev.nom,
                "detail": ev.detail,
                "fuseau": ev.tz.name(),
                "heure_locale": format!("{:02}:{:02}", ev.heure, ev.minute),
                "jours": ev.jours,
                "calendrier": ev.calendrier,
                "prochaine_ts": prochaine.map(|p| p.timestamp()),
                "prochaine_heure_paris": paris.map(|p| format!("{:02}:{:02} {}/{}", p.hour(), p.minute(), p.day(), p.month())),
                "lignes": lignes,
            })
        })
        .collect();
    evenements.sort_by(|a, b| {
        let cle = |v: &serde_json::Value| {
            v["lignes"]
                .as_array()
                .map(|l| l.iter().map(|x| x["somme_r"].as_f64().unwrap_or(0.0)).sum::<f64>())
                .unwrap_or(0.0)
        };
        cle(b).partial_cmp(&cle(a)).unwrap_or(std::cmp::Ordering::Equal)
    });

    // Archive : les créneaux statistiques remplacés — verdicts récents.
    let total_archive: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM creneaux_ia")
            .fetch_one(state.db.pool())
            .await
            .unwrap_or(0);
    let archive = sqlx::query(
        "SELECT asset, jour, heure, verdict_test, occurrences, somme_r FROM creneaux_ia
         WHERE verdict_test IS NOT NULL ORDER BY conclut_le DESC LIMIT 12",
    )
    .fetch_all(state.db.pool())
    .await
    .unwrap_or_default()
    .iter()
    .map(|r| serde_json::json!({
        "asset": r.get::<String, _>("asset"),
        "jour": r.get::<i64, _>("jour"),
        "heure": r.get::<i64, _>("heure"),
        "verdict_test": r.get::<String, _>("verdict_test"),
        "occurrences": r.get::<i64, _>("occurrences"),
        "somme_r": r.get::<f64, _>("somme_r"),
    }))
    .collect::<Vec<_>>();

    let (seuil_min, plancher_r) = lire_seuils(&state.db).await;
    HttpResponse::Ok().json(serde_json::json!({
        "assets": perimetre,
        "evenements": evenements,
        "seuils": { "min": seuil_min, "plancher_r": plancher_r },
        "archive": { "total": total_archive, "verdicts": archive },
    }))
}

#[derive(serde::Deserialize)]
pub struct BodyBascule {
    pub asset: String,
    pub evenement: String,
}

/// POST /api/evenements/armement/basculer — armement propriétaire d'une
/// case (asset × événement). Armer = nouveau test : compteurs à zéro.
pub async fn basculer(state: web::Data<AppState>, body: web::Json<BodyBascule>) -> impl Responder {
    if !EVENEMENTS.iter().any(|e| e.ident == body.evenement) {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": format!("Événement inconnu : {}", body.evenement)
        }));
    }
    let actuel: Option<i64> = sqlx::query_scalar(
        "SELECT arme FROM creneaux_evenements WHERE asset = ? AND evenement = ?",
    )
    .bind(&body.asset)
    .bind(&body.evenement)
    .fetch_optional(state.db.pool())
    .await
    .ok()
    .flatten();
    let Some(arme) = actuel else {
        return HttpResponse::NotFound().json(serde_json::json!({
            "error": format!("{} n'a pas de créneau pour cet événement (périmètre)", body.asset)
        }));
    };
    let res = if arme == 1 {
        sqlx::query(
            "UPDATE creneaux_evenements SET arme = 0, arme_le = NULL
             WHERE asset = ? AND evenement = ?",
        )
    } else {
        sqlx::query(
            "UPDATE creneaux_evenements SET arme = 1, arme_le = strftime('%s','now'),
                    occurrences = 0, somme_r = 0, verdict_test = NULL, conclut_le = NULL
             WHERE asset = ? AND evenement = ?",
        )
    }
    .bind(&body.asset)
    .bind(&body.evenement)
    .execute(state.db.pool())
    .await;
    match res {
        Ok(r) if r.rows_affected() > 0 => HttpResponse::Ok().json(serde_json::json!({ "ok": true })),
        _ => HttpResponse::InternalServerError().json(serde_json::json!({ "error": "échec mise à jour" })),
    }
}

#[derive(serde::Deserialize)]
pub struct BodyTout {
    pub arme: bool,
}

/// POST /api/evenements/armement/tout — balayage large owner 28/09 :
/// armer (ou désarmer) toutes les cases du périmètre d'un coup. Armer =
/// les compteurs repartent de zéro (nouveau test général).
pub async fn tout_armer(state: web::Data<AppState>, body: web::Json<BodyTout>) -> impl Responder {
    let res = if body.arme {
        sqlx::query(
            "UPDATE creneaux_evenements SET arme = 1, arme_le = strftime('%s','now'),
                    occurrences = 0, somme_r = 0, verdict_test = NULL, conclut_le = NULL",
        )
    } else {
        sqlx::query("UPDATE creneaux_evenements SET arme = 0, arme_le = NULL")
    }
    .execute(state.db.pool())
    .await;
    match res {
        Ok(r) => HttpResponse::Ok().json(serde_json::json!({ "modifiees": r.rows_affected() })),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({ "error": e.to_string() })),
    }
}

#[derive(serde::Deserialize)]
pub struct BodyEvenementLigne {
    pub evenement: String,
    pub arme: bool,
}

/// POST /api/evenements/armement/evenement — armer/désarmer TOUTES les
/// cases d'un événement (ligne de la matrice croisée, owner 10/10).
/// Mêmes sémantiques que basculer : armer = compteurs à zéro (nouveau
/// test), désarmer = compteurs conservés.
pub async fn basculer_evenement(
    state: web::Data<AppState>,
    body: web::Json<BodyEvenementLigne>,
) -> impl Responder {
    if !EVENEMENTS.iter().any(|e| e.ident == body.evenement) {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": format!("Événement inconnu : {}", body.evenement)
        }));
    }
    let res = if body.arme {
        sqlx::query(
            "UPDATE creneaux_evenements SET arme = 1, arme_le = strftime('%s','now'),
                    occurrences = 0, somme_r = 0, verdict_test = NULL, conclut_le = NULL
             WHERE evenement = ?",
        )
        .bind(&body.evenement)
    } else {
        sqlx::query("UPDATE creneaux_evenements SET arme = 0, arme_le = NULL WHERE evenement = ?")
            .bind(&body.evenement)
    }
    .execute(state.db.pool())
    .await;
    match res {
        Ok(r) => HttpResponse::Ok().json(serde_json::json!({ "modifiees": r.rows_affected() })),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({ "error": e.to_string() })),
    }
}

#[derive(serde::Deserialize)]
pub struct BodySeuils {
    pub min: i64,
    pub plancher_r: f64,
}

/// PUT /api/evenements/seuils — réglages propriétaires de la boucle de
/// validation (N tirages minimum, plancher ΣR de réfutation).
pub async fn mettre_seuils(state: web::Data<AppState>, body: web::Json<BodySeuils>) -> impl Responder {
    if !(1..=52).contains(&body.min) || !(-10.0..0.0).contains(&body.plancher_r) {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Seuils hors bornes (min 1-52, plancher −10 à 0 exclus)"
        }));
    }
    let res = state
        .db
        .ecrire_config("creneaux_test_min", &body.min.to_string())
        .await
        .and(state.db.ecrire_config("creneaux_test_plancher_r", &body.plancher_r.to_string()).await);
    match res {
        Ok(()) => HttpResponse::Ok().json(serde_json::json!({ "ok": true })),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({ "error": e.to_string() })),
    }
}

// ── Tests d'intégration (DB mémoire) : semis + annonces + désarmement ───────

#[cfg(test)]
mod tests_db {
    use super::*;
    use db::Database;
    use crate::evenements::EVENEMENTS;
    use crate::evenements_armement::{semer, annonces_evenements, occurrences_suivantes, OCCURRENCES_ANNONCES};

    pub(crate) async fn db_test_util() -> Database {
        let db = Database::new(":memory:").await.expect("DB mémoire");
        db.run_migrations().await.expect("migrations OK");
        db
    }

    async fn db_test() -> Database {
        db_test_util().await
    }

    /// Le semis couvre périmètre × taxonomie, est idempotent, et produit
    /// des annonces synthétiques uniquement pour les créneaux armés.
    #[tokio::test]
    async fn semis_annonces_et_desarmement() {
        let db = db_test().await;
        db.ecrire_config("perimetre_straddle", r#"["XAUUSD","DAX"]"#)
            .await
            .expect("config");

        semer(&db).await;
        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM creneaux_evenements")
            .fetch_one(db.pool())
            .await
            .unwrap();
        assert_eq!(total, (EVENEMENTS.len() * 2) as i64, "2 assets × taxonomie");

        // Idempotent : re-semer ne double rien.
        semer(&db).await;
        let total2: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM creneaux_evenements")
            .fetch_one(db.pool())
            .await
            .unwrap();
        assert_eq!(total2, total);

        // Toutes armées.
        let armes: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM creneaux_evenements WHERE arme = 1",
        )
        .fetch_one(db.pool())
        .await
        .unwrap();
        assert_eq!(armes, total);

        // Annonces XAUUSD : 14 occurrences par événement — SAUF les
        // créneaux « annonces » gated : calendrier vide dans ce test,
        // ils ne tirent aucun jour (owner 29/09).
        let annonces = annonces_evenements(&db, "XAUUSD").await;
        // Fixes NON calendrier uniquement : les types calendaire tirent sur
        // le calendrier (vide ici → zéro) et les slots gated ne tirent pas.
        let fixes = EVENEMENTS
            .iter()
            .filter(|e| !e.gate_calendrier && !e.calendrier)
            .count();
        assert_eq!(annonces.len(), fixes * OCCURRENCES_ANNONCES);
        let maintenant = Utc::now().timestamp();
        assert!(annonces.iter().all(|a| a.ts > maintenant));
        assert!(annonces.iter().all(|a| a.devise == "USD"));
        let annonces_dax = annonces_evenements(&db, "DAX").await;
        assert!(annonces_dax.iter().all(|a| a.devise == "EUR"));

        // Désarmement global : plus aucune annonce.
        sqlx::query("UPDATE creneaux_evenements SET arme = 0")
            .execute(db.pool())
            .await
            .unwrap();
        assert!(annonces_evenements(&db, "XAUUSD").await.is_empty());
    }

    /// Owner 29/09 — « entrer uniquement sur les annonces réelles » : les
    /// créneaux 8:30/10:00 New York ne tirent QUE les jours où le
    /// calendrier porte une annonce USD High ou Medium à la minute du
    /// slot ; les événements de marché (réouverture CME…) tirent toujours.
    #[tokio::test]
    async fn creneaux_annonces_tirent_seulement_si_annonce_reelle() {
        let db = db_test().await;
        db.ecrire_config("perimetre_straddle", r#"["XAUUSD"]"#)
            .await
            .expect("config");
        semer(&db).await;

        // Un jour ouvré futur du slot 8:30 NY ( occurrence réelle de la
        // taxonomie — DST suivi automatiquement).
        let ev830 = EVENEMENTS.iter().find(|e| e.ident == "annonces_us_0830").unwrap();
        let cible = occurrences_suivantes(ev830, Utc::now(), 14)
            .into_iter()
            .nth(3)
            .expect("occurrence");
        // Annonce réelle ce jour-là : USD Medium (le niveau JOLTS/Claims).
        sqlx::query(
            "INSERT INTO calendrier_cache (id, date_heure, devise, titre, impact, fetched_at)
             VALUES ('t1', ?, 'USD', 'JOLTS Job Openings', 'Medium', 0)",
        )
        .bind(chrono::DateTime::from_timestamp(cible, 0).unwrap().to_rfc3339())
        .execute(db.pool())
        .await
        .unwrap();

        let annonces = annonces_evenements(&db, "XAUUSD").await;
        // Le slot 8:30 ne vit QUE le jour calendrier — une seule occurrence.
        let n830 = annonces
            .iter()
            .filter(|a| a.titre.contains("8:30"))
            .count();
        assert_eq!(n830, 1, "8:30 : seulement le jour d'annonce réelle");
        assert!(
            annonces.iter().any(|a| (a.ts - cible).abs() <= 60),
            "l'occurrence du jour calendrier est présente"
        );
        // Le slot 10:00 (gated, aucune annonce au calendrier à 10:00) : silence.
        let n1000 = annonces.iter().filter(|a| a.titre.contains("10:00")).count();
        assert_eq!(n1000, 0, "10:00 sans annonce réelle : aucune passe");
        // La réouverture CME (événement de marché, jamais gated) : complete.
        let ncme = annonces
            .iter()
            .filter(|a| a.titre.contains("Réouverture CME"))
            .count();
        assert_eq!(ncme, OCCURRENCES_ANNONCES, "événement de marché : toutes les occurrences");

        // Retrait de l'annonce → le slot 8:30 redevient muet.
        sqlx::query("DELETE FROM calendrier_cache")
            .execute(db.pool())
            .await
            .unwrap();
        let annonces = annonces_evenements(&db, "XAUUSD").await;
        assert!(
            !annonces.iter().any(|a| a.titre.contains("8:30")),
            "sans annonce au calendrier, le créneau ne tire pas"
        );
    }
}



#[cfg(test)]
mod tests_dedup {
    use super::tests_db::db_test_util;
    use crate::evenements::{classifie_annonce, EVENEMENTS};
    use crate::evenements_armement::{annonces_evenements, occurrences_suivantes};

    /// Déduplication owner 10/10 : une annonce NFP réelle posée PILE sur le
    /// slot gardé 8:30 New York ne doit produire qu'UNE passe (sa ligne 📅
    /// cal_nfp) — le slot fixe garde le silence à cette minute. Avec le type
    /// désarmé en revanche, le slot tire (l'annonce existe à sa minute).
    #[tokio::test]
    async fn nfp_sur_le_slot_830_une_seule_passe() {
        let db = db_test_util().await;
        // Périmètre minimal + semis complet.
        db.ecrire_config("perimetre_straddle", r#"["XAUUSD"]"#).await.unwrap();
        crate::evenements_armement::semer(&db).await;

        // L'annonce NFP réelle = pile la prochaine occurrence du slot 8:30.
        let slot = EVENEMENTS.iter().find(|e| e.ident == "annonces_us_0830").unwrap();
        let ts = occurrences_suivantes(slot, chrono::Utc::now(), 1)[0];
        assert_eq!(classifie_annonce("Non-Farm Employment Change"), Some("cal_nfp"));
        let rfc = chrono::DateTime::from_timestamp(ts, 0).unwrap().to_rfc3339();
        sqlx::query(
            "INSERT INTO calendrier_cache (id, date_heure, devise, titre, impact, precedent, prevision, fetched_at)
             VALUES ('test-nfp', ?, 'USD', 'Non-Farm Employment Change', 'High', '', '', 0)",
        )
        .bind(&rfc)
        .execute(db.pool())
        .await
        .unwrap();

        // cal_nfp ARMÉ (semis) : une seule passe à cette minute, la 📅.
        let annonces = annonces_evenements(&db, "XAUUSD").await;
        let a_ts: Vec<i64> = annonces.iter().map(|a| a.ts).collect();
        assert_eq!(a_ts.iter().filter(|t| **t == ts).count(), 1, "UNE passe, pas deux : {annonces:?}");
        assert!(annonces.iter().any(|a| a.ts == ts && a.titre.starts_with("📅")), "c'est la ligne NFP qui tire");

        // cal_nfp DÉSARMÉ : le slot gardé 8:30 reprend la main (l'annonce
        // existe à sa minute) — toujours exactement une passe.
        sqlx::query("UPDATE creneaux_evenements SET arme = 0 WHERE evenement = 'cal_nfp'")
            .execute(db.pool())
            .await
            .unwrap();
        let annonces = annonces_evenements(&db, "XAUUSD").await;
        let n = annonces.iter().filter(|a| a.ts == ts).count();
        assert_eq!(n, 1, "le slot 8:30 tire en relais : {annonces:?}");
        assert!(annonces.iter().any(|a| a.ts == ts && a.titre.starts_with("🎻") || annonces.iter().any(|a| a.ts == ts && a.titre.starts_with("📯"))));
    }
}
