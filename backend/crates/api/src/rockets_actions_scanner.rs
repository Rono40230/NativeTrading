//! Scanner actions US (étape C, 31/08) — Observation silencieuse.
//!
//! Chaîne quotidienne (22h30 UTC + boot), 100 % lecture locale :
//!   bougies_actions → trend template Minervini (pré-screen, qui ENTRE
//!   dans le périmètre) → classement /10 des passants (même moteur que la
//!   crypto, référence QQQ) → journalisation rockets_candidats
//!   (univers='action').
//!
//! OBSERVATION SILENCIEUSE (décision propriétaire) : aucun signal publié,
//! aucune position, aucun ranker — les cassures sont journalisées avec
//! leur score pour observer la qualité des détections avant d'armer.

use actix_web::{web, HttpResponse, Responder};
use std::sync::Arc;

use crate::state::AppState;
use db::Database;
use rockets::classement::{classement_rocket, contexte_marche, BougieD1};
use rockets::trend_template::trend_template;

/// Convertit une ligne bougies_actions en BougieD1 du moteur. Pur (testé).
fn vers_bougies_d1(lignes: &[(i64, f64, f64, f64, f64, f64)]) -> Vec<BougieD1> {
    lignes
        .iter()
        .map(|(ts, o, h, l, c, v)| BougieD1 {
            ts: *ts,
            open: *o,
            high: *h,
            low: *l,
            close: *c,
            volume: *v,
        })
        .collect()
}

/// Un passage du scanner. Retour le compte-rendu (évalués, passants,
/// candidats ≥5, cassures journalisées).
pub async fn scanner_actions(db: &Arc<Database>) -> serde_json::Value {
    // 1. Contexte de marché : QQQ (même source que les actions).
    let qqq = db.bougies_actions("QQQ").await.unwrap_or_default();
    let clotures_qqq: Vec<f64> = qqq.iter().map(|b| b.4).collect();
    let Some(ctx) = contexte_marche(&clotures_qqq) else {
        tracing::warn!("🚀 Scanner actions différé : QQQ pas encore alimenté (course avec le backfill au boot — reprise au prochain passage)");
        return serde_json::json!({
            "erreur": "QQQ insuffisant pour le contexte — scanner différé au prochain passage"
        });
    };

    // 2. Univers évaluable (≥ 261 séances) + nom pour l'entonnoir.
    let tickers = db.tickers_evaluables().await.unwrap_or_default();
    let mut evalues = 0usize;
    let mut passants = 0usize;
    let mut candidats = 0usize;
    let mut cassures = 0usize;

    for ticker in &tickers {
        let lignes = db.bougies_actions(ticker).await.unwrap_or_default();
        if lignes.len() < 261 {
            continue;
        }
        evalues += 1;
        let clotures: Vec<f64> = lignes.iter().map(|b| b.4).collect();
        let Some(tt) = trend_template(&clotures, ctx.perf_marche_4s) else {
            continue;
        };

        // Entonnoir : passants (8) + approchants (≥ 6) — photo quotidienne.
        if tt.conditions >= 6 {
            let nom: String = sqlx::query_scalar("SELECT nom FROM univers_actions WHERE ticker = ?")
                .bind(ticker)
                .fetch_one(db.pool())
                .await
                .unwrap_or_default();
            let _ = db
                .maj_prescreen(ticker, &nom, tt.conditions as i64, 0, tt.perf_4s * 100.0)
                .await;
        }
        if !tt.reussi {
            continue;
        }
        passants += 1;

        // 3. Classement /10 des passants (moteur commun crypto/actions).
        let bougies = vers_bougies_d1(&lignes);
        let r = classement_rocket(ticker, &bougies, &ctx);
        if r.points >= 5 {
            candidats += 1;
            let _ = sqlx::query(
                "INSERT OR REPLACE INTO rockets_candidats
                 (symbole, points, points_base, verdict, pivot, stop, cassure, detail, maj_le, univers)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, strftime('%s','now'), 'action')",
            )
            .bind(&r.symbole)
            .bind(r.points as i64)
            .bind(r.points as i64)
            .bind(format!("{:?}", r.verdict))
            .bind(r.pivot.unwrap_or(0.0))
            .bind(r.stop.unwrap_or(0.0))
            .bind(r.cassure)
            .bind(serde_json::to_string(&r.detail).unwrap_or_default())
            .execute(db.pool())
            .await;
            if r.cassure {
                cassures += 1;
                tracing::info!(
                    "🚀 [Observation] Actions {} : CASSURE au pivot {:.2} — {}/10 (journalisée, rien d'exécuté)",
                    ticker, r.pivot.unwrap_or(0.0), r.points
                );
            }
        }
    }

    // Candidats actions non rafraîchis depuis 2 jours : MARQUÉS éliminés.
    let _ = sqlx::query(
        "UPDATE rockets_candidats SET elimine_le = strftime('%s','now')
         WHERE univers = 'action' AND elimine_le IS NULL
           AND maj_le < strftime('%s','now') - 2*86400",
    )
    .execute(db.pool())
    .await;

    tracing::info!(
        "🚀 Scanner actions : {evalues} évalués, {passants} passants template, {candidats} candidats ≥5, {cassures} cassure(s) — marché QQQ haussier={}",
        ctx.marche_haussier
    );
    serde_json::json!({
        "evalues": evalues,
        "passants_template": passants,
        "candidats_5_plus": candidats,
        "cassures": cassures,
        "marche_qqq_haussier": ctx.marche_haussier,
        "perf_qqq_4s_pct": (ctx.perf_marche_4s * 100.0).round() / 100.0,
    })
}

/// Boucle de fond : un passage au boot, puis quotidien à 22h30 UTC —
/// après la clôture US (21h00 UTC) et la publication EOD Tiingo.
pub async fn boucle_scanner_actions(db: Arc<Database>) {
    tracing::info!("🚀 Scanner actions armé (boot + 22h30 UTC quotidien — Observation)");
    scanner_actions(&db).await;
    loop {
        let maintenant = chrono::Utc::now();
        let prochain = maintenant
            .date_naive()
            .and_hms_opt(22, 30, 0)
            .map(|h| chrono::DateTime::from_naive_utc_and_offset(h, chrono::Utc))
            .unwrap_or(maintenant + chrono::Duration::hours(24));
        let prochain = if prochain <= maintenant {
            prochain + chrono::Duration::days(1)
        } else {
            prochain
        };
        tokio::time::sleep(std::time::Duration::from_secs(
            (prochain - maintenant).num_seconds().max(60) as u64,
        ))
        .await;
        scanner_actions(&db).await;
    }
}

// ── Endpoints ────────────────────────────────────────────────────────────────

/// POST /api/rockets/actions/scan — un passage immédiat (Observation :
/// journalisation seule).
pub async fn post_scan(state: web::Data<AppState>) -> impl Responder {
    HttpResponse::Ok().json(scanner_actions(&state.db).await)
}

/// GET /api/rockets/actions/prescreen?limite=N — entonnoir du pré-screen.
pub async fn get_prescreen(state: web::Data<AppState>, q: web::Query<Limiter>) -> impl Responder {
    let limite = q.limite.unwrap_or(50).clamp(1, 500);
    match state.db.lire_prescreen(limite).await {
        Ok(lignes) => HttpResponse::Ok().json(lignes),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({ "error": e.to_string() })),
    }
}

#[derive(serde::Deserialize)]
pub struct Limiter {
    pub limite: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::vers_bougies_d1;

    #[test]
    fn conversion_bougies_d1_conserve_ohlc_et_volume() {
        let lignes = vec![(1787918400_i64, 100.0, 110.0, 95.0, 105.0, 1_234_567.0)];
        let b = vers_bougies_d1(&lignes);
        assert_eq!(b.len(), 1);
        assert_eq!(b[0].ts, 1787918400);
        assert_eq!(b[0].close, 105.0);
        assert_eq!(b[0].volume, 1_234_567.0);
    }

    #[test]
    fn conversion_vide_reste_vide() {
        assert!(vers_bougies_d1(&[]).is_empty());
    }
}
