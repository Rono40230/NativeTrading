//! Sonde diagnostique BPR — vérifie la LOGIQUE de détection MODULE 6b sur
//! données réelles (DB), indépendamment du Pine. Reproduit verbatim les règles
//! d'appariement du Pine étalon : fenêtre 10 bars, gap opposé le plus récent
//! chevauchant, intersection stricte, anti-doublon 80%, FIFO 10, âge 15.
//!
//! Usage : DATABASE_PATH=../data/trading.db cargo run -p api --bin probe_bpr
//! Si le compte est > 0 → la logique est saine, le bug est côté Pine.
//! Si 0 → la logique est trop stricte (conditions à revoir).

use std::sync::Arc;

use common::{Asset, Timeframe};
use smc::v12::types::BarInput;

const FENETRE: i64 = 10; // i_bprWindow (LuxAlgo défaut)
const AGE_MAX: i64 = 15; // i_bprMaxAge
const FIFO_MAX: usize = 20; // i_maxBPR (Show Last 20 — LuxAlgo)

#[derive(Clone)]
struct Bpr {
    top: f64,
    bot: f64,
    bar: i64,
    is_bull: bool,
    /// Figée (clôture au travers ou âge) — reste affichée, hors scoring.
    dead: bool,
}

/// Applique le lifecycle Pine : mort (figée) si clôture au-delà du bord lointain
/// (bull : close < bot · bear : close > top) ou âge > 15. Les mortes RESTENT au
/// pool (affichage, comportement LuxAlgo) mais sortent du scoring.
fn lifecycle(bprs: &mut [Bpr], close: f64, bar_index: i64) -> usize {
    let mut morts = 0;
    for p in bprs.iter_mut() {
        if p.dead {
            continue;
        }
        let travers = if p.is_bull { close < p.bot } else { close > p.top };
        let vieux = bar_index - p.bar > AGE_MAX;
        if travers || vieux {
            p.dead = true;
            morts += 1;
        }
    }
    morts
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let db_path = std::env::var("DATABASE_PATH")
        .unwrap_or_else(|_| "data/trading.db".to_string());
    let db = Arc::new(db::Database::new(&db_path).await?);
    db.run_migrations().await?;

    for asset_id in ["XAUUSD", "BTC", "NAS100"] {
        let asset = Asset::try_from(asset_id).unwrap_or(Asset::from("XAUUSD"));
        let bougies = db
            .obtenir_bougies(&asset, &Timeframe::M15, 30_000)
            .await
            .unwrap_or_default();
        if bougies.len() < 500 {
            eprintln!("⚠ {} : {} bougies", asset_id, bougies.len());
            continue;
        }
        let mut moteur = smc::v12::SmcV12Engine::new(asset_id, "M15");
        let mut bprs: Vec<Bpr> = Vec::new();
        let mut crees = 0usize;
        let mut bar_index: i64 = -1;
        let mut morts_close = 0usize; // figées par clôture au travers
        let mut morts_age = 0usize; // figées par l'âge (15 bars)

        for b in &bougies {
            let bar: BarInput = BarInput {
                timestamp: b.timestamp.timestamp(),
                open: b.open,
                high: b.high,
                low: b.low,
                close: b.close,
                volume: b.volume,
            };
            moteur.update(&bar);
            bar_index += 1; // bar_index Pine (0-based)
            let n_bar = bar_index - 2;

            // Nouveau FVG cette bar ? → dernier élément du pool né à n_bar.
            let nouveau_bull = moteur
                .fvg
                .bull_zones()
                .last()
                .filter(|z| z.bar as i64 == n_bar)
                .map(|z| (z.top, z.bot));
            let nouveau_bear = moteur
                .fvg
                .bear_zones()
                .last()
                .filter(|z| z.bar as i64 == n_bar)
                .map(|z| (z.top, z.bot));

            // Lifecycle (ordre Pine : détection PUIS lifecycle — la zone née cette
            // bar peut être marquée morte par sa propre clôture, théoriquement
            // impossible géométriquement). FIFO 20 (actives + figées).
            morts_close += lifecycle(&mut bprs, bar.close, bar_index);
            if bprs.len() > FIFO_MAX {
                let excedent = bprs.len() - FIFO_MAX;
                bprs.drain(0..excedent);
            }

            for (is_bull, nouveau) in [(true, nouveau_bull), (false, nouveau_bear)] {
                let Some((n_top, n_bot)) = nouveau else { continue };
                let (opp_top, opp_bot, opp_bar): (Vec<f64>, Vec<f64>, Vec<i64>) = if is_bull {
                    let z = moteur.fvg.bear_zones();
                    (
                        z.iter().map(|x| x.top).collect(),
                        z.iter().map(|x| x.bot).collect(),
                        z.iter().map(|x| x.bar as i64).collect(),
                    )
                } else {
                    let z = moteur.fvg.bull_zones();
                    (
                        z.iter().map(|x| x.top).collect(),
                        z.iter().map(|x| x.bot).collect(),
                        z.iter().map(|x| x.bar as i64).collect(),
                    )
                };
                // Scan du plus récent au plus ancien (fin → début).
                let mut hit: Option<usize> = None;
                for k in (0..opp_top.len()).rev() {
                    if n_bar >= opp_bar[k] && n_bar - opp_bar[k] <= FENETRE {
                        let it = n_top.min(opp_top[k]);
                        let ib = n_bot.max(opp_bot[k]);
                        if it > ib {
                            hit = Some(k);
                            break;
                        }
                    }
                }
                let Some(k) = hit else { continue };
                let t = n_top.min(opp_top[k]);
                let b_ = n_bot.max(opp_bot[k]);
                // Anti-doublon ≥ 80% — BPR ACTIVES seulement (le Pine ignore les figées).
                let dup = bprs
                    .iter()
                    .filter(|p| !p.dead)
                    .any(|p| {
                        let dt = t.min(p.top);
                        let db_ = b_.max(p.bot);
                        let minh = (t - b_).min(p.top - p.bot);
                        minh > 0.0 && (dt - db_) / minh >= 0.8
                    });
                if dup {
                    continue;
                }
                bprs.push(Bpr { top: t, bot: b_, bar: bar_index, is_bull, dead: false });
                crees += 1;
            }
        }
        let vivants = bprs.iter().filter(|p| !p.dead).count();
        // Visibilité « vue graphique » : boxes nées dans les 500 dernières bars
        // (actives ou figées — le sillon doré/gris visible en scrollant).
        let visibles = bprs
            .iter()
            .filter(|p| p.bar >= bar_index - 500)
            .count();
        println!(
            "{} M15 ({} bars) : créés={} · vivants={} · figées={} · VISIBLES sur 500 bars={} · morts clôture={} · morts âge={}",
            asset_id,
            bougies.len(),
            crees,
            vivants,
            bprs.len() - vivants,
            visibles,
            morts_close,
            morts_age
        );
    }
    Ok(())
}
