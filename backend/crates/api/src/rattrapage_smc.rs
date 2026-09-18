//! Rattrapage des positions SMC orphelines (incident 18/09).
//!
//! Le suivi des trades SMC vit dans l'état mémoire du moteur runtime : un
//! redémarrage de l'app orphelinise toute position encore ouverte (le
//! redémarrage du 17/09 19h30 a laissé 5 positions M15 sans gestion — LTC a
//! touché TP3 le lendemain sans jamais clôturer). Ce poste reprend le relais :
//! à chaque tick, toute position SMC remplie encore Active est re-jouée depuis
//! son entrée par le MÊME lifecycle que le moteur (gestion_trades, validé
//! miroir Pine) — si sa condition de sortie est atteinte, elle est clôturée
//! officiellement (même format que le runtime) ; sinon son suivi progressif
//! (sl_effectif / tps_atteints — la fonction restée morte jusqu'ici) est
//! persisté. Idempotent : une position déjà Fermée n'est jamais retouchée,
//! et le moteur vivant clôturant le premier rend l'écriture de l'autre no-op.

use crate::state::AppState;
use actix_web::{web, HttpResponse};
use std::sync::Arc;

/// Verdict canonique en base — miroir de engine_v12::lifecycle_diff.
fn verdict_texte(t: &gestion_trades::Trade) -> String {
    use gestion_trades::Verdict;
    match t.verdict() {
        Verdict::Tp3 => "TP3",
        Verdict::Ts => "TS",
        Verdict::Tp2 => "TP2+BE",
        Verdict::Tp1 => "TP1+BE",
        Verdict::Sl => "SL",
        Verdict::Be => "BE",
        Verdict::Expire => "Expire",
    }
    .to_string()
}

/// Prix de sortie selon la cause (même logique que le moteur).
fn prix_de_sortie(t: &gestion_trades::Trade, close_bougies: f64) -> f64 {
    use gestion_trades::CloseReason;
    match t.close_reason {
        Some(CloseReason::Ts) => t.ts_px.unwrap_or(close_bougies),
        Some(CloseReason::Tp2Sl) => t.tp1,
        Some(CloseReason::Be) => t.entry,
        Some(CloseReason::Sl) => t.sl,
        Some(CloseReason::Tp3) => t.tp3,
        _ => close_bougies,
    }
}

/// Une position ouverte à rejouer.
struct Orphelin {
    id: String,
    tf_nom: String,
    cle: String,
    asset: String,
    tf_mins: u32,
    long: bool,
    entree: f64,
    sl: f64,
    tps: Vec<f64>,
    heure_entree: i64,
    sl_effectif_actuel: Option<f64>,
    tps_atteints_actuel: Option<String>,
}

/// Passe de rattrapage : clôture les sorties dues, persiste le suivi des
/// vivantes. Appelée à chaque tick runtime (coût : lecture des Actifs + rejeu
/// des bougies depuis l'entrée — quelques centaines au pire).
pub async fn rattraper(db: &Arc<db::Database>) {
    let rows = match sqlx::query(
        "SELECT id, cle_moteur, asset, timeframe, direction, prix_entree, stop_loss,
                take_profit, heure_entree, sl_effectif, tps_atteints
         FROM signaux
         WHERE strategie = 'SMC' AND statut = 'Actif' AND heure_entree IS NOT NULL
           AND cle_moteur IS NOT NULL",
    )
    .fetch_all(db.pool())
    .await
    {
        Ok(r) => r,
        Err(_) => return,
    };

    for r in rows {
        use sqlx::Row;
        let tps: Vec<f64> = r
            .try_get::<String, _>("take_profit")
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        let o = Orphelin {
            id: r.get("id"),
            cle: r.get("cle_moteur"),
            asset: r.get("asset"),
            tf_mins: match r.get::<String, _>("timeframe").as_str() {
                "M1" => 1,
                "M5" => 5,
                "M30" => 30,
                "H1" => 60,
                "H4" => 240,
                "D1" => 1440,
                _ => 15,
            },
            tf_nom: r.get("timeframe"),
            long: r.get::<String, _>("direction").eq_ignore_ascii_case("long"),
            entree: r.try_get("prix_entree").unwrap_or(0.0),
            sl: r.try_get("stop_loss").unwrap_or(0.0),
            tps,
            heure_entree: r.get("heure_entree"),
            sl_effectif_actuel: r.try_get::<Option<f64>, _>("sl_effectif").ok().flatten(),
            tps_atteints_actuel: r
                .try_get::<Option<String>, _>("tps_atteints")
                .ok()
                .flatten(),
        };
        if o.tps.len() < 2 || o.entree <= 0.0 {
            continue;
        }
        if let Err(e) = rattraper_un(db, &o).await {
            tracing::warn!("Rattrapage SMC {} {}: {}", o.asset, o.id, e);
        }
    }
}

async fn rattraper_un(db: &Arc<db::Database>, o: &Orphelin) -> anyhow::Result<()> {
    let jours = ((chrono::Utc::now().timestamp() - o.heure_entree) / 86_400 + 2).max(1) as u32;
    let asset = common::Asset::from(o.asset.as_str());
    let Ok(tf) = common::Timeframe::try_from(o.tf_nom.as_str()) else {
        return Ok(());
    };
    let bougies = db
        .obtenir_bougies_depuis_jours(&asset, &tf, jours)
        .await
        .unwrap_or_default();
    let fenetre: Vec<&common::Candle> = bougies
        .iter()
        .filter(|b| b.timestamp.timestamp() >= o.heure_entree)
        .collect();
    if fenetre.is_empty() {
        return Ok(());
    }

    // Trade reconstruit à l'identique du moteur (niveaux du signal, risk0).
    let bar0 = gestion_trades::BarInput {
        timestamp: o.heure_entree,
        open: o.entree, high: o.entree, low: o.entree, close: o.entree,
        volume: 0.0,
    };
    let mut trade = if o.long {
        gestion_trades::Trade::new_buy(
            1, gestion_trades::TradeSource::Ob, o.entree, o.sl,
            o.tps[0], o.tps[1], *o.tps.get(2).unwrap_or(&o.tps[1]),
            10, (o.entree - o.sl).abs(), &bar0, 0, None,
        )
    } else {
        gestion_trades::Trade::new_sell(
            1, gestion_trades::TradeSource::Ob, o.entree, o.sl,
            o.tps[0], o.tps[1], *o.tps.get(2).unwrap_or(&o.tps[1]),
            10, (o.entree - o.sl).abs(), &bar0, 0, None,
        )
    };
    trade.filled = true;
    trade.fill_ts = Some(o.heure_entree);

    // MÊMES time-stops que le moteur v12 (durees.rs).
    let trade_max = smc::v12::durees::trade_max_mins(o.tf_mins) * 60;
    let cal = smc::v12::calibration::AssetCalibration::detect(&o.asset, &o.tf_nom);
    let tp3_max = smc::v12::durees::tp3_max_mins(&cal, o.tf_mins) * 60;
    let mut lifecycle = gestion_trades::TradeLifecycle::new(trade_max, tp3_max);
    lifecycle.definir_be_offset_r(0.0); // sémantique SMC

    let mut jambes = [trade];
    let mut ferme: Option<(String, f64, f64, i64)> = None;
    for (i, b) in fenetre.iter().enumerate() {
        let bar = gestion_trades::BarInput {
            timestamp: b.timestamp.timestamp(),
            open: b.open, high: b.high, low: b.low, close: b.close,
            volume: 0.0,
        };
        lifecycle.update(&mut jambes, &bar, i + 1, &mut gestion_trades::HookVide);
        let trade = &jambes[0];
        if trade.close_reason.is_some() {
            let verdict = verdict_texte(&trade);
            let prix = prix_de_sortie(&trade, b.close);
            let r = trade.realized_r();
            ferme = Some((verdict, prix, r, b.timestamp.timestamp()));
            break;
        }
    }

    let trade = &jambes[0];
    if let Some((verdict, prix, r, ferme_le)) = ferme {
        let n = db
            .fermer_signal_par_cle(&o.cle, &o.asset, &verdict, prix, r, ferme_le)
            .await
            .unwrap_or(0);
        if n > 0 {
            tracing::info!(
                "🩹 Rattrapage SMC : {} {} clôturé {} ({:+.2}R) — position orpheline réparée",
                o.asset, o.tf_nom, verdict, r
            );
        }
        return Ok(());
    }

    // Toujours vivante : persister le suivi progressif (fonction historiquement
    // morte — branchée ici) uniquement s'il a changé.
    let mut tps_touches: Vec<&str> = Vec::new();
    if trade.tp1_hit {
        tps_touches.push("tp1");
    }
    if trade.tp2_ts > 0 {
        tps_touches.push("tp2");
    }
    let tps_json = serde_json::to_string(&tps_touches).unwrap_or_default();
    let inchangé = o.sl_effectif_actuel == Some(trade.sl)
        && o.tps_atteints_actuel.as_deref() == Some(tps_json.as_str());
    if !inchangé {
        let _ = db::signaux::maj_suivi_progressif_smc(
            db.pool(),
            &o.id,
            trade.sl,
            &tps_touches,
        )
        .await;
    }
    Ok(())
}

/// État du rattrapage pour l'UI (endpoint de contrôle).
pub async fn get_etat(state: web::Data<AppState>) -> impl actix_web::Responder {
    let orphelins: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM signaux
         WHERE strategie='SMC' AND statut='Actif' AND heure_entree IS NOT NULL",
    )
    .fetch_one(state.db.pool())
    .await
    .unwrap_or(0);
    HttpResponse::Ok().json(serde_json::json!({
        "positions_actives_smc": orphelins,
        "poste": "rattrapage au tick (lifecycle moteur, idempotent)",
    }))
}
