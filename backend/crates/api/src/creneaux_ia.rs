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
//!    armer/ignorer — l'IA n'y touche jamais.
//!
//! Un créneau armé devient une annonce synthétique (`annonces_armees`) :
//! prochaine occurrence hebdomadaire au format `straddle::Annonce` — le
//! moteur M1 existant pose les 2 jambes au timer T-10 s, Observation.

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

    const JOURS: [&str; 7] = ["lundi", "mardi", "mercredi", "jeudi", "vendredi", "samedi", "dimanche"];
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
            let candidat = maintenant + chrono::Duration::days(delta_jours as i64);
            if candidat.weekday().number_from_monday() != jour_cible
                || (candidat.hour(), candidat.minute()) != (heure, 0)
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

/// GET /api/straddle/creneaux-ia — propositions + armés + plafond.
pub async fn lister(state: web::Data<AppState>) -> impl Responder {
    let rows = sqlx::query(
        "SELECT asset, jour, heure, vol_pct, ratio, fiabilite, nb_semaines,
                verdict_ia, conviction, justification, arme
         FROM creneaux_ia ORDER BY arme DESC, vol_pct * ratio * fiabilite DESC",
    )
    .fetch_all(state.db.pool())
    .await
    .unwrap_or_default();
    let armes: usize = rows.iter().filter(|r| r.get::<i64, _>("arme") == 1).count();
    let liste: Vec<serde_json::Value> = rows
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
        }))
        .collect();
    HttpResponse::Ok().json(serde_json::json!({ "creneaux": liste, "armes": armes, "plafond": PLAFOND_ARMES }))
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
        "UPDATE creneaux_ia SET arme = 1, arme_le = strftime('%s','now') WHERE asset = ? AND jour = ? AND heure = ?",
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

/// Boucle de fond : recalcul + évaluation quotidiens (4h du matin Paris,
/// loin des sessions) — les propositions sont fraîches au matin.
pub async fn boucle(db: Arc<Database>) {
    tracing::info!("🤖 Créneaux IA armés (quotidien 4h Paris + boot)");
    recalculer(&db).await;
    evaluer(&db, false).await;
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
    }
}
