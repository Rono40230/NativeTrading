//! Étape 5 — verticale Rockets : scanner D1 + gestion des positions.
//!
//! Scanner (1×/jour après la clôture D1 + au boot) : top 100 Binance USDT
//! en volume → classement /10 (crate rockets) → candidats journalisés ;
//! cassure au pivot → signal officiel (moteur « rockets », état Observation :
//! journalisé, silencieux) + position ouverte dans rockets_positions.
//!
//! Gestion (30 min) : chaque bougie D1 confirmée fait vivre les positions —
//! invalidation (−1R) OU R1 → vendre 50 % + trailing % → sortie (crate
//! rockets::gestion). Les clôtures ferment les lignes signaux avec verdict
//! et R réel.
//!
//! V1 honnête : détection ET gestion sur clôtures D1 (le live au tick des
//! pivots viendra avec l'extension des abonnements WS) ; entrée = prix du
//! pivot (stop-limit théorique, sans slippage) ; le point « News » (1/10)
//! est réservé à l'IA (étape 6) et le véto unlocks aux calendriers externes.

use std::sync::Arc;

use db::Database;
use common::Direction;
use engine::types::SignalBrut;
use engine::BusSignaux;
use rockets::gestion::{pas_gestion, PositionRocket};
use rockets::types::{ParamsRockets, ProfilRisque};
use rockets::{classement_rocket, BougieD1, ContexteMarche};
use sqlx::Row;

/// Nom du moteur (manifeste rockets).
const MOTEUR: &str = "rockets";

pub fn demarrer(db: Arc<Database>, bus: BusSignaux) {
    tokio::spawn(boucle_scan(db.clone(), bus));
    tokio::spawn(boucle_gestion(db));
}

// ── Paramètres (table rockets_params, carte Paramètres › Rockets) ───────────

pub async fn lire_params(db: &Database) -> ParamsRockets {
    let row = sqlx::query("SELECT profil, plafond_position_pct, trailing_pct, volume_pivot_mult, cassure_min_pct FROM rockets_params WHERE id = 1")
        .fetch_optional(db.pool())
        .await
        .ok()
        .flatten();
    let profil = row
        .as_ref()
        .and_then(|r| r.try_get::<String, _>("profil").ok())
        .map(|p| match p.as_str() {
            "PeuRisque" => ProfilRisque::PeuRisque,
            "Risque" => ProfilRisque::Risque,
            _ => ProfilRisque::Neutre,
        })
        .unwrap_or(ProfilRisque::Neutre);
    ParamsRockets {
        profil,
        plafond_position_pct: row.as_ref().and_then(|r| r.try_get("plafond_position_pct").ok()).unwrap_or(5.0),
        trailing_pct: row.as_ref().and_then(|r| r.try_get("trailing_pct").ok()).unwrap_or(5.0),
        volume_pivot_mult: row.as_ref().and_then(|r| r.try_get("volume_pivot_mult").ok()).unwrap_or(1.5),
        cassure_min_pct: row.as_ref().and_then(|r| r.try_get("cassure_min_pct").ok()).unwrap_or(3.0),
    }
}

// ── Accès Binance REST ──────────────────────────────────────────────────────

async fn klines_d1(symbole: &str, limite: usize) -> Vec<BougieD1> {
    let url = format!(
        "https://api.binance.com/api/v3/klines?symbol={}&interval=1d&limit={}",
        symbole, limite
    );
    let rep = match crate::http_client::HTTP_CLIENT.get(&url).send().await {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    let json: Vec<serde_json::Value> = match rep.json().await {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    json.iter()
        .filter_map(|k| {
            Some(BougieD1 {
                ts: k.get(0)?.as_i64()?,
                open: k.get(1)?.as_str()?.parse().ok()?,
                high: k.get(2)?.as_str()?.parse().ok()?,
                low: k.get(3)?.as_str()?.parse().ok()?,
                close: k.get(4)?.as_str()?.parse().ok()?,
                volume: k.get(5)?.as_str()?.parse().ok()?,
            })
        })
        .collect()
}

/// Top N paires USDT par volume 24 h (noire : leviers UP/DOWN, stables).
async fn univers_top(n: usize, db: &Database) -> Vec<String> {
    let rep = match crate::http_client::HTTP_CLIENT
        .get("https://api.binance.com/api/v3/ticker/24hr")
        .send()
        .await
    {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    let json: Vec<serde_json::Value> = match rep.json().await {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    let mut paires: Vec<(String, f64)> = json
        .iter()
        .filter_map(|t| {
            let symbole = t.get("symbol")?.as_str()?.to_string();
            if !symbole.ends_with("USDT") || symbole.ends_with("UPUSDT") || symbole.ends_with("DOWNUSDT") {
                return None;
            }
            let base = &symbole[..symbole.len() - 4];
            if matches!(base, "USDC" | "FDUSD" | "TUSD" | "BUSD" | "DAI") {
                return None;
            }
            let volume: f64 = t.get("quoteVolume")?.as_str()?.parse().ok()?;
            Some((symbole, volume))
        })
        .collect();
    paires.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    paires.truncate(n);
    let mut candidats: Vec<String> = Vec::new();
    for (s, _) in paires {
        let blackliste = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM rockets_blacklist WHERE ticker = ?",
        )
        .bind(&s)
        .fetch_one(db.pool())
        .await
        .unwrap_or(0);
        if blackliste == 0 {
            candidats.push(s);
        }
    }
    candidats
}

// ── Scanner quotidien ───────────────────────────────────────────────────────

async fn boucle_scan(db: Arc<Database>, bus: BusSignaux) {
    tracing::info!("🚀 Rockets scanner armé (quotidien + boot)");
    scanner(&db, &bus).await;
    loop {
        // Prochain passage à 00h40 UTC (après la clôture D1).
        let maintenant = chrono::Utc::now();
        let prochain = (maintenant + chrono::Duration::hours(1))
            .date_naive()
            .and_hms_opt(0, 40, 0)
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
        scanner(&db, &bus).await;
    }
}

async fn scanner(db: &Arc<Database>, bus: &BusSignaux) {
    // Contexte BTC : régime + performance 4 semaines.
    let btc = klines_d1("BTCUSDT", 260).await;
    let clotures: Vec<f64> = btc.iter().map(|b| b.close).collect();
    let (m50, m200) = (
        rockets::classement::mma(&clotures, 50),
        rockets::classement::mma(&clotures, 200),
    );
    let btc_haussier = matches!((m50, m200), (Some(a), Some(b)) if btc.last().map(|d| d.close > a && a > b).unwrap_or(false));
    let perf_btc_4s = btc
        .last()
        .zip(btc.len().checked_sub(29).map(|i| &btc[i]))
        .map(|(d, vieux)| d.close / vieux.close - 1.0)
        .unwrap_or(0.0);
    let ctx = ContexteMarche { btc_haussier, perf_btc_4s };

    let univers = univers_top(100, db).await;
    tracing::info!("🚀 Rockets scan : {} symboles, BTC haussier={}", univers.len(), btc_haussier);

    let mut nb_candidats = 0usize;
    let mut nb_signaux = 0usize;
    for symbole in &univers {
        let bougies = klines_d1(symbole, 220).await;
        if bougies.len() < 210 {
            continue;
        }
        // Stablecoins et actifs figés : amplitude 220 j < 10 % → hors jeu.
        let (haut, bas) = bougies.iter().fold((f64::MIN, f64::MAX), |(h, l), b| (h.max(b.high), l.min(b.low)));
        if bas > 0.0 && haut / bas - 1.0 < 0.10 {
            continue;
        }
        let r = classement_rocket(symbole, &bougies, &ctx);
        let ts_derniere = bougies.last().map(|b| b.ts).unwrap_or(0);

        // Journal des candidats (≥ 5 points : suivis en approche).
        if r.points >= 5 {
            nb_candidats += 1;
            let _ = sqlx::query(
                "INSERT OR REPLACE INTO rockets_candidats (symbole, points, verdict, pivot, stop, cassure, detail, maj_le)
                 VALUES (?, ?, ?, ?, ?, ?, ?, strftime('%s','now'))",
            )
            .bind(&r.symbole)
            .bind(r.points as i64)
            .bind(format!("{:?}", r.verdict))
            .bind(r.pivot.unwrap_or(0.0))
            .bind(r.stop.unwrap_or(0.0))
            .bind(r.cassure)
            .bind(serde_json::to_string(&r.detail).unwrap_or_default())
            .execute(db.pool())
            .await;
        }

        // Cassure au pivot + classement ≥ 7 → signal officiel + position.
        if r.cassure && r.points >= 7 {
            if let (Some(pivot), Some(stop)) = (r.pivot, r.stop) {
                let cle = format!("rockets-{}-{}", r.symbole, ts_derniere);
                let deja = sqlx::query_scalar::<_, i64>(
                    "SELECT COUNT(*) FROM rockets_positions WHERE cle = ?",
                )
                .bind(&cle)
                .fetch_one(db.pool())
                .await
                .unwrap_or(0);
                if deja == 0 {
                    let _ = sqlx::query(
                        "INSERT OR IGNORE INTO rockets_positions (cle, symbole, entree, stop, r1, neutralise, trailing, ts_entree, fermee)
                         VALUES (?, ?, ?, ?, ?, 0, NULL, ?, 0)",
                    )
                    .bind(&cle)
                    .bind(&r.symbole)
                    .bind(pivot)
                    .bind(stop)
                    .bind(pivot + (pivot - stop))
                    .bind(ts_derniere)
                    .execute(db.pool())
                    .await;
                    bus.publier(SignalBrut::avec_cle(
                        MOTEUR,
                        common::Asset::nouveau(&r.symbole),
                        common::Timeframe::try_from("D1").unwrap_or(common::Timeframe::D1),
                        Direction::Long,
                        pivot,
                        stop,
                        vec![pivot + (pivot - stop)],
                        r.points as i32,
                        format!("rockets {} {}/{} pivot={:.4}", r.symbole, r.points, 10, pivot),
                        ts_derniere,
                        cle.clone(),
                    ));
                    nb_signaux += 1;
                }
            }
        }
        // Ménage : purge des candidats disparus de l'univers.
        tokio::time::sleep(std::time::Duration::from_millis(120)).await;
    }
    // Candidats non rafraîchis depuis 2 jours : sortis de l'univers actif.
    let _ = sqlx::query("DELETE FROM rockets_candidats WHERE maj_le < strftime('%s','now') - 2*86400")
        .execute(db.pool())
        .await;
    tracing::info!("🚀 Rockets scan terminé : {} candidats, {} signal(s)", nb_candidats, nb_signaux);
}

// ── Gestion des positions ouvertes ─────────────────────────────────────────

async fn boucle_gestion(db: Arc<Database>) {
    tracing::info!("🚀 Rockets gestion armée (30 min)");
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(30 * 60)).await;
        gerer_positions(&db).await;
    }
}

async fn gerer_positions(db: &Arc<Database>) {
    let params = lire_params(db).await;
    let lignes = match sqlx::query(
        "SELECT cle, symbole, entree, stop, r1, neutralise, trailing FROM rockets_positions WHERE fermee = 0",
    )
    .fetch_all(db.pool())
    .await
    {
        Ok(l) => l,
        Err(_) => return,
    };
    for l in lignes {
        let cle: String = l.get("cle");
        let symbole: String = l.get("symbole");
        let mut p = PositionRocket {
            symbole: symbole.clone(),
            entree: l.get("entree"),
            stop: l.get("stop"),
            r1: l.get("r1"),
            neutralise: l.get::<i64, _>("neutralise") != 0,
            trailing: l.try_get::<Option<f64>, _>("trailing").ok().flatten(),
        };
        // Bougie D1 CONFIRMÉE = l'avant-dernière (la dernière se forme).
        let bougies = klines_d1(&symbole, 3).await;
        let Some(b) = bougies.get(bougies.len().saturating_sub(2)) else { continue };
        match pas_gestion(&mut p, b.high, b.low, b.close, &params) {
            rockets::gestion::ActionRocket::Rien => {
                let _ = sqlx::query("UPDATE rockets_positions SET neutralise = ?, trailing = ? WHERE cle = ?")
                    .bind(p.neutralise as i64)
                    .bind(p.trailing)
                    .bind(&cle)
                    .execute(db.pool())
                    .await;
            }
            rockets::gestion::ActionRocket::Neutraliser { prix, trailing } => {
                let _ = sqlx::query("UPDATE rockets_positions SET neutralise = 1, trailing = ?, prix_r1 = ? WHERE cle = ?")
                    .bind(trailing)
                    .bind(prix)
                    .bind(&cle)
                    .execute(db.pool())
                    .await;
                tracing::info!("🚀 Rockets {} : R1 atteint — 50 % vendus, trailing {:.4}", symbole, trailing);
            }
            rockets::gestion::ActionRocket::Cloturer { prix, verdict, r_realise } => {
                let verdict_str = match verdict {
                    rockets::gestion::VerdictRocket::Sl => "SL",
                    _ => "TS",
                };
                let _ = sqlx::query("UPDATE rockets_positions SET fermee = 1, verdict = ?, r_realise = ?, prix_sortie = ? WHERE cle = ?")
                    .bind(verdict_str)
                    .bind(r_realise)
                    .bind(prix)
                    .bind(&cle)
                    .execute(db.pool())
                    .await;
                let _ = db.fermer_signal_par_cle(&cle, verdict_str, prix, r_realise, chrono::Utc::now().timestamp()).await;
                tracing::info!("🚀 Rockets {} : {} ({:.2} R)", symbole, verdict_str, r_realise);
            }
        }
    }
}

// ── API : candidats du scanner (page Scanner + vérification) ────────────────

/// GET /api/rockets/candidats — candidats classés (≥ 5 points), du mieux
/// noté au moins bien noté, avec pivot/stop/cassure et détail JSON.
pub async fn get_candidats(state: actix_web::web::Data<crate::state::AppState>) -> impl actix_web::Responder {
    let rows = match sqlx::query(
        "SELECT symbole, points, verdict, pivot, stop, cassure, detail, maj_le
         FROM rockets_candidats ORDER BY points DESC, maj_le DESC LIMIT 60",
    )
    .fetch_all(state.db.pool())
    .await
    {
        Ok(r) => r,
        Err(e) => {
            return actix_web::HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": e.to_string() }))
        }
    };
    let liste: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "symbole": r.get::<String, _>("symbole"),
                "points": r.get::<i64, _>("points"),
                "verdict": r.get::<String, _>("verdict"),
                "pivot": r.get::<f64, _>("pivot"),
                "stop": r.get::<f64, _>("stop"),
                "cassure": r.get::<i64, _>("cassure") != 0,
                "detail": serde_json::from_str::<serde_json::Value>(
                    &r.get::<String, _>("detail")).unwrap_or(serde_json::json!({})),
                "maj_le": r.get::<i64, _>("maj_le"),
            })
        })
        .collect();
    actix_web::HttpResponse::Ok().json(liste)
}

// ── API : paramètres de la stratégie (carte Paramètres › Rockets) ───────────

#[derive(serde::Deserialize)]
pub struct BodyParamsRockets {
    pub profil: Option<String>,
    pub plafond_position_pct: Option<f64>,
    pub trailing_pct: Option<f64>,
    pub volume_pivot_mult: Option<f64>,
    pub cassure_min_pct: Option<f64>,
}

/// PUT /api/rockets/params — profil de risque, plafond, trailing, seuils.
pub async fn maj_params(state: actix_web::web::Data<crate::state::AppState>, body: actix_web::web::Json<BodyParamsRockets>) -> impl actix_web::Responder {
    let actuel = lire_params(&state.db).await;
    let profil = body.profil.clone().unwrap_or_else(|| actuel.profil.libelle().to_string());
    if !matches!(profil.as_str(), "PeuRisque" | "Neutre" | "Risque") {
        return actix_web::HttpResponse::BadRequest()
            .json(serde_json::json!({ "error": "Profil invalide (PeuRisque | Neutre | Risque)" }));
    }
    let maj = sqlx::query(
        "UPDATE rockets_params SET profil = ?, plafond_position_pct = ?, trailing_pct = ?, volume_pivot_mult = ?, cassure_min_pct = ? WHERE id = 1",
    )
    .bind(&profil)
    .bind(body.plafond_position_pct.unwrap_or(actuel.plafond_position_pct).clamp(1.0, 25.0))
    .bind(body.trailing_pct.unwrap_or(actuel.trailing_pct).clamp(1.0, 30.0))
    .bind(body.volume_pivot_mult.unwrap_or(actuel.volume_pivot_mult).clamp(1.0, 3.0))
    .bind(body.cassure_min_pct.unwrap_or(actuel.cassure_min_pct).clamp(1.0, 10.0))
    .execute(state.db.pool())
    .await;
    match maj {
        Ok(_) => actix_web::HttpResponse::Ok().json(serde_json::json!({ "ok": true, "profil": profil })),
        Err(e) => actix_web::HttpResponse::InternalServerError().json(serde_json::json!({ "error": e.to_string() })),
    }
}
