//! Catalogue des créneaux fixes — source DE DONNÉES depuis le 15/10
//! (migration 0122) : les événements structurels vivent dans la table
//! `evenements_catalogue` et deviennent éditables par le propriétaire
//! (modale « Gérer les créneaux fixes »). Les 9 types 📅 restent en code
//! (`evenements::types_calendrier`) — leur heure vient de l'annonce réelle.
//!
//! Lecture : cache 30 s + version — chaque CRUD incrémente VERSION, qui
//! entre dans l'EMPREINTE straddle du hot-reload (≤ 60 s après une édition,
//! les moteurs M1 se réinscrivent avec les nouvelles annonces, garde « passe
//! en fenêtre active différée » conservée).


use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Instant;

use actix_web::{web, HttpResponse, Responder};
use chrono::{Datelike, Timelike, Utc};
use sqlx::Row;

use crate::evenements::{Evenement, prochaine_occurrence};
use crate::state::AppState;

/// Version du catalogue — bumpée à chaque CRUD ; l'empreinte straddle du
/// hot-reload l'embarque (une édition = réinscription des moteurs ≤ 60 s).
pub static VERSION: AtomicU64 = AtomicU64::new(0);

type Cache = (Instant, Arc<Vec<Evenement>>);
static CACHE: OnceLock<Mutex<Option<Cache>>> = OnceLock::new();
const TTL: std::time::Duration = std::time::Duration::from_secs(30);

/// Le catalogue complet : structurels de la base (actifs ET en pause —
/// filtrer sur `actif` côté usage) + les 9 types 📅 (toujours actifs).
/// Lecture cachée 30 s — invalider() force le rechargement.
pub(crate) async fn lire(db: &db::Database) -> Arc<Vec<Evenement>> {
    let garde = CACHE.get_or_init(|| Mutex::new(None));
    if let Ok(c) = garde.lock() {
        if let Some((t, v)) = c.as_ref() {
            if t.elapsed() < TTL {
                return v.clone();
            }
        }
    }
    let mut out = structurels_db(db).await;
    out.extend(crate::evenements::types_calendrier());
    let arc = Arc::new(out);
    if let Ok(mut c) = garde.lock() {
        *c = Some((Instant::now(), arc.clone()));
    }
    arc
}

/// Les structurels de la table — une ligne invalide (tz inconnue, jours
/// illisibles) est SKIPPÉE avec un log : elle reste visible/corrigeable
/// dans la modale, mais ne tire aucune annonce.
async fn structurels_db(db: &db::Database) -> Vec<Evenement> {
    let rows = sqlx::query(
        "SELECT ident, nom, detail, tz, heure, minute, jours, actif
         FROM evenements_catalogue ORDER BY ordre, ident",
    )
    .fetch_all(db.pool())
    .await
    .unwrap_or_default();
    rows.iter()
        .filter_map(|r| {
            let tz: String = r.try_get("tz").ok()?;
            let jours_txt: String = r.try_get("jours").ok()?;
            let jours: Vec<u32> = serde_json::from_str(&jours_txt).ok()?;
            if jours.is_empty() || !jours.iter().all(|j| (1..=7).contains(j)) {
                return None;
            }
            Some(Evenement {
                ident: r.try_get("ident").ok()?,
                nom: r.try_get("nom").ok()?,
                detail: r.try_get("detail").ok()?,
                tz: tz.parse::<chrono_tz::Tz>().ok()?,
                heure: r.try_get::<i64, _>("heure").ok()? as u32,
                minute: r.try_get::<i64, _>("minute").ok()? as u32,
                jours,
                calendrier: false,
                actif: r.try_get::<i64, _>("actif").ok()? == 1,
            })
        })
        .collect()
}

/// Force le rechargement + bump de version (empreinte hot-reload).
fn invalider() {
    VERSION.fetch_add(1, Ordering::Relaxed);
    if let Ok(mut c) = CACHE.get_or_init(|| Mutex::new(None)).lock() {
        *c = None;
    }
}

// ── Endpoints CRUD (owner 15/10) ─────────────────────────────────────────────

fn slug(nom: &str) -> String {
    let mut s: String = nom
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect();
    while s.contains("__") {
        s = s.replace("__", "_");
    }
    let s = s.trim_matches('_').to_string();
    if s.len() < 2 {
        "creneau".to_string()
    } else {
        s
    }
}

/// Vérifie qu'un ident est libre (table + types 📅 réservés).
async fn ident_libre(db: &db::Database, ident: &str) -> bool {
    if crate::evenements::CAL_TYPES.iter().any(|(i, _, _)| *i == ident) {
        return false;
    }
    sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM evenements_catalogue WHERE ident = ?")
        .bind(ident)
        .fetch_one(db.pool())
        .await
        .map(|n| n == 0)
        .unwrap_or(false)
}

/// Valide tz + jours d'un corps de requête → (tz, jours) ou message d'erreur.
fn valider_tz_jours(tz: &str, jours: &[u64]) -> Result<(chrono_tz::Tz, Vec<u32>), String> {
    let tz = tz
        .parse::<chrono_tz::Tz>()
        .map_err(|_| format!("Fuseau inconnu : {tz} (ex. Europe/Paris, America/New_York)"))?;
    let mut js: Vec<u32> = jours.iter().map(|&j| j as u32).collect();
    js.sort_unstable();
    js.dedup();
    if js.is_empty() || !js.iter().all(|j| (1..=7).contains(j)) {
        return Err("Jours invalides : cocher au moins un jour (lundi=1..dimanche=7)".into());
    }
    Ok((tz, js))
}

#[derive(serde::Deserialize)]
pub struct BodyCreation {
    pub nom: String,
    pub tz: String,
    pub heure: i64,
    pub minute: i64,
    pub jours: Vec<u64>,
    pub detail: Option<String>,
}

/// POST /api/evenements/catalogue — créer un créneau fixe.
/// DOCTRINE : les cellules naissent DÉSARMÉES (arme=0, compteurs vierges)
/// pour tout le périmètre — visibles et cliquables dans la matrice ;
/// l'armement reste au propriétaire seul (jamais armé à la création).
pub async fn creer(state: web::Data<AppState>, body: web::Json<BodyCreation>) -> impl Responder {
    let nom = body.nom.trim();
    if nom.len() < 2 || nom.len() > 80 {
        return HttpResponse::BadRequest().json(serde_json::json!({ "error": "Nom : 2 à 80 caractères" }));
    }
    if !(0..=23).contains(&body.heure) || !(0..=59).contains(&body.minute) {
        return HttpResponse::BadRequest().json(serde_json::json!({ "error": "Horaire invalide (0-23 h, 0-59 min)" }));
    }
    let (tz, jours) = match valider_tz_jours(&body.tz, &body.jours) {
        Ok(v) => v,
        Err(e) => return HttpResponse::BadRequest().json(serde_json::json!({ "error": e })),
    };
    let mut ident = slug(nom);
    if !ident_libre(&state.db, &ident).await {
        let mut k = 2;
        while !ident_libre(&state.db, &format!("{ident}_{k}")).await {
            k += 1;
        }
        ident = format!("{ident}_{k}");
    }
    let jours_txt = serde_json::to_string(&jours).unwrap_or_else(|_| "[1,2,3,4,5]".into());
    let res = sqlx::query(
        "INSERT INTO evenements_catalogue (ident, nom, detail, tz, heure, minute, jours, actif, ordre, modifie_le)
         VALUES (?, ?, ?, ?, ?, ?, ?, 1,
                 (SELECT COALESCE(MAX(ordre), 0) + 1 FROM evenements_catalogue),
                 strftime('%s','now'))",
    )
    .bind(&ident)
    .bind(nom)
    .bind(body.detail.as_deref().unwrap_or(""))
    .bind(tz.name())
    .bind(body.heure)
    .bind(body.minute)
    .bind(&jours_txt)
    .execute(state.db.pool())
    .await;
    match res {
        Ok(_) => {
            // Cellules désarmées × périmètre (correctif owner 15/10 soir :
            // une cellule absente serait impossible à armer dans la matrice).
            let perimetre = crate::runtime_perimetre::lire_perimetre_straddle(&state.db).await;
            let mut cellules = 0u32;
            for asset in &perimetre {
                if let Ok(r) = sqlx::query(
                    "INSERT OR IGNORE INTO creneaux_evenements (asset, evenement, arme) VALUES (?, ?, 0)",
                )
                .bind(asset)
                .bind(&ident)
                .execute(state.db.pool())
                .await
                {
                    cellules += r.rows_affected() as u32;
                }
            }
            invalider();
            tracing::info!(
                "📯 Créneau fixe créé : {ident} « {nom} » ({} {:02}:{:02}) — {cellules} cellule(s) désarmée(s)",
                tz.name(), body.heure, body.minute
            );
            HttpResponse::Ok().json(serde_json::json!({ "ok": true, "ident": ident }))
        }
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({ "error": e.to_string() })),
    }
}

#[derive(serde::Deserialize)]
pub struct BodyModification {
    pub nom: Option<String>,
    pub detail: Option<String>,
    pub tz: Option<String>,
    pub heure: Option<i64>,
    pub minute: Option<i64>,
    pub jours: Option<Vec<u64>>,
    pub actif: Option<bool>,
    /// Propriétaire seul : remise à zéro des compteurs quand l'horaire (ou
    /// les jours/fuseau) change — sinon la case mélange deux populations.
    pub raz_compteurs: Option<bool>,
}

/// PUT /api/evenements/catalogue/{ident} — modifier un créneau.
/// Changement d'horaire/jours/fuseau SANS raz : les compteurs restent tels
/// quels (l'UI avertit ; choix propriétaire explicite).
pub async fn modifier(
    state: web::Data<AppState>,
    path: web::Path<String>,
    body: web::Json<BodyModification>,
) -> impl Responder {
    let ident = path.into_inner();
    let actuel = sqlx::query(
        "SELECT ident, nom, detail, tz, heure, minute, jours, actif FROM evenements_catalogue WHERE ident = ?",
    )
    .bind(&ident)
    .fetch_optional(state.db.pool())
    .await
    .ok()
    .flatten();
    let Some(actuel) = actuel else {
        return HttpResponse::NotFound().json(serde_json::json!({ "error": format!("Créneau inconnu : {ident}") }));
    };

    let tz_txt: String = body.tz.clone().unwrap_or_else(|| actuel.get("tz"));
    let heure = body.heure.unwrap_or_else(|| actuel.get::<i64, _>("heure"));
    let minute = body.minute.unwrap_or_else(|| actuel.get::<i64, _>("minute"));
    let jours_brut = body.jours.clone().unwrap_or_else(|| {
        serde_json::from_str::<Vec<u64>>(&actuel.get::<String, _>("jours")).unwrap_or_default()
    });
    if !(0..=23).contains(&heure) || !(0..=59).contains(&minute) {
        return HttpResponse::BadRequest().json(serde_json::json!({ "error": "Horaire invalide (0-23 h, 0-59 min)" }));
    }
    let (tz, jours) = match valider_tz_jours(&tz_txt, &jours_brut) {
        Ok(v) => v,
        Err(e) => return HttpResponse::BadRequest().json(serde_json::json!({ "error": e })),
    };
    let nom = body.nom.clone().unwrap_or_else(|| actuel.get("nom"));
    if nom.trim().len() < 2 || nom.trim().len() > 80 {
        return HttpResponse::BadRequest().json(serde_json::json!({ "error": "Nom : 2 à 80 caractères" }));
    }

    // L'horaire de tirage change-t-il ? (fuseau, heure, minute ou jours)
    let ancien_jours: Vec<u64> =
        serde_json::from_str(&actuel.get::<String, _>("jours")).unwrap_or_default();
    let horaire_change = tz_txt != actuel.get::<String, _>("tz")
        || heure != actuel.get::<i64, _>("heure")
        || minute != actuel.get::<i64, _>("minute")
        || jours_brut != ancien_jours;

    let jours_txt = serde_json::to_string(&jours).unwrap_or_else(|_| "[1,2,3,4,5]".into());
    let res = sqlx::query(
        "UPDATE evenements_catalogue
         SET nom = ?, detail = ?, tz = ?, heure = ?, minute = ?, jours = ?, actif = ?, modifie_le = strftime('%s','now')
         WHERE ident = ?",
    )
    .bind(nom.trim())
    .bind(body.detail.as_deref().unwrap_or_else(|| actuel.get("detail")))
    .bind(tz.name())
    .bind(heure)
    .bind(minute)
    .bind(&jours_txt)
    .bind(i64::from(body.actif.unwrap_or(actuel.get::<i64, _>("actif") == 1)))
    .bind(&ident)
    .execute(state.db.pool())
    .await;

    match res {
        Ok(_) => {
            // Raz des compteurs (nouveau test) si demandé sur changement
            // d'horaire — mêmes sémantiques que l'armement : arme_le = now.
            if horaire_change && body.raz_compteurs.unwrap_or(false) {
                let _ = sqlx::query(
                    "UPDATE creneaux_evenements
                     SET occurrences = 0, somme_r = 0, verdict_test = NULL, conclut_le = NULL,
                         arme_le = CASE WHEN arme = 1 THEN strftime('%s','now') ELSE arme_le END
                     WHERE evenement = ?",
                )
                .bind(&ident)
                .execute(state.db.pool())
                .await;
                tracing::info!("📯 Créneau {ident} modifié (horaire) — compteurs remis à zéro");
            }
            invalider();
            tracing::info!("📯 Créneau fixe modifié : {ident}");
            HttpResponse::Ok().json(serde_json::json!({ "ok": true }))
        }
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({ "error": e.to_string() })),
    }
}

/// DELETE /api/evenements/catalogue/{ident} — supprimer le créneau ET ses
/// cases de matrice (comme le retrait d'asset supprime sa colonne, écart
/// 14/10). Le vécu garde ses trades ; la ligne disparaît des stats par
/// source.
pub async fn supprimer(state: web::Data<AppState>, path: web::Path<String>) -> impl Responder {
    let ident = path.into_inner();
    if crate::evenements::CAL_TYPES.iter().any(|(i, _, _)| *i == ident) {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Les types 📅 (annonces réelles) ne sont pas supprimables — doctrine 14/10"
        }));
    }
    let _ = sqlx::query("DELETE FROM creneaux_evenements WHERE evenement = ?")
        .bind(&ident)
        .execute(state.db.pool())
        .await;
    let res = sqlx::query("DELETE FROM evenements_catalogue WHERE ident = ?")
        .bind(&ident)
        .execute(state.db.pool())
        .await;
    match res {
        Ok(r) if r.rows_affected() > 0 => {
            invalider();
            tracing::info!("📯 Créneau fixe supprimé : {ident} (+ cases de matrice)");
            HttpResponse::Ok().json(serde_json::json!({ "ok": true }))
        }
        Ok(_) => HttpResponse::NotFound().json(serde_json::json!({ "error": format!("Créneau inconnu : {ident}") })),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({ "error": e.to_string() })),
    }
}

/// GET /api/evenements/catalogue — la liste de la modale « Gérer les
/// créneaux fixes » : structurels (avec état + prochaine occurrence) ;
/// les types 📅 sont rappelés en pied de liste (non éditables).
pub async fn lister(state: web::Data<AppState>) -> impl Responder {
    let catalogue = lire(&state.db).await;
    let maintenant = Utc::now();
    let structurels: Vec<serde_json::Value> = catalogue
        .iter()
        .filter(|e| !e.calendrier)
        .map(|ev| {
            let prochaine = prochaine_occurrence(ev, maintenant)
                .map(|p| p.with_timezone(&chrono_tz::Europe::Paris));
            // Compteurs de la ligne (toutes cases, périmètre confondu).
            serde_json::json!({
                "ident": ev.ident,
                "nom": ev.nom,
                "detail": ev.detail,
                "tz": ev.tz.name(),
                "heure": ev.heure,
                "minute": ev.minute,
                "jours": ev.jours,
                "actif": ev.actif,
                "prochaine_heure_paris": prochaine
                    .as_ref()
                    .map(|p| format!("{:02}:{:02} {}", p.hour(), p.minute(),
                        ["lun", "mar", "mer", "jeu", "ven", "sam", "dim"]
                            [(p.weekday().number_from_monday() - 1) as usize])),
            })
        })
        .collect();
    let cal: Vec<serde_json::Value> = catalogue
        .iter()
        .filter(|e| e.calendrier)
        .map(|ev| serde_json::json!({ "ident": ev.ident, "nom": ev.nom }))
        .collect();
    HttpResponse::Ok().json(serde_json::json!({
        "structurels": structurels,
        "types_calendrier": cal,
    }))
}

// ── Tests d'intégration (DB mémoire) ────────────────────────────────────────

#[cfg(test)]
mod tests_db {
    use super::*;

    async fn db_test() -> db::Database {
        let db = db::Database::new(":memory:").await.expect("DB mémoire");
        db.run_migrations().await.expect("migrations OK");
        // Le cache catalogue est PROCESS-GLOBAL (une seule base en prod) :
        // chaque test invalide pour ne pas lire le catalogue d'un autre.
        invalider();
        db
    }

    /// Le seed : 9 structurels (8 d'origine + réouverture or) + 9 types 📅.
    #[tokio::test]
    async fn catalogue_seed_9_structurels_plus_9_cal() {
        let db = db_test().await;
        let c = lire(&db).await;
        assert_eq!(c.iter().filter(|e| !e.calendrier).count(), 9, "8 + or_reouverture");
        assert_eq!(c.iter().filter(|e| e.calendrier).count(), 9);
        let or = c.iter().find(|e| e.ident == "or_reouverture").expect("or seedé");
        assert_eq!(or.tz.name(), "Europe/Paris");
        assert_eq!((or.heure, or.minute), (0, 0));
        assert_eq!(or.jours, vec![1], "lundi seul");
    }

    /// Création = cellules DÉSARMÉES × périmètre (correctif 15/10 soir :
    /// une cellule absente serait impossible à armer dans la matrice).
    #[tokio::test]
    async fn creation_cellules_desarmees() {
        let db = db_test().await;
        db.ecrire_config("perimetre_straddle", r#"["XAUUSD","BTC"]"#).await.unwrap();
        sqlx::query(
            "INSERT INTO evenements_catalogue (ident, nom, detail, tz, heure, minute, jours, actif, ordre)
             VALUES ('test_x', 'Test X', '', 'Europe/Paris', 3, 0, '[1,2]', 1, 99)",
        ).execute(db.pool()).await.unwrap();
        // Même semis que le handler creer : désarmé × périmètre.
        for asset in ["XAUUSD", "BTC"] {
            sqlx::query(
                "INSERT OR IGNORE INTO creneaux_evenements (asset, evenement, arme) VALUES (?, 'test_x', 0)",
            ).bind(asset).execute(db.pool()).await.unwrap();
        }
        invalider();
        let cellules = sqlx::query(
            "SELECT asset, arme, occurrences FROM creneaux_evenements WHERE evenement = 'test_x'",
        ).fetch_all(db.pool()).await.unwrap();
        assert_eq!(cellules.len(), 2, "une cellule par asset du périmètre");
        assert!(cellules.iter().all(|r| r.get::<i64, _>("arme") == 0), "désarmées");
        assert!(cellules.iter().all(|r| r.get::<i64, _>("occurrences") == 0), "compteurs vierges");
        // La migration 0123 fait le même rattrapage pour l'existant :
        // un couple (périmètre × catalogue) manquant naît désarmé.
        sqlx::query(
            "INSERT OR IGNORE INTO creneaux_evenements (asset, evenement, arme)
             SELECT je.value, c.ident, 0 FROM evenements_catalogue c, configuration cfg, json_each(cfg.valeur) je
             WHERE cfg.cle = 'perimetre_straddle'",
        ).execute(db.pool()).await.unwrap();
        let xau: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM creneaux_evenements WHERE asset = 'XAUUSD' AND arme = 0 AND occurrences = 0",
        ).fetch_one(db.pool()).await.unwrap();
        assert!(xau >= 1, "rattrapage 0123 : les couples manquants naissent désarmés");
    }

    /// Suppression = cases de matrice supprimées avec le créneau.
    #[tokio::test]
    async fn suppression_emporte_les_cases() {
        let db = db_test().await;
        sqlx::query(
            "INSERT INTO creneaux_evenements (asset, evenement, arme, occurrences, somme_r)
             VALUES ('XAUUSD', 'asie_ouverture', 1, 7, -1.25)",
        ).execute(db.pool()).await.unwrap();
        // Même ordre que le handler supprimer : les cases PUIS le créneau.
        sqlx::query("DELETE FROM creneaux_evenements WHERE evenement = 'asie_ouverture'")
            .execute(db.pool()).await.unwrap();
        sqlx::query("DELETE FROM evenements_catalogue WHERE ident = 'asie_ouverture'")
            .execute(db.pool()).await.unwrap();
        invalider();
        let cases: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM creneaux_evenements WHERE evenement = 'asie_ouverture'",
        ).fetch_one(db.pool()).await.unwrap();
        assert_eq!(cases, 0);
        assert!(!lire(&db).await.iter().any(|e| e.ident == "asie_ouverture"));
    }

    /// Modification d'horaire + raz : compteurs à zéro, arme_le rafraîchi.
    #[tokio::test]
    async fn modification_horaire_raz_compteurs() {
        let db = db_test().await;
        sqlx::query(
            "INSERT INTO creneaux_evenements (asset, evenement, arme, arme_le, occurrences, somme_r, verdict_test)
             VALUES ('XAUUSD', 'lbma_am', 1, 1000, 5, 3.5, 'incertain')",
        ).execute(db.pool()).await.unwrap();
        sqlx::query("UPDATE evenements_catalogue SET heure = 12 WHERE ident = 'lbma_am'")
            .execute(db.pool()).await.unwrap();
        sqlx::query(
            "UPDATE creneaux_evenements SET occurrences = 0, somme_r = 0, verdict_test = NULL, conclut_le = NULL,
                    arme_le = CASE WHEN arme = 1 THEN strftime('%s','now') ELSE arme_le END
             WHERE evenement = 'lbma_am'",
        ).execute(db.pool()).await.unwrap();
        let r = sqlx::query("SELECT occurrences, somme_r, verdict_test, arme_le FROM creneaux_evenements
                             WHERE evenement = 'lbma_am'")
            .fetch_one(db.pool()).await.unwrap();
        assert_eq!(r.get::<i64, _>("occurrences"), 0);
        assert_eq!(r.get::<f64, _>("somme_r"), 0.0);
        assert!(r.try_get::<Option<String>, _>("verdict_test").ok().flatten().is_none());
        assert!(r.get::<Option<i64>, _>("arme_le").unwrap_or(0) > 1000);
    }

    /// Les idents cal_* sont réservés : jamais de créneau fixe à ce nom.
    #[test]
    fn cal_reserves() {
        assert!(!crate::evenements::CAL_TYPES.iter().any(|(i, _, _)| *i == "mon_nouveau"));
        assert!(crate::evenements::CAL_TYPES.iter().any(|(i, _, _)| *i == "cal_cpi"));
    }

    /// Le slug déduit l'ident du nom (minuscules, tirets bas).
    #[test]
    fn slug_du_nom() {
        assert_eq!(slug("Réouverture or (pause Axi)"), "r_ouverture_or_pause_axi");
        assert_eq!(slug("Fix  London  PM"), "fix_london_pm");
    }
}
