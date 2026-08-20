//! Outil de diagnostic des zones OB v12 (étape 2 pyramide — écart scoring).
//!
//! Rejoue le moteur sur les N dernières bougies et imprime chaque zone active :
//! bornes, état, force et DIAG (flags actifs au moment du nouveau max du
//! score — même format que `diagFlags` du MQL5). Comparaison directe avec la
//! table debug TradingView.
//!
//! ```sh
//! cargo run -p api --bin debug_zones -- --asset BTC --tf M15 --limit 5000
//! ```

use std::sync::Arc;

use common::{Asset, Timeframe};
use db::Database;
use smc::v12::{SmcOutput, SmcV12Engine};

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let mut asset_str = "BTC".to_string();
    let mut tf_str = "M15".to_string();
    let mut limit: i64 = 5000;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--asset" if i + 1 < args.len() => {
                asset_str = args[i + 1].clone();
                i += 1;
            }
            "--tf" if i + 1 < args.len() => {
                tf_str = args[i + 1].clone();
                i += 1;
            }
            "--limit" if i + 1 < args.len() => {
                limit = args[i + 1].parse().unwrap_or(5000);
                i += 1;
            }
            _ => {}
        }
        i += 1;
    }

    let db_path = std::env::var("DATABASE_PATH")
        .unwrap_or_else(|_| "data/trading.db".to_string());
    let rt = tokio::runtime::Runtime::new()?;
    let db = rt.block_on(Database::new(&db_path))?;
    let asset = Asset::try_from(asset_str.as_str())?;
    let timeframe = Timeframe::try_from(tf_str.as_str())?;
    let bougies = rt.block_on(db.obtenir_bougies(&asset, &timeframe, limit))?;
    let db = Arc::new(db); // garde vivant jusqu'à la fin
    drop(db);

    println!(
        "═ debug_zones {} {} — {} bougies ═",
        asset_str,
        tf_str,
        bougies.len()
    );
    let mut engine = SmcV12Engine::new(&asset_str, &tf_str);
    let mut dernier: Option<(i64, SmcOutput)> = None;
    for b in &bougies {
        let bar = smc::v12::BarInput {
            timestamp: b.timestamp.timestamp(),
            open: b.open,
            high: b.high,
            low: b.low,
            close: b.close,
            volume: b.volume,
        };
        let out = engine.update(&bar);
        dernier = Some((b.timestamp.timestamp(), out));
    }
    let _ = dernier;

    let cal = &engine.calibration;
    println!("\n── Pool EQH/EQL (MODULE 4) ──");
    for l in engine.liquidites.pool() {
        println!(
            "  {} {:.0} · touches {} · {}",
            if l.is_high { "EQH" } else { "EQL" },
            l.price,
            l.touches,
            if l.swept { "SWEEPÉ" } else { "actif" }
        );
    }
    println!("\n── Zones OB actives ──");
    for z in engine.order_blocks.bull_zones() {
        let score = engine.scoring_v11.ob_score(true, z.impulse_bar);
        let force = smc::v12::scoring_v11::ScoringV11::force(score, cal);
        let diag = engine.scoring_v11.ob_diag(true, z.impulse_bar).unwrap_or("?");
        println!(
            "  BULL {:.0}/{:.0} · état {:?} · score {score} · force {force}\n       diag: {diag}",
            z.top, z.bot, z.state
        );
    }
    for z in engine.order_blocks.bear_zones() {
        let score = engine.scoring_v11.ob_score(false, z.impulse_bar);
        let force = smc::v12::scoring_v11::ScoringV11::force(score, cal);
        let diag = engine.scoring_v11.ob_diag(false, z.impulse_bar).unwrap_or("?");
        println!(
            "  BEAR {:.0}/{:.0} · état {:?} · score {score} · force {force}\n       diag: {diag}",
            z.top, z.bot, z.state
        );
    }
    Ok(())
}
