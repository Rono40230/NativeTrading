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

    let db_path = std::env::var("DATABASE_PATH").unwrap_or_else(|_| "data/trading.db".to_string());
    let rt = tokio::runtime::Runtime::new()?;
    let db = rt.block_on(Database::new(&db_path))?;
    let asset = Asset::try_from(asset_str.as_str())?;
    let timeframe = Timeframe::try_from(tf_str.as_str())?;
    let bougies = rt.block_on(db.obtenir_bougies(&asset, &timeframe, limit))?;
    let db = Arc::new(db); // garde vivant jusqu'à la fin

    println!(
        "═ debug_zones {} {} — {} bougies ═",
        asset_str,
        tf_str,
        bougies.len()
    );
    let mut engine = SmcV12Engine::new(&asset_str, &tf_str);
    // Amorçage MTF identique au handler API (H1/H4/W1 + MN agrégée de D1).
    if let Some(premiere) = bougies.first() {
        let t0 = premiere.timestamp.timestamp();
        use common::Timeframe;
        use smc::v12::{agreger_mensuel, BarInput as BarMtf};
        let charger = |tf: Timeframe| -> Vec<BarMtf> {
            rt.block_on(db.obtenir_bougies(&asset, &tf, 600))
                .unwrap_or_default()
                .into_iter()
                .map(|b| BarMtf {
                    timestamp: b.timestamp.timestamp(),
                    open: b.open,
                    high: b.high,
                    low: b.low,
                    close: b.close,
                    volume: b.volume,
                })
                .collect()
        };

        let charger_d1 =
            |db: &std::sync::Arc<db::Database>, asset: &common::Asset| -> Vec<BarMtf> {
                rt.block_on(db.obtenir_bougies(asset, &Timeframe::D1, 2000))
                    .unwrap_or_default()
                    .into_iter()
                    .map(|b| BarMtf {
                        timestamp: b.timestamp.timestamp(),
                        open: b.open,
                        high: b.high,
                        low: b.low,
                        close: b.close,
                        volume: b.volume,
                    })
                    .collect()
            };

        let (h1, h4, w1) = (
            charger(Timeframe::H1),
            charger(Timeframe::H4),
            charger(Timeframe::W1),
        );
        let mn = agreger_mensuel(&charger_d1(&db, &asset));
        engine.primer_mtf(&h1, &h4, &w1, &mn, t0);
    }
    let mut dernier: Option<(i64, SmcOutput)> = None;
    // Trace zone-cœur : (dir, ob_bar) → (ts_création, ts_dernière_barre vivante).
    let mut zc_vu: std::collections::BTreeMap<(char, usize), (i64, i64)> =
        std::collections::BTreeMap::new();
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
        let ts = b.timestamp.timestamp();
        for (dir, zones) in [
            ('B', &out.zone_coeur.live_bull),
            ('S', &out.zone_coeur.live_bear),
        ] {
            for z in zones.iter() {
                let e = zc_vu.entry((dir, z.ob_bar)).or_insert((ts, ts));
                e.1 = ts;
            }
        }
        dernier = Some((ts, out));
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
        let diag = engine
            .scoring_v11
            .ob_diag(true, z.impulse_bar)
            .unwrap_or("?");
        println!(
            "  BULL {:.0}/{:.0} · état {:?} · score {score} · force {force}\n       diag: {diag}",
            z.top, z.bot, z.state
        );
    }
    for z in engine.order_blocks.bear_zones() {
        let score = engine.scoring_v11.ob_score(false, z.impulse_bar);
        let force = smc::v12::scoring_v11::ScoringV11::force(score, cal);
        let diag = engine
            .scoring_v11
            .ob_diag(false, z.impulse_bar)
            .unwrap_or("?");
        println!(
            "  BEAR {:.0}/{:.0} · état {:?} · score {score} · force {force}\n       diag: {diag}",
            z.top, z.bot, z.state
        );
    }
    println!("\n── MTF (amorçage + agrégation) ──");
    let mtf_ev = engine.mtf.last_event();
    println!(
        "  OB H1:{} H4:{} W1:{} MN:{} · confluences H1:{} H4:{} W1:{} MN:{}",
        mtf_ev.h1.bull_obs.len() + mtf_ev.h1.bear_obs.len(),
        mtf_ev.h4.bull_obs.len() + mtf_ev.h4.bear_obs.len(),
        mtf_ev.w1.bull_obs.len() + mtf_ev.w1.bear_obs.len(),
        mtf_ev.mn.bull_obs.len() + mtf_ev.mn.bear_obs.len(),
        mtf_ev.confluence_h1 as u8,
        mtf_ev.confluence_h4 as u8,
        mtf_ev.confluence_w1 as u8,
        mtf_ev.confluence_mn as u8,
    );
    // Longueurs de séries vues par replay_htf (diagnostic amorçage).
    {
        use smc::v12::BarInput as BarMtf;
        let mut serie: Vec<BarMtf> = Vec::new();
        engine.mtf.serie_mn(&mut serie);
        println!(
            "  série MN vue : {} bars (1re {:?}, dernière {:?})",
            serie.len(),
            serie.first().map(|b| b.timestamp),
            serie.last().map(|b| b.timestamp)
        );
    }
    for (nom, st) in [("W1", &mtf_ev.w1), ("MN", &mtf_ev.mn)] {
        for z in st.bull_obs.iter().chain(st.bear_obs.iter()) {
            println!("  {nom} OB {:.0}/{:.0}", z.top, z.bot);
        }
    }

    println!("\n── Zone-cœur (lifecycle live) ──");
    let fmt = |t: i64| {
        chrono::DateTime::from_timestamp(t, 0)
            .map(|d| d.format("%d/%m %H:%M").to_string())
            .unwrap_or_else(|| "?".into())
    };
    if zc_vu.is_empty() {
        println!("  (aucune zone-cœur n'a vécu pendant le replay)");
    }
    for ((dir, ob_bar), (crea, fin)) in &zc_vu {
        let vivante = *fin == dernier.as_ref().map(|(t, _)| *t).unwrap_or(0);
        println!(
            "  {} ob_bar={ob_bar} · créée {} · dernière barre vivante {}{}",
            if *dir == 'B' { "ACHAT" } else { "VENTE" },
            fmt(*crea),
            fmt(*fin),
            if vivante { "  ← ENCORE VIVANTE" } else { "" }
        );
    }
    Ok(())
}
