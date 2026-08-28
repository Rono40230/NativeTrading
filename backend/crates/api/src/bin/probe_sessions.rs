//! Sonde Module F — vérifie que le bonus Sessions H/L s'ACTIVE réellement en
//! replay (comptage des bars où la proximité H/L de session est vraie),
//! pour distinguer « aucun apport mesurable » de « greffon mort ».
//!
//! Usage : cargo run --release -p api --bin probe_sessions

use std::sync::Arc;

use common::{Asset, Timeframe};
use smc::v12::types::BarInput;

const ASSETS: &[&str] = &["BTC", "XAUUSD", "XAGUSD", "NAS100", "DAX"];

fn main() -> anyhow::Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(executer())
}

async fn executer() -> anyhow::Result<()> {
    let db_path = std::env::var("DATABASE_PATH")
        .unwrap_or_else(|_| "data/trading.db".to_string());
    let db = Arc::new(db::Database::new(&db_path).await?);
    db.run_migrations().await?;

    for asset_id in ASSETS {
        let Ok(asset) = Asset::try_from(*asset_id) else { continue };
        for tf_id in ["M15", "M5", "M1"] {
            let Ok(tf) = Timeframe::try_from(tf_id) else { continue };
            let bougies = db
                .obtenir_bougies(&asset, &tf, 20_000)
                .await
                .unwrap_or_default();
            if bougies.len() < 500 {
                continue;
            }
            let mut moteur = smc::v12::SmcV12Engine::new(asset_id, tf_id);
            // Niveaux drawn de la bar N-1 (même logique que le greffon moteur).
            let (mut ah_h, mut ah_l, mut ld_h, mut ld_l) =
                (None, None, None, None);
            let (mut bars_near_bull, mut bars_near_bear) = (0u32, 0u32);
            let mut bars = 0u32;
            let mut trades = moteur_len(&moteur);
            let mut trades_nees_near = 0u32;
            for b in &bougies {
                let bar = BarInput {
                    timestamp: b.timestamp.timestamp(),
                    open: b.open,
                    high: b.high,
                    low: b.low,
                    close: b.close,
                    volume: b.volume,
                };
                let out = moteur.update(&bar);
                let atr = out.atr14;
                if atr > 0.0 {
                    let prox = 0.35 * atr;
                    let near = |n: Option<f64>| {
                        n.is_some_and(|v| (bar.close - v).abs() <= prox)
                    };
                    let near_bull = near(ah_l) || near(ld_l);
                    let near_bear = near(ah_h) || near(ld_h);
                    if near_bull {
                        bars_near_bull += 1;
                    }
                    if near_bear {
                        bars_near_bear += 1;
                    }
                    // Trade né cette bar pendant proximité ?
                    let n = moteur_len(&moteur);
                    if n > trades {
                        trades = n;
                        if near_bull || near_bear {
                            trades_nees_near += 1;
                        }
                    }
                }
                // Rotation N-1 (après le scoring — comme le moteur).
                ah_h = out.asian_hl.high;
                ah_l = out.asian_hl.low;
                ld_h = out.london_hl.high;
                ld_l = out.london_hl.low;
                bars += 1;
            }
            println!(
                "{} {} : {} bars · near(bull)={} · near(bear)={} · trades nés pendant proximité={}",
                asset_id,
                tf_id,
                bars,
                bars_near_bull,
                bars_near_bear,
                trades_nees_near
            );
        }
    }
    Ok(())
}

fn moteur_len(m: &smc::v12::SmcV12Engine) -> usize {
    m.signals.trades.len()
}
