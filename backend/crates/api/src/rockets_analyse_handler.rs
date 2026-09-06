//! Bouton 📊 Analyse de la page Rockets — V2 recâblée le 05/09 (§10) : les
//! métriques viennent des signaux officiels de la verticale (table
//! `signaux`) et du vivier du scanner ; la réponse est persistée dans
//! `rockets_analyses_llm`. La lecture SQL vit ici — la crate llm ne fait
//! que l'analyse (séparation des rôles).

use actix_web::{web, HttpResponse, Responder};
use sqlx::Row;

use crate::state::AppState;
use db::rockets;

const MIN_TRADES_ANALYSE: i64 = 5;
const LIMITE_TRADES: i64 = 30;

/// POST /api/rockets/analyse-llm — déclenche une analyse stratégique immédiate.
pub async fn lancer_analyse(state: web::Data<AppState>) -> impl Responder {
    let pool = state.db.pool();

    let nb = nb_trades_clotures(pool).await;
    if nb < MIN_TRADES_ANALYSE {
        return HttpResponse::Conflict().json(serde_json::json!({
            "error": format!("Pas assez de trades clôturés ({nb} < {MIN_TRADES_ANALYSE}) — la verticale est jeune")
        }));
    }

    let contexte = formater_contexte(pool, LIMITE_TRADES).await;
    let reponse = match llm::rockets_analyse::analyser_strategie(&contexte).await {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("Analyse LLM rockets: {e}");
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": e.to_string() }));
        }
    };

    let recommandations_json = serde_json::to_string(&reponse.recommandations)
        .unwrap_or_else(|_| "[]".to_string());
    if let Err(e) = rockets::sauvegarder_analyse(
        pool,
        nb,
        &reponse.synthese,
        reponse.meilleur_setup.as_deref(),
        reponse.pire_setup.as_deref(),
        &recommandations_json,
    )
    .await
    {
        tracing::error!("Sauvegarde analyse rockets: {e}");
        return HttpResponse::InternalServerError()
            .json(serde_json::json!({ "error": e.to_string() }));
    }
    tracing::info!("Analyse LLM rockets sauvegardée ({nb} trades clôturés)");

    match rockets::derniere_analyse(pool).await {
        Ok(Some(a)) => HttpResponse::Ok().json(a),
        Ok(None) => HttpResponse::InternalServerError().json(serde_json::json!({ "error": "analyse introuvable" })),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({ "error": e.to_string() })),
    }
}

/// GET /api/rockets/analyse-llm — retourne la dernière analyse stockée.
pub async fn get_derniere_analyse(state: web::Data<AppState>) -> impl Responder {
    let pool = state.db.pool();
    match rockets::derniere_analyse(pool).await {
        Ok(Some(a)) => HttpResponse::Ok().json(a),
        Ok(None) => HttpResponse::NoContent().finish(),
        Err(e) => {
            tracing::error!("Lecture analyse LLM: {e}");
            HttpResponse::InternalServerError().json(serde_json::json!({ "error": e.to_string() }))
        }
    }
}

// ── Construction du contexte (vraies données) ────────────────────────────────

async fn nb_trades_clotures(pool: &sqlx::SqlitePool) -> i64 {
    sqlx::query_scalar(
        "SELECT COUNT(*) FROM signaux
         WHERE strategie = 'rockets' AND statut = 'Fermé' AND verdict IS NOT NULL",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0)
}

struct TradeFerme {
    asset: String,
    verdict: String,
    r: f64,
}

/// Univers d'un symbole : les cryptos du scanner Binance finissent en USDT.
fn univers_du_symbole(s: &str) -> &'static str {
    if s.ends_with("USDT") { "crypto" } else { "action" }
}

/// Contexte texte pour l'analyste : réglages, trades clôturés, vivier de
/// candidats (classement par palier × univers, suivis vs éliminés) et
/// positions ouvertes.
async fn formater_contexte(pool: &sqlx::SqlitePool, limite: i64) -> String {
    let params = sqlx::query(
        "SELECT profil, plafond_position_pct, trailing_pct, conviction_min FROM rockets_params WHERE id = 1",
    )
    .fetch_optional(pool)
    .await
    .unwrap_or(None);

    let mut ctx = String::from("=== RÉGLAGES ACTUELS ===\n");
    if let Some(p) = &params {
        ctx.push_str(&format!(
            "profil={:?} | plafond_position={:.1}% | trailing={:.1}% | conviction_min={}\n",
            p.try_get::<String, _>("profil").unwrap_or_else(|_| "Neutre".into()),
            p.try_get::<f64, _>("plafond_position_pct").unwrap_or(5.0),
            p.try_get::<f64, _>("trailing_pct").unwrap_or(5.0),
            p.try_get::<i64, _>("conviction_min").unwrap_or(40),
        ));
    } else {
        ctx.push_str("réglages par défaut (profil Neutre, trailing 5 %, plafond 5 %)\n");
    }

    // ── Trades clôturés : global puis par univers ──
    let trades: Vec<TradeFerme> = sqlx::query(
        "SELECT asset, verdict, COALESCE(r_realise, 0.0) AS r FROM signaux
         WHERE strategie = 'rockets' AND statut = 'Fermé' AND verdict IS NOT NULL
         ORDER BY ferme_le DESC LIMIT ?",
    )
    .bind(limite)
    .fetch_all(pool)
    .await
    .unwrap_or_default()
    .iter()
    .map(|r| TradeFerme {
        asset: r.get::<String, _>("asset"),
        verdict: r.get::<String, _>("verdict"),
        r: r.get::<f64, _>("r"),
    })
    .collect();

    ctx.push_str(&format!("\n=== TRADES CLÔTURÉS ({}) ===\n", trades.len()));
    if trades.is_empty() {
        ctx.push_str("Aucun trade clôturé — la verticale est jeune. Décris l'état, ne conclus pas sur la performance.\n");
    } else {
        let gagnants = trades.iter().filter(|t| t.r > 0.0).count();
        let r_total: f64 = trades.iter().map(|t| t.r).sum();
        ctx.push_str(&format!(
            "WR={:.0}% | ΣR={:.2} | verdicts : {}\n",
            gagnants as f64 * 100.0 / trades.len() as f64,
            r_total,
            trades.iter().map(|t| t.verdict.clone()).collect::<Vec<_>>().join(", ")
        ));
        for univers in ["crypto", "action"] {
            let groupe: Vec<&TradeFerme> = trades
                .iter()
                .filter(|t| univers_du_symbole(&t.asset) == univers)
                .collect();
            if groupe.is_empty() {
                continue;
            }
            let g = groupe.iter().filter(|t| t.r > 0.0).count();
            let r: f64 = groupe.iter().map(|t| t.r).sum();
            ctx.push_str(&format!(
                "  {univers} : {} trades, WR={:.0}%, ΣR={:.2} ({})\n",
                groupe.len(),
                g as f64 * 100.0 / groupe.len() as f64,
                r,
                groupe.iter().map(|t| t.asset.as_str()).collect::<Vec<_>>().join(", ")
            ));
        }
    }

    // ── Vivier de candidats : palier × univers, suivis vs éliminés ──
    if let Ok(rows) = sqlx::query(
        "SELECT univers, points, elimine_le IS NOT NULL AS elimine FROM rockets_candidats",
    )
    .fetch_all(pool)
    .await
    {
        let suivis: Vec<&sqlx::sqlite::SqliteRow> =
            rows.iter().filter(|r| !r.get::<bool, _>("elimine")).collect();
        ctx.push_str(&format!(
            "\n=== VIVIER DU SCANNER : {} suivis, {} éliminés (conservés pour les faux négatifs) ===\n",
            suivis.len(),
            rows.len() - suivis.len()
        ));
        for univers in ["crypto", "action"] {
            let groupe: Vec<&&sqlx::sqlite::SqliteRow> = suivis
                .iter()
                .filter(|r| {
                    r.get::<String, _>("univers") == univers
                })
                .collect();
            if groupe.is_empty() {
                continue;
            }
            for (min, max, label) in
                [(5i64, 6i64, "5-6 (observation)"), (7, 8, "7-8 (Rocket)"), (9, 10, "9-10 (Alpha)")]
            {
                let n = groupe
                    .iter()
                    .filter(|r| r.get::<i64, _>("points") >= min && r.get::<i64, _>("points") <= max)
                    .count();
                if n > 0 {
                    ctx.push_str(&format!("  {univers} palier {label} : {n} candidat(s)\n"));
                }
            }
        }
    }

    // ── Positions ouvertes ──
    let ouvertes: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM rockets_positions WHERE fermee = 0")
            .fetch_one(pool)
            .await
            .unwrap_or(0);
    ctx.push_str(&format!("\n=== POSITIONS OUVERTES : {ouvertes} ===\n"));

    ctx
}
