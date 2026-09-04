//! Re-jeu paramétrique straddle — les passes historiques re-dérivées des
//! réglages courants (moteur unifié). Même principe que `smc_rejeu` : les
//! passes passées (annonces, entrées, R) sont fixes, mais leur ISSUE est
//! recalculée en rejouant les 2 jambes sur les bougies M1 via le LIFECYCLE
//! COMMUN avec les params actuels (trailing ×R, tampon 0,5R, TP3 3R,
//! expiration 60 min). Cache mémoire invalidé par changement de params.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::Instant;

use actix_web::{web, HttpResponse};
use serde::Serialize;
use tokio::sync::RwLock;

use crate::state::AppState;

/// Fenêtre de re-jeu par passe : T-30 (préparation) → T+75 min
/// (time-stop 60 min + marge).
const AVANT_SEC: i64 = 30 * 60;
const APRES_SEC: i64 = 75 * 60;

#[derive(Debug, Clone, Serialize)]
pub struct ClotureRejeuStraddle {
    pub asset: String,
    pub annonce_ts: i64,
    pub ferme_le: i64,
    pub verdict: String,
    /// R net de la passe (survivante + jambe morte — comptabilité TP acquis).
    pub r_net: f64,
    /// Capital simulé après cette passe.
    pub capital_apres: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct RejeuStraddle {
    pub calcule_le: i64,
    pub nb_passes: usize,
    pub duree_ms: u128,
    /// Paramètres utilisés (traçabilité du re-jeu).
    pub trailing_r: f64,
    pub sl_atr: f64,
    pub time_stop_min: i64,
    pub clotures: Vec<ClotureRejeuStraddle>,
    pub total: usize,
    pub gagnants: usize,
    pub taux_reussite: f64,
    /// Σ R net de référence (paliers, pénalité −1R appliquée).
    pub r_total: f64,
    /// Σ R net réalisé.
    pub r_total_net: f64,
    pub capital_depart: f64,
    pub capital_actuel: f64,
}

static CACHE: OnceLock<RwLock<Option<Arc<RejeuStraddle>>>> = OnceLock::new();
static EN_COURS: AtomicBool = AtomicBool::new(false);

const REFRESH_SEC: i64 = 1_800;

fn cache() -> &'static RwLock<Option<Arc<RejeuStraddle>>> {
    CACHE.get_or_init(|| RwLock::new(None))
}

/// Cache chaud du re-jeu straddle (None si jamais calculé ou params changés).
pub async fn lire_cache() -> Option<Arc<RejeuStraddle>> {
    cache().read().await.clone()
}

/// Signature des paramètres qui invalident le cache (trailing + SL de la
/// carte straddle — les autres réglages moteur sont constants).
async fn signature_params(pool: &db::Database) -> (f64, f64) {
    let p = db::strategies_params::lire_straddle_params(pool.pool()).await;
    (p.trailing_atr, p.sl_mult)
}

/// Lance le re-jeu si nécessaire : pas de cache, params changés, ou périmé.
pub async fn lancer_si_necessaire(pool: Arc<db::Database>) {
    let sig = signature_params(&pool).await;
    if let Some(c) = cache().read().await.clone() {
        let frais = chrono::Utc::now().timestamp() - c.calcule_le < REFRESH_SEC;
        let pareil = (c.trailing_r - sig.0).abs() < 1e-9
            && (c.sl_atr - sig.1).abs() < 1e-9;
        if pareil && frais {
            return;
        }
    }
    if EN_COURS.swap(true, Ordering::SeqCst) {
        return;
    }
    tokio::spawn(async move {
        let debut = Instant::now();
        match calculer(&pool).await {
            Ok(rejeu) => {
                tracing::info!(
                    "Rejeu straddle : {} passe(s), WR {:.0} %, R réf {:+.1}, R net {:+.1}, capital {:.2} → {:.2} ({:?})",
                    rejeu.total, rejeu.taux_reussite * 100.0, rejeu.r_total, rejeu.r_total_net,
                    rejeu.capital_depart, rejeu.capital_actuel, debut.elapsed()
                );
                *cache().write().await = Some(Arc::new(rejeu));
            }
            Err(e) => tracing::warn!("Rejeu straddle : échec — {e}"),
        }
        EN_COURS.store(false, Ordering::SeqCst);
    });
}

/// GET /api/straddle/rejeu — passes straddle re-dérivées des params courants.
pub async fn get_rejeu(state: web::Data<AppState>) -> impl actix_web::Responder {
    lancer_si_necessaire(state.db.clone()).await;
    if let Some(c) = cache().read().await.clone() {
        return HttpResponse::Ok().json(serde_json::json!({ "en_cours": false, "rejeu": *c }));
    }
    HttpResponse::Ok().json(serde_json::json!({ "en_cours": true, "rejeu": null }))
}

/// Rejoue UNE passe : 2 jambes au lifecycle commun sur les M1 stockées.
/// Retourne (verdict, r_net, ferme_le) ou None si encore vivante.
fn rejouer_passe(
    entree: f64,
    r: f64,
    ouverture: i64,
    bougies: &[common::Candle],
    trailing_r: f64,
    time_stop_min: i64,
) -> Option<(String, f64, i64)> {
    let bar0 = gestion_trades::BarInput {
        timestamp: ouverture,
        open: entree, high: entree, low: entree, close: entree,
        volume: 0.0,
    };
    let mut long = gestion_trades::Trade::new_buy(
        1, gestion_trades::TradeSource::Ob, entree, entree - r,
        entree + r, entree + 2.0 * r, entree + 3.0 * r,
        78, r, &bar0, 0, None,
    );
    long.filled = true;
    long.fill_ts = Some(ouverture);
    let mut short = gestion_trades::Trade::new_sell(
        2, gestion_trades::TradeSource::Ob, entree, entree + r,
        entree - r, entree - 2.0 * r, entree - 3.0 * r,
        78, r, &bar0, 0, None,
    );
    short.filled = true;
    short.fill_ts = Some(ouverture);
    let mut jambes = [long, short];

    let exp = time_stop_min * 60;
    let mut lifecycle = gestion_trades::TradeLifecycle::new(exp, exp);
    lifecycle.definir_be_offset_r(0.5); // tampon 27/08
    lifecycle.definir_trailing_tp2(Some(trailing_r));

    let mut ferme_le = ouverture;
    for (i, b) in bougies.iter().enumerate() {
        let ts = b.timestamp.timestamp();
        if ts < ouverture {
            continue;
        }
        let bar = gestion_trades::BarInput {
            timestamp: ts,
            open: b.open, high: b.high, low: b.low, close: b.close,
            volume: 0.0,
        };
        lifecycle.update(&mut jambes, &bar, i + 1, &mut gestion_trades::HookVide);
        ferme_le = ts;
        if jambes.iter().all(|t| t.close_reason.is_some()) {
            break;
        }
    }
    if jambes.iter().any(|t| t.close_reason.is_none()) {
        return None;
    }
    let net: f64 = jambes.iter().map(|t| t.close_r.unwrap_or(0.0)).sum();
    let un_tp1 = jambes.iter().any(|t| t.tp1_hit);
    let verdict = if net > 1e-9 {
        "tp2"
    } else if net < -1e-9 {
        "sl"
    } else if un_tp1 {
        "be"
    } else {
        "expire"
    };
    Some((verdict.to_string(), net, ferme_le))
}

/// Palier de référence (pénalité −1R, comme la base).
fn palier_reference(verdict: &str) -> f64 {
    match verdict {
        "tp2" => 1.0,   // TP2 net = 1R
        "sl" => -1.0,
        _ => 0.0,         // be / expire
    }
}

async fn calculer(pool: &Arc<db::Database>) -> anyhow::Result<RejeuStraddle> {
    // Params moteur : trailing lu de la carte straddle (DB), SL via le R
    // historique de chaque passe (rejoué tel quel), time-stop canonique 60.
    let params_db = db::strategies_params::lire_straddle_params(pool.pool()).await;
    let trailing_r = params_db.trailing_atr;
    let time_stop_min: i64 = 60;

    // Source des passes : les signaux straddle (clés avec annonce_ts).
    let actifs: Vec<db::signaux::SignalActifCle> =
        db::signaux::lister_actifs_avec_cle(pool.pool()).await.unwrap_or_default();
    let mut passe_sources: Vec<(String, i64, f64, f64)> = Vec::new(); // (asset, annonce_ts, entree, r)
    for a in &actifs {
        if a.strategie != "straddle" {
            continue;
        }
        // clé : straddle-{asset}-{ts}-B ou straddle-{ts}-B (BTC legacy)
        let parts: Vec<&str> = a.cle_moteur.split('-').collect();
        let (asset, ts) = if parts.len() >= 4 {
            (parts[1].to_string(), parts[2].parse::<i64>().unwrap_or(0))
        } else {
            (a.asset.clone(), a.heure_entree.unwrap_or(a.cree_le))
        };
        let r = (a.prix_entree - a.stop_loss).abs();
        if r > 0.0 && ts > 0 {
            passe_sources.push((asset, ts, a.prix_entree, r));
        }
    }
    // Aussi les clôturés (le rejeu porte sur tout l'historique).
    // → lister_actifs_avec_cle ne donne que les Actifs ; pour les Fermés on
    //   lit les clés via la vue historique.
    let fermes = pool.clotures_pour_capital_straddle().await?;
    for (asset, annonce_ts, entree, r) in fermes {
        passe_sources.push((asset, annonce_ts, entree, r));
    }

    let reg = pool.lire_strategie("straddle").await?.unwrap_or_default();
    let capital_depart = reg.capital;
    let fraction = reg.risque_pct / 100.0;
    let mut capital = capital_depart;

    let mut clotures: Vec<ClotureRejeuStraddle> = Vec::new();
    // Fenêtre M1 : depuis la PLUS VIEILLE passe (les bougies nécessaires
    // s'étendent de T-30 à T+75 autour de chaque annonce).
    let plus_vieille = passe_sources.iter().map(|(_, ts, _, _)| *ts).min().unwrap_or(0);
    let jours_requis = ((chrono::Utc::now().timestamp() - plus_vieille) / 86_400 + 1).max(1) as u32;
    for (asset, annonce_ts, entree, r) in &passe_sources {
        let actif = common::Asset::from(asset.as_str());
        let tf = common::Timeframe::M1;
        // Bougies M1 autour de la passe : T-30 → T+75 min.
        let bougies = pool
            .obtenir_bougies_depuis_jours(&actif, &tf, jours_requis)
            .await
            .unwrap_or_default();
        let fenetre: Vec<common::Candle> = bougies
            .into_iter()
            .filter(|b| {
                let ts = b.timestamp.timestamp();
                ts >= annonce_ts - AVANT_SEC && ts <= annonce_ts + APRES_SEC
            })
            .collect();
        if fenetre.is_empty() {
            continue;
        }
        let ouverture = annonce_ts - 10; // T-10 s (placement par le timer)
        if let Some((verdict, net, ferme_le)) =
            rejouer_passe(*entree, *r, ouverture, &fenetre, trailing_r, time_stop_min)
        {
            let _ = palier_reference(&verdict);
            capital += net * capital * fraction;
            clotures.push(ClotureRejeuStraddle {
                asset: asset.clone(),
                annonce_ts: *annonce_ts,
                ferme_le,
                verdict,
                r_net: net,
                capital_apres: capital,
            });
        }
    }

    // Capital chronologique.
    clotures.sort_by_key(|c| c.ferme_le);
    let mut capital2 = capital_depart;
    for c in &mut clotures {
        capital2 += c.r_net * capital2 * fraction;
        c.capital_apres = capital2;
    }

    let total = clotures.len();
    let gagnants = clotures.iter().filter(|c| c.r_net > 0.0).count();
    let r_total_net: f64 = clotures.iter().map(|c| c.r_net).sum();
    // Référence : paliers avec pénalité (tp2 → 1R net, sl → −1R, autres 0).
    let r_total: f64 = clotures
        .iter()
        .map(|c| match c.verdict.as_str() {
            "tp2" => 1.0,
            "sl" => -1.0,
            _ => 0.0,
        })
        .sum();

    Ok(RejeuStraddle {
        calcule_le: chrono::Utc::now().timestamp(),
        nb_passes: passe_sources.len(),
        duree_ms: 0,
        trailing_r,
        sl_atr: params_db.sl_mult,
        time_stop_min,
        clotures,
        total,
        gagnants,
        taux_reussite: if total > 0 { gagnants as f64 / total as f64 } else { 0.0 },
        r_total,
        r_total_net,
        capital_depart,
        capital_actuel: capital2,
    })
}
