//! CLI du replay harness v12 (Phase 2.5 ROADMAP).
//!
//! Usage :
//! ```sh
//! cargo run -p api --bin replay_v12 -- --asset XAUUSD --tf M15 --semaines 4 [--ticks]
//! ```
//! Rejoue l'historique DB par le chemin du plugin v12, imprime le résumé
//! (parité vs référence incluse) et archive le journal complet en table
//! `runtime_replay`. Identique à `POST /api/runtime/replay`, en ligne de
//! commande — utilisable sans serveur pour la Gate 2.

use std::sync::Arc;

use common::{Asset, Timeframe};
use db::Database;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let mut asset_str = String::new();
    let mut tf_str = String::new();
    let mut semaines: i64 = 4;
    let mut simuler_ticks = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--asset" if i + 1 < args.len() => {
                asset_str = args[i + 1].clone();
                i += 1;
            }
            "--tf" | "--timeframe" if i + 1 < args.len() => {
                tf_str = args[i + 1].clone();
                i += 1;
            }
            "--semaines" if i + 1 < args.len() => {
                semaines = args[i + 1].parse().unwrap_or(4).clamp(1, 52);
                i += 1;
            }
            "--ticks" => simuler_ticks = true,
            autre => anyhow::bail!(
                "argument inconnu : {} (usage : --asset X --tf M15 --semaines N [--ticks])",
                autre
            ),
        }
        i += 1;
    }
    if asset_str.is_empty() || tf_str.is_empty() {
        anyhow::bail!("usage : replay_v12 --asset XAUUSD --tf M15 --semaines 4 [--ticks]");
    }

    let asset = Asset::try_from(asset_str.as_str())
        .map_err(|e| anyhow::anyhow!("asset inconnu '{}': {:?}", asset_str, e))?;
    let tf = Timeframe::try_from(tf_str.as_str())
        .map_err(|e| anyhow::anyhow!("timeframe inconnu '{}': {:?}", tf_str, e))?;

    let db_path = std::env::var("DATABASE_PATH").unwrap_or_else(|_| "data/trading.db".to_string());
    let db = Arc::new(Database::new(&db_path).await?);
    db.run_migrations().await?;

    // Profondeur : semaines × barres/semaine (marché 24/7 — majorant).
    let barres = semaines * 7 * 1440 / tf.minutes() as i64;
    let bougies = db.obtenir_bougies(&asset, &tf, barres).await?;
    if bougies.is_empty() {
        anyhow::bail!("aucune bougie {} {} en DB", asset_str, tf_str);
    }
    println!(
        "Replay {} {} — {} bougies ({} → {})…",
        asset_str,
        tf_str,
        bougies.len(),
        bougies[0].timestamp,
        bougies[bougies.len() - 1].timestamp
    );

    // Amorce MTF identique au handler API (H1/H4/W1 + MN agrégée de D1).
    let amorce = {
        use common::Timeframe;
        use smc::v12::{agreger_mensuel, AmorceMtf, BarInput as BarMtf};
        async fn charger(db: &db::Database, asset: &common::Asset, tf: Timeframe) -> Vec<BarMtf> {
            let bougies = db
                .obtenir_bougies(asset, &tf, 600)
                .await
                .unwrap_or_default();
            let vers_bars = |bougies: Vec<common::Candle>| -> Vec<BarMtf> {
                bougies
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
            vers_bars(bougies)
        }
        let (h1, h4) = tokio::join!(
            charger(&db, &asset, Timeframe::H1),
            charger(&db, &asset, Timeframe::H4)
        );
        let (w1, d1) = tokio::join!(
            charger(&db, &asset, Timeframe::W1),
            charger(&db, &asset, Timeframe::D1)
        );
        AmorceMtf {
            h1,
            h4,
            w1,
            mn: agreger_mensuel(&d1),
        }
    };
    let resultat = engine_v12::replay::rejouer_bougies_mode(asset, tf, &bougies, simuler_ticks, amorce, smc::v12::lifecycle::ModeBeForce::Supprime);
    println!("{}", engine_v12::replay::resume(&resultat));

    let journal = serde_json::json!({
        "signaux": resultat.signaux,
        "evenements": resultat.evenements,
    });
    let id = db
        .inserer_run_replay(
            &resultat.asset,
            &resultat.timeframe,
            resultat.simule_ticks,
            resultat.nb_bougies,
            resultat.periode_de,
            resultat.periode_a,
            resultat.signaux.len(),
            resultat.evenements.len(),
            resultat.conforme_reference,
            resultat.nb_trades_reference,
            resultat.duree_ms as u64,
            &journal,
        )
        .await?;
    println!("Run archivé : id={} (table runtime_replay)", id);
    Ok(())
}
