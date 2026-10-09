//! Diagnostic temporaire — pourquoi l'OB XAU M15 touché n'a pas alerté.
//! Rejoue le moteur M15 (dernier état commité par clôture) et superpose la
//! trajectoire M1 : pour chaque zone, première entrée en bande (≤ seuil × ATR)
//! et premier contact. Verdict par zone touchée dans la fenêtre.
//!
//! ```sh
//! cargo run -p api --bin diag_approche -- --asset XAUUSD --tf M15
//! ```

use std::sync::Arc;

use chrono::{DateTime, Local};
use common::{Asset, Timeframe};
use db::Database;
use smc::v12::types::ObState;
use smc::v12::{BarInput, SmcV12Engine};

fn h(ts: i64) -> String {
    DateTime::from_timestamp(ts, 0)
        .map(|d| d.with_timezone(&Local).format("%d/%m %H:%M").to_string())
        .unwrap_or_else(|| "?".into())
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let mut asset_str = "XAUUSD".to_string();
    let mut tf_str = "M15".to_string();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--asset" if i + 1 < args.len() => asset_str = args[i + 1].clone(),
            "--tf" if i + 1 < args.len() => tf_str = args[i + 1].clone(),
            _ => {}
        }
        i += 1;
    }
    let seuil = 0.25_f64;
    let db_path = std::env::var("DATABASE_PATH").unwrap_or_else(|_| "data/trading.db".to_string());
    let rt = tokio::runtime::Runtime::new()?;
    let db = Arc::new(rt.block_on(Database::new(&db_path))?);
    let asset = Asset::nouveau(&asset_str);
    let tf = Timeframe::try_from(tf_str.as_str())?;

    // 1. Replay M15 — état commité par clôture (zones + ATR + transitions).
    let bougies = rt.block_on(db.obtenir_bougies(&asset, &tf, 700))?;
    let mut moteur = SmcV12Engine::new(&asset_str, &tf_str);
    // (ts_close, atr, zones_commitées)
    let mut snapshots: Vec<(i64, f64, Vec<(bool, f64, f64, i64, ObState)>)> = Vec::new();
    for c in &bougies {
        let bar = BarInput {
            timestamp: c.timestamp.timestamp(),
            open: c.open, high: c.high, low: c.low, close: c.close, volume: c.volume,
        };
        moteur.update(&bar);
        let atr = moteur.atr.value();
        let mut zones: Vec<(bool, f64, f64, i64, ObState)> = moteur
            .order_blocks
            .bull_zones()
            .iter()
            .map(|z| (true, z.top, z.bot, z.timestamp, z.state))
            .collect();
        zones.extend(
            moteur
                .order_blocks
                .bear_zones()
                .iter()
                .map(|z| (false, z.bot, z.top, z.timestamp, z.state)),
        );
        snapshots.push((bar.timestamp, atr, zones));
    }
    let Some((last_ts, atr, zones_finales)) = snapshots.last() else { return Ok(()) };
    println!("Replay {} {}: {} barres, dernière clôture {} , ATR={atr:.2}", asset_str, tf_str, bougies.len(), h(*last_ts));
    println!("Zones actuelles (toutes) :");
    for (bull, proche, lointain, ts, state) in zones_finales {
        println!(
            "  {} [{proche:.2} ; {lointain:.2}] ts={ts} état={:?}",
            if *bull { "ACHAT " } else { "VENTE  " },
            state
        );
    }

    // 2. Trajectoire M1 sur les 3 dernières heures, contre l'état commité M15.
    let m1 = rt.block_on(db.obtenir_bougies(&asset, &Timeframe::M1, 200))?;
    let debut = chrono::Utc::now().timestamp() - 3 * 3600;
    let mut dernier_snap = 0usize;
    println!("\n— Trajectoire M1 (3 h) : événements par zone —");
    for b in &m1 {
        let ts = b.timestamp.timestamp();
        if ts < debut {
            continue;
        }
        // état commité = dernière clôture M15 STRICTEMENT avant cette M1.
        while dernier_snap + 1 < snapshots.len() && snapshots[dernier_snap + 1].0 <= ts {
            dernier_snap += 1;
        }
        let (_, atr, zones) = &snapshots[dernier_snap];
        if *atr <= 0.0 {
            continue;
        }
        for (bull, proche, _lointain, zts, state) in zones {
            // Zone VIERGE à la dernière clôture M15 ?
            if *state != ObState::Vierge {
                continue;
            }
            // Contact M1 : bull → low ≤ top ; bear → high ≥ bot.
            let touche = if *bull { b.low <= *proche } else { b.high >= *proche };
            let dist_haut = if *bull { b.high - proche } else { proche - b.low }; // meilleure approche de la barre
            let bande = dist_haut >= 0.0 && dist_haut <= seuil * atr;
            if touche || bande {
                println!(
                    "{} M1 {} OHLC {:.2}/{:.2}/{:.2}/{:.2} → zone {} ts={zts} [{}]: {}{}",
                    h(ts),
                    if *bull { "sous le prix" } else { "au-dessus" },
                    b.open, b.high, b.low, b.close,
                    if *bull { "ACHAT" } else { "VENTE" },
                    proche,
                    if touche { "CONTACT" } else { "bande" },
                    if bande && !touche { format!(" (dist {:.2} ≤ {:.2})", dist_haut, seuil * atr) } else { String::new() },
                );
            }
        }
    }

    // 3. Vie de chaque zone : création, premier contact (Vierge→touchée),
    //    sur tout le replay — pour dater les re-touches.
    println!("\n— Vie des zones (création → 1er contact) —");
    use std::collections::HashMap;
    let mut vues: HashMap<i64, (bool, f64, i64)> = HashMap::new(); // ts → (bull, bord, ts création)
    for (ts, _, zones) in &snapshots {
        for (bull, proche, _, zts, state) in zones {
            let e = vues.entry(*zts).or_insert((*bull, *proche, *ts));
            if *state != ObState::Vierge && e.2 > 0 {
                println!(
                    "  zone {} [bord {:.2}] ts={zts} : créée {}, 1er CONTACT {}",
                    if e.0 { "ACHAT" } else { "VENTE" },
                    e.1,
                    h(e.2),
                    h(*ts + 15 * 60)
                );
                e.2 = -e.2; // contact déjà daté
            }
        }
    }
    let restees: Vec<String> = vues
        .iter()
        .filter(|(_, v)| v.2 > 0)
        .map(|(_, v)| format!("{}[bord {:.2}] créée {}", if v.0 { "ACHAT" } else { "VENTE" }, v.1, h(v.2)))
        .collect();
    if !restees.is_empty() {
        println!("  jamais touchées : {}", restees.join(" ; "));
    }
    Ok(())
}
