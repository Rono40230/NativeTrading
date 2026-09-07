//! §16 (07/09) — agenda intelligent straddle.
//!
//! Trois étages, la constitution à chaque ligne :
//! 1. **Calcul statistique** (`recalculer`) : créneaux heure×jour récurrents
//!    sur 24 mois de M15 — vol %, ratio vs moyenne de l'asset, fiabilité
//!    inter-semaines. Déterministe, déterministe, déterministe.
//! 2. **Proposition IA** (`evaluer`) : l'analyste lit les cases avec leur
//!    contexte (sessions, recouvrement annonces tier 1) → ARMER/IGNORER +
//!    conviction + justification. Stocké, jamais exécuté.
//! 3. **Armement propriétaire** : `arme` ne bouge QUE par les endpoints
//!    armer/ignorer — l'IA n'y touche jamais. (Exception §16-b : la boucle
//!    désarme un créneau RÉFUTÉ — règle moteur aux seuils du propriétaire.)
//!
//! Un créneau armé devient une annonce synthétique (`annonces_armees`) :
//! prochaine occurrence hebdomadaire au format `straddle::Annonce` — le
//! moteur M1 existant pose les 2 jambes au timer T-10 s, Observation.
//!
//! §16-b (07/09) — boucle de validation fermée : chaque créneau armé tire
//! chaque semaine, ses passes sont agrégées (`statuer`, quotidien) et au
//! bout de N tirages la boucle statue — VALIDÉ (ΣR > 0, pilier armé),
//! RÉFUTÉ (ΣR ≤ plancher ou 0 gagnant, désarmé), INCERTAIN (prolongé
//! 2 tirages puis tranché). L'UI ne montre plus un catalogue de 75 cartes
//! mais les slots en test + une file dédoublonnée (1 par actif).

use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;

use actix_web::{web, HttpResponse, Responder};
use chrono::{Datelike, Timelike, Utc};
use sqlx::Row;

use crate::state::AppState;
use db::Database;

/// Périmètre straddle (les armés du rail M1).
const ASSETS: &[&str] = &["XAUUSD", "BTC", "DAX", "NAS100", "SP500"];
/// Seuils d'éligibilité d'une case heure×jour (anti-bruit).
const MIN_SEMAINES: i64 = 20;
const MIN_FIABILITE: f64 = 0.40;
const MIN_RATIO: f64 = 1.40;
/// Nombre max de propositions soumises à l'analyste.
const MAX_PROPOSITIONS: usize = 15;
/// Plafond de créneaux armés simultanés (réglage propriétaire).
const PLAFOND_ARMES: usize = 3;

/// Jours ISO 1-7 → libellé (partagé avec la boucle de validation).
pub(crate) const JOURS: [&str; 7] = ["lundi", "mardi", "mercredi", "jeudi", "vendredi", "samedi", "dimanche"];


// ── 1. Calcul statistique ────────────────────────────────────────────────────

/// Recalcule les cases heure×jour éligibles et les écrit (INSERT OR REPLACE
/// — les verdicts/armements des cases disparues sont purgés).
pub async fn recalculer(db: &Database) -> usize {
    let mut ecrites = 0usize;
    for asset in ASSETS {
        if let Ok(n) = recalculer_asset(db, asset).await {
            ecrites += n;
        }
    }
    ecrites
}

async fn recalculer_asset(db: &Database, asset: &str) -> anyhow::Result<usize> {
    let bougies = db
        .obtenir_bougies(&common::Asset::from(asset), &common::Timeframe::M15, 70_000)
        .await
        .unwrap_or_default();
    if bougies.len() < 2_000 {
        return Ok(0); // ~6 mois M15 minimum
    }

    // Médiane des ranges % de l'asset → barre « notable » = 1,5×.
    let mut ranges: Vec<f64> = bougies
        .iter()
        .filter(|b| b.close > 0.0)
        .map(|b| (b.high - b.low) / b.close)
        .collect();
    ranges.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mediane = ranges[ranges.len() / 2].max(1e-12);
    let seuil_notable = 1.5 * mediane;

    // (jour ISO, heure Paris) → (Σ vol %, n, semaines, semaines notables).
    let mut cases: HashMap<(u32, u32), (f64, u64, HashSet<i64>, HashSet<i64>)> = HashMap::new();
    for b in &bougies {
        if b.close <= 0.0 {
            continue;
        }
        let paris = chrono::DateTime::from_timestamp(b.timestamp.timestamp(), 0)
            .unwrap_or_default()
            .with_timezone(&chrono_tz::Europe::Paris);
        let cle_sem = paris.iso_week().year() as i64 * 100 + paris.iso_week().week() as i64;
        let e = cases
            .entry((paris.weekday().number_from_monday(), paris.hour()))
            .or_default();
        let vol = (b.high - b.low) / b.close * 100.0;
        e.0 += vol;
        e.1 += 1;
        e.2.insert(cle_sem);
        if vol >= seuil_notable {
            e.3.insert(cle_sem);
        }
    }

    // Cases éligibles, triées par score (vol × ratio × fiabilité).
    let mut eligibles: Vec<(u32, u32, f64, f64, f64, i64, f64)> = cases
        .into_iter()
        .filter_map(|((jour, heure), (somme, n, semaines, notables))| {
            if n == 0 || semaines.len() < MIN_SEMAINES as usize {
                return None;
            }
            let vol = somme / n as f64;
            let ratio = vol / (mediane * 100.0);
            let fiab = notables.len() as f64 / semaines.len() as f64;
            (ratio >= MIN_RATIO && fiab >= MIN_FIABILITE)
                .then(|| (jour, heure, vol, ratio, fiab, semaines.len() as i64, vol * ratio * fiab))
        })
        .collect();
    eligibles.sort_by(|a, b| b.6.partial_cmp(&a.6).unwrap_or(std::cmp::Ordering::Equal));
    eligibles.truncate(MAX_PROPOSITIONS);

    let maintenant = Utc::now().timestamp();
    for (jour, heure, vol, ratio, fiab, nb, _) in &eligibles {
        sqlx::query(
            "INSERT INTO creneaux_ia (asset, jour, heure, vol_pct, ratio, fiabilite, nb_semaines, maj_le)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(asset, jour, heure) DO UPDATE SET
                vol_pct = excluded.vol_pct, ratio = excluded.ratio,
                fiabilite = excluded.fiabilite, nb_semaines = excluded.nb_semaines,
                maj_le = excluded.maj_le",
        )
        .bind(asset)
        .bind(*jour as i64)
        .bind(*heure as i64)
        .bind(vol)
        .bind(ratio)
        .bind(fiab)
        .bind(nb)
        .bind(maintenant)
        .execute(db.pool())
        .await?;
    }
    // Cases disparues (données retirées) : purgées, armement avec.
    sqlx::query("DELETE FROM creneaux_ia WHERE asset = ? AND maj_le < ?")
        .bind(asset)
        .bind(maintenant)
        .execute(db.pool())
        .await?;
    Ok(eligibles.len())
}

// ── 2. Proposition IA ────────────────────────────────────────────────────────

/// L'analyste note chaque case non encore évaluée (ou toutes si `forcer`).
/// Batch : un prompt, toutes les cases, JSON array en retour.
pub async fn evaluer(db: &Database, forcer: bool) -> usize {
    let rows = sqlx::query(
        "SELECT asset, jour, heure, vol_pct, ratio, fiabilite, nb_semaines
         FROM creneaux_ia",
    )
    .fetch_all(db.pool())
    .await
    .unwrap_or_default();

    let a_evaluer: Vec<_> = rows
        .iter()
        .filter(|r| forcer || r.try_get::<Option<String>, _>("verdict_ia").ok().flatten().is_none())
        .collect();
    if a_evaluer.is_empty() {
        return 0;
    }

    let lignes: Vec<String> = a_evaluer
        .iter()
        .enumerate()
        .map(|(i, r)| {
            format!(
                "{}. {} {} {}h-{}h : vol {:.3}% (×{:.2} vs moyenne), fiabilité {:.0}% sur {} semaines",
                i + 1,
                r.get::<String, _>("asset"),
                JOURS[(r.get::<i64, _>("jour") - 1).clamp(0, 6) as usize],
                r.get::<i64, _>("heure"),
                r.get::<i64, _>("heure") + 1,
                r.get::<f64, _>("vol_pct"),
                r.get::<f64, _>("ratio"),
                r.get::<f64, _>("fiabilite") * 100.0,
                r.get::<i64, _>("nb_semaines"),
            )
        })
        .collect();

    let prompt = format!(
        "{}\n\nCréneaux statistiques à noter (heure Paris) :\n{}\n\nSessions : Londres 8h-9h, NY 14h30-16h Paris (hiver : 13h30-15h). Les annonces tier 1 (NFP, CPI, FOMC…) tombent à 14h30 — un créneau qui vit DEJA à cette heure double une annonce réelle : le dire. Réponds UNIQUEMENT en JSON : [{{\"i\":1,\"verdict\":\"ARMER\"|\"IGNORER\",\"conviction\":0-100,\"justification\":\"1 phrase\"}}]",
        llm::prompt_effectif("creneaux_proposition"),
        lignes.join("\n"),
    );

    let Ok(texte) = llm::ollama::interroger(&prompt).await else {
        tracing::info!("🤖 Créneaux IA : analyste indisponible — réessai au prochain calcul");
        return 0;
    };
    let debut = texte.find('[').unwrap_or(0);
    let fin = texte.rfind(']').map(|i| i + 1).unwrap_or(texte.len());
    let Ok(verdicts) = serde_json::from_str::<Vec<serde_json::Value>>(&texte[debut..fin]) else {
        tracing::warn!("🤖 Créneaux IA : réponse non parsable");
        return 0;
    };

    let mut notes = 0usize;
    for v in &verdicts {
        let Some(i) = v.get("i").and_then(|x| x.as_u64()).map(|x| x as usize) else { continue };
        let Some(idx) = i.checked_sub(1) else { continue };
        let Some(row) = a_evaluer.get(idx) else { continue };
        let verdict = v.get("verdict").and_then(|x| x.as_str()).unwrap_or("IGNORER").to_string();
        let conviction = v.get("conviction").and_then(|x| x.as_i64()).unwrap_or(50);
        let justification = v.get("justification").and_then(|x| x.as_str()).unwrap_or("").to_string();
        let _ = sqlx::query(
            "UPDATE creneaux_ia SET verdict_ia = ?, conviction = ?, justification = ? WHERE asset = ? AND jour = ? AND heure = ?",
        )
        .bind(if verdict.eq_ignore_ascii_case("ARMER") { "ARMER" } else { "IGNORER" })
        .bind(conviction)
        .bind(&justification)
        .bind(row.get::<String, _>("asset"))
        .bind(row.get::<i64, _>("jour"))
        .bind(row.get::<i64, _>("heure"))
        .execute(db.pool())
        .await;
        notes += 1;
    }
    if notes > 0 {
        tracing::info!("🤖 Créneaux IA : {notes} case(s) notée(s) par l'analyste");
    }
    notes
}

// ── 3. Annonces synthétiques ─────────────────────────────────────────────────

/// Prochaines occurrences des créneaux ARMÉS pour un asset, au format
/// `Annonce` du rail M1 — une par semaine à venir, devise filtrée par le
/// runtime comme les tier 1. Le moteur fait le reste sans modification.
pub async fn annonces_armees(db: &Database, asset: &str) -> Vec<straddle::Annonce> {
    let rows = sqlx::query(
        "SELECT jour, heure FROM creneaux_ia WHERE asset = ? AND arme = 1",
    )
    .bind(asset)
    .fetch_all(db.pool())
    .await
    .unwrap_or_default();

    let devise = if asset == "DAX" { "EUR" } else { "USD" };
    let mut out = Vec::new();
    let maintenant = Utc::now().with_timezone(&chrono_tz::Europe::Paris);
    for r in &rows {
        let jour_cible = r.get::<i64, _>("jour").clamp(1, 7) as u32;
        let heure = r.get::<i64, _>("heure").clamp(0, 23) as u32;
        // Les 2 prochaines occurrences hebdomadaires (heure Paris).
        for delta_jours in 0..=14u64 {
            use chrono::Timelike;
            // Candidat à l'heure PILE du créneau (maintenant + N jours garde
            // sinon les minutes courantes — l'heure pile ne matche jamais).
            let jour_glisse = maintenant + chrono::Duration::days(delta_jours as i64);
            let Some(candidat) = jour_glisse
                .with_hour(heure)
                .and_then(|d| d.with_minute(0))
                .and_then(|d| d.with_second(0))
            else { continue };
            if candidat.weekday().number_from_monday() != jour_cible
                || candidat <= maintenant
            {
                continue;
            }
            out.push(straddle::Annonce {
                ts: candidat.timestamp(),
                devise: devise.to_string(),
                titre: format!("🤖 Créneau IA {}", asset),
            });
            break; // une occurrence par créneau (rafraîchie à chaque armement)
        }
    }
    out
}

// ── Endpoints ────────────────────────────────────────────────────────────────

/// GET /api/straddle/creneaux-ia — vue « file d'attente » (§16-b) :
/// `slots` (armés, avec stats de test), `file` (meilleure non testée par
/// actif, verdict IA ARMER uniquement — le dédoublonnage par phénomène),
/// `reserve_liste` (tout le reste, dépliable), `verdicts` (conclus ces 7
/// derniers jours, pour la bannière) et les `seuils` de la boucle.
pub async fn lister(state: web::Data<AppState>) -> impl Responder {
    let rows = sqlx::query(
        "SELECT asset, jour, heure, vol_pct, ratio, fiabilite, nb_semaines,
                verdict_ia, conviction, justification, arme,
                occurrences, somme_r, verdict_test, conclut_le
         FROM creneaux_ia ORDER BY vol_pct * ratio * fiabilite DESC",
    )
    .fetch_all(state.db.pool())
    .await
    .unwrap_or_default();

    let toutes: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| serde_json::json!({
            "asset": r.get::<String, _>("asset"),
            "jour": r.get::<i64, _>("jour"),
            "heure": r.get::<i64, _>("heure"),
            "vol_pct": r.get::<f64, _>("vol_pct"),
            "ratio": r.get::<f64, _>("ratio"),
            "fiabilite": r.get::<f64, _>("fiabilite"),
            "nb_semaines": r.get::<i64, _>("nb_semaines"),
            "verdict_ia": r.try_get::<Option<String>, _>("verdict_ia").ok().flatten(),
            "conviction": r.try_get::<Option<i64>, _>("conviction").ok().flatten(),
            "justification": r.try_get::<Option<String>, _>("justification").ok().flatten(),
            "arme": r.get::<i64, _>("arme") == 1,
            "occurrences": r.get::<i64, _>("occurrences"),
            "somme_r": r.get::<f64, _>("somme_r"),
            "verdict_test": r.try_get::<Option<String>, _>("verdict_test").ok().flatten(),
            "conclut_le": r.try_get::<Option<i64>, _>("conclut_le").ok().flatten(),
        }))
        .collect();

    let cle_case = |v: &serde_json::Value| {
        (
            v["asset"].as_str().unwrap_or("").to_string(),
            v["jour"].as_i64().unwrap_or(0),
            v["heure"].as_i64().unwrap_or(0),
        )
    };
    let est_arme = |v: &serde_json::Value| v["arme"].as_bool().unwrap_or(false);

    // Slots armés (dans l'ordre du score) puis file : meilleure non testée
    // par ACTIF — BTC 16h lundi→vendredi ne propose qu'une carte.
    let mut prises: HashSet<(String, i64, i64)> = HashSet::new();
    let mut slots: Vec<serde_json::Value> = Vec::new();
    let mut file: Vec<serde_json::Value> = Vec::new();
    let mut actifs_vus: HashSet<String> = HashSet::new();
    for v in &toutes {
        if est_arme(v) {
            prises.insert(cle_case(v));
            slots.push(v.clone());
        }
    }
    for v in &toutes {
        if est_arme(v) || v["verdict_test"].is_string() || v["verdict_ia"].as_str() != Some("ARMER") {
            continue;
        }
        if let Some(asset) = v["asset"].as_str() {
            if actifs_vus.insert(asset.to_string()) {
                prises.insert(cle_case(v));
                file.push(v.clone());
            }
        }
    }

    let reserve_liste: Vec<serde_json::Value> = toutes
        .iter()
        .filter(|v| !prises.contains(&cle_case(v)))
        .cloned()
        .collect();
    let il_y_a_7j = Utc::now().timestamp() - 7 * 86_400;
    let verdicts: Vec<serde_json::Value> = toutes
        .iter()
        .filter(|v| {
            v["verdict_test"].as_str().is_some_and(|s| s != "incertain")
                && v["conclut_le"].as_i64().unwrap_or(0) > il_y_a_7j
        })
        .cloned()
        .collect();

    let (seuil_min, plancher_r) = crate::creneaux_test::lire_seuils(&state.db).await;
    HttpResponse::Ok().json(serde_json::json!({
        "slots": slots,
        "file": file,
        "reserve": reserve_liste.len(),
        "reserve_liste": reserve_liste,
        "verdicts": verdicts,
        "armes": slots.len(),
        "plafond": PLAFOND_ARMES,
        "seuils": { "min": seuil_min, "plancher_r": plancher_r },
    }))
}

/// POST /api/straddle/creneaux-ia/calculer — recalcul + évaluation IA
/// (body `{"forcer": bool}` — force à re-noter même les déjà évaluées).
pub async fn calculer_et_evaluer(state: web::Data<AppState>, body: Option<web::Json<serde_json::Value>>) -> impl Responder {
    let forcer = body
        .and_then(|b| b.get("forcer").cloned())
        .and_then(|f| f.as_bool())
        .unwrap_or(false);
    let cases = recalculer(&state.db).await;
    let notes = evaluer(&state.db, forcer).await;
    HttpResponse::Ok().json(serde_json::json!({ "cases": cases, "notees": notes }))
}

/// POST /api/straddle/creneaux-ia/armer — armement propriétaire (plafonné).
pub async fn armer(state: web::Data<AppState>, body: web::Json<BodyCase>) -> impl Responder {
    let armes: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM creneaux_ia WHERE arme = 1")
        .fetch_one(state.db.pool())
        .await
        .unwrap_or(0);
    if armes >= PLAFOND_ARMES as i64 {
        return HttpResponse::Conflict().json(serde_json::json!({
            "error": format!("Plafond de {} créneaux armés atteint — désarme un créneau d'abord", PLAFOND_ARMES)
        }));
    }
    let res = sqlx::query(
        // Armement = nouveau test : les compteurs repartent de zéro.
        "UPDATE creneaux_ia SET arme = 1, arme_le = strftime('%s','now'),
                occurrences = 0, somme_r = 0, verdict_test = NULL, conclut_le = NULL
         WHERE asset = ? AND jour = ? AND heure = ?",
    )
    .bind(&body.asset)
    .bind(body.jour)
    .bind(body.heure)
    .execute(state.db.pool())
    .await;
    match res {
        Ok(r) if r.rows_affected() > 0 => HttpResponse::Ok().json(serde_json::json!({ "ok": true })),
        _ => HttpResponse::NotFound().json(serde_json::json!({ "error": "Créneau inconnu" })),
    }
}

/// POST /api/straddle/creneaux-ia/armer-file — armement propriétaire EN LOT
/// (§16-b) : remplit les slots libres avec les têtes de la file (ARMER,
/// non testées, 1 par actif, ordre du score) en privilégiant les actifs
/// non déjà armés — trois slots = trois contextes indépendants. Le clic
/// reste au propriétaire : l'IA n'arme jamais seule.
pub async fn armer_file(state: web::Data<AppState>) -> impl Responder {
    let armes: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM creneaux_ia WHERE arme = 1")
        .fetch_one(state.db.pool())
        .await
        .unwrap_or(0);
    let libres = PLAFOND_ARMES.saturating_sub(armes as usize);
    if libres == 0 {
        return HttpResponse::Conflict().json(serde_json::json!({
            "error": format!("Aucun slot libre — plafond de {} créneaux armés atteint", PLAFOND_ARMES)
        }));
    }

    let rows = sqlx::query(
        "SELECT asset, jour, heure FROM creneaux_ia
         WHERE arme = 0 AND verdict_test IS NULL AND verdict_ia = 'ARMER'
         ORDER BY vol_pct * ratio * fiabilite DESC",
    )
    .fetch_all(state.db.pool())
    .await
    .unwrap_or_default();
    let assets_deja_armes: HashSet<String> = sqlx::query(
        "SELECT DISTINCT asset FROM creneaux_ia WHERE arme = 1",
    )
    .fetch_all(state.db.pool())
    .await
    .unwrap_or_default()
    .iter()
    .map(|r| r.get::<String, _>("asset"))
    .collect();

    let mut vus: HashSet<String> = HashSet::new();
    let mut armees: Vec<serde_json::Value> = Vec::new();
    for r in &rows {
        if armees.len() >= libres {
            break;
        }
        let asset: String = r.get("asset");
        if assets_deja_armes.contains(&asset) || !vus.insert(asset.clone()) {
            continue;
        }
        let jour = r.get::<i64, _>("jour");
        let heure = r.get::<i64, _>("heure");
        // Armement = nouveau test : compteurs repartis de zéro.
        let _ = sqlx::query(
            "UPDATE creneaux_ia SET arme = 1, arme_le = strftime('%s','now'),
                    occurrences = 0, somme_r = 0, verdict_test = NULL, conclut_le = NULL
             WHERE asset = ? AND jour = ? AND heure = ?",
        )
        .bind(&asset)
        .bind(jour)
        .bind(heure)
        .execute(state.db.pool())
        .await;
        armees.push(serde_json::json!({ "asset": asset, "jour": jour, "heure": heure }));
    }
    HttpResponse::Ok().json(serde_json::json!({ "armees": armees }))
}

/// POST /api/straddle/creneaux-ia/ignorer — désarmement propriétaire.
pub async fn ignorer(state: web::Data<AppState>, body: web::Json<BodyCase>) -> impl Responder {
    let res = sqlx::query(
        "UPDATE creneaux_ia SET arme = 0, arme_le = NULL WHERE asset = ? AND jour = ? AND heure = ?",
    )
    .bind(&body.asset)
    .bind(body.jour)
    .bind(body.heure)
    .execute(state.db.pool())
    .await;
    match res {
        Ok(r) if r.rows_affected() > 0 => HttpResponse::Ok().json(serde_json::json!({ "ok": true })),
        _ => HttpResponse::NotFound().json(serde_json::json!({ "error": "Créneau inconnu" })),
    }
}

#[derive(serde::Deserialize)]
pub struct BodyCase {
    pub asset: String,
    pub jour: i64,
    pub heure: i64,
}

#[derive(serde::Deserialize)]
pub struct BodySeuils {
    pub min: i64,
    pub plancher_r: f64,
}

/// PUT /api/straddle/creneaux-ia/seuils — réglages propriétaires de la
/// boucle de validation (N tirages minimum, plancher ΣR de réfutation).
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

/// Boucle de fond : recalcul + évaluation + verdicts quotidiens (4h du
/// matin Paris, loin des sessions) — les propositions sont fraîches au
/// matin et les verdicts rendus avant l'ouverture.
pub async fn boucle(db: Arc<Database>) {
    tracing::info!("🤖 Créneaux IA armés (quotidien 4h Paris + boot)");
    recalculer(&db).await;
    evaluer(&db, false).await;
    crate::creneaux_test::statuer(&db).await;
    loop {
        let maintenant = Utc::now().with_timezone(&chrono_tz::Europe::Paris);
        let prochain = (maintenant + chrono::Duration::days(1))
            .with_hour(4)
            .and_then(|d| d.with_minute(0))
            .unwrap_or(maintenant + chrono::Duration::hours(24));
        tokio::time::sleep(std::time::Duration::from_secs(
            (prochain - maintenant).num_seconds().max(60) as u64,
        ))
        .await;
        recalculer(&db).await;
        evaluer(&db, false).await;
        crate::creneaux_test::statuer(&db).await;
    }
}
