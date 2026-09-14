//! Étude B (14/09) — calibration de la FORCE des Order Blocks v12 par
//! classe d'actifs. Mesure avant décision : ce binaire rejoue le moteur v12
//! sur l'historique DB (M15 par défaut) et dresse la distribution des
//! scores maximaux atteints par chaque OB née, puis traduit cette
//! distribution en taux de forces pour plusieurs profils de seuils.
//!
//! La force affichée (et la qualification v11 « force ≥ 4 » des trades)
//! vient de `ScoringV11::force` : paliers seuil_moyen/seuil_fort/
//! seuil_instit/score_max — calibrés historiquement pour
//! XAU/XAG/BTC/DAX/NAS/SP500, profil « défaut » (7/99/99/13) pour tout le
//! reste : plafond ~5.
//!
//! Usage :
//! ```sh
//! cargo run -p api --bin etude_calibration_smc -- --assets BTC,XAUUSD --tf M15 [--barres 20000]
//! ```

use std::collections::HashMap;
use std::sync::Arc;

use common::{Asset, Timeframe};
use db::Database;
use smc::v12::lifecycle::ModeBeForce;
use smc::v12::{agreger_mensuel, BarInput, SmcV12Engine};

/// Seuils HISTORIQUES des 6 actifs calibrés (miroir de calibration.rs) —
/// servent de référence : le taux d'OB « visibles » (force ≥ 5) de ces
/// profils définit la cible à atteindre pour les nouvelles classes.
const SEUILS_HISTORIQUES: &[(&str, i32, i32, i32, i32)] = &[
    ("XAUUSD", 7, 10, 12, 13),
    ("XAGUSD", 7, 99, 99, 14),
    ("BTC", 8, 99, 99, 15),
    ("DAX", 11, 16, 19, 21),
    ("NAS100", 10, 15, 17, 19),
    ("SP500", 10, 15, 17, 19),
];

fn force10(s: i32, seuil_moyen: i32, seuil_fort: i32, seuil_instit: i32, score_max: i32) -> i32 {
    let (sm, sf, si, sx) = (
        seuil_moyen as f64,
        seuil_fort as f64,
        seuil_instit as f64,
        score_max as f64,
    );
    let x = s as f64;
    let f = if x < sm {
        1.0 + 3.0 * x / sm.max(1.0)
    } else if x < sf {
        5.0 + (x - sm) / (sf - sm).max(1.0)
    } else if x < si {
        7.0 + (x - sf) / (si - sf).max(1.0)
    } else {
        9.0 + (x - si) / (sx - si).max(1.0)
    };
    (f.round() as i32).clamp(1, 10)
}

fn percentile(tries: &[i32], p: f64) -> i32 {
    if tries.is_empty() {
        return 0;
    }
    let idx = ((tries.len() as f64 - 1.0) * p).round() as usize;
    tries[idx.min(tries.len() - 1)]
}

async fn amorce_mtf(db: &Database, asset: &Asset) -> (Vec<BarInput>, Vec<BarInput>, Vec<BarInput>, Vec<BarInput>) {
    const MAX_BARS: i64 = 600;
    async fn charger(db: &Database, asset: &Asset, tf: Timeframe, max: i64) -> Vec<BarInput> {
        db.obtenir_bougies(asset, &tf, max)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|b| BarInput {
                timestamp: b.timestamp.timestamp(),
                open: b.open,
                high: b.high,
                low: b.low,
                close: b.close,
                volume: b.volume,
            })
            .collect()
    }
    let h1 = charger(db, asset, Timeframe::H1, MAX_BARS).await;
    let h4 = charger(db, asset, Timeframe::H4, MAX_BARS).await;
    let w1 = charger(db, asset, Timeframe::W1, MAX_BARS).await;
    let d1 = charger(db, asset, Timeframe::D1, 2000).await;
    (h1, h4, w1, agreger_mensuel(&d1))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let mut assets_str = String::new();
    let mut tf_str = "M15".to_string();
    let mut barres: i64 = 20_000;
    let mut cal_str = String::new();

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--assets" if i + 1 < args.len() => {
                assets_str = args[i + 1].clone();
                i += 1;
            }
            "--cal" if i + 1 < args.len() => {
                cal_str = args[i + 1].clone();
                i += 1;
            }
            "--tf" if i + 1 < args.len() => {
                tf_str = args[i + 1].clone();
                i += 1;
            }
            "--barres" if i + 1 < args.len() => {
                barres = args[i + 1].parse().unwrap_or(20_000).clamp(50, 200_000);
                i += 1;
            }
            autre => anyhow::bail!(
                "argument inconnu : {} (usage : --assets A,B --tf M15 --barres N)",
                autre
            ),
        }
        i += 1;
    }
    if assets_str.is_empty() {
        anyhow::bail!("usage : etude_calibration_smc --assets BTC,ETH,... --tf M15 --barres N");
    }
    let tf = Timeframe::try_from(tf_str.as_str())
        .map_err(|e| anyhow::anyhow!("timeframe inconnu '{}': {:?}", tf_str, e))?;

    let db_path = std::env::var("DATABASE_PATH").unwrap_or_else(|_| "data/trading.db".to_string());
    let db = Arc::new(Database::new(&db_path).await?);
    db.run_migrations().await?;

    for asset_str in assets_str.split(',') {
        let asset = Asset::try_from(asset_str)
            .map_err(|e| anyhow::anyhow!("asset inconnu '{}': {:?}", asset_str, e))?;
        let bougies = db.obtenir_bougies(&asset, &tf, barres).await?;
        if bougies.len() < 100 {
            println!("== {} {} : {} bougies seulement — ignoré", asset_str, tf_str, bougies.len());
            continue;
        }
        let (h1, h4, w1, mn) = amorce_mtf(&db, &asset).await;
        // --cal : rejouer les données de l'asset AVEC la calibration d'un
        // autre (ex. --cal BTC pour mesurer les alt-cryptos en profil BTC).
        let cal_asset = if cal_str.is_empty() { asset_str } else { &cal_str };
        let mut engine = SmcV12Engine::new(cal_asset, &tf_str)
            .avec_mode_be_force(ModeBeForce::Supprime);
        if let Some(premiere) = bougies.first() {
            engine.primer_mtf(&h1, &h4, &w1, &mn, premiere.timestamp.timestamp());
        }

        // Score MAX atteint par chaque OB née (celui qui fixe sa force).
        let mut scores: HashMap<(bool, usize), i32> = HashMap::new();
        for b in &bougies {
            let bar = BarInput {
                timestamp: b.timestamp.timestamp(),
                open: b.open,
                high: b.high,
                low: b.low,
                close: b.close,
                volume: b.volume,
            };
            engine.update(&bar);
            for z in engine.order_blocks.bull_zones() {
                let s = engine.scoring_v11.ob_score(true, z.impulse_bar);
                scores
                    .entry((true, z.impulse_bar))
                    .and_modify(|e| *e = (*e).max(s))
                    .or_insert(s);
            }
            for z in engine.order_blocks.bear_zones() {
                let s = engine.scoring_v11.ob_score(false, z.impulse_bar);
                scores
                    .entry((false, z.impulse_bar))
                    .and_modify(|e| *e = (*e).max(s))
                    .or_insert(s);
            }
        }

        let mut tries: Vec<i32> = scores.values().copied().collect();
        tries.sort_unstable();
        let nb = tries.len();
        println!(
            "\n== {} {} ({} bougies, {} → {}) ==",
            asset_str,
            tf_str,
            bougies.len(),
            bougies[0].timestamp.format("%Y-%m-%d"),
            bougies[bougies.len() - 1].timestamp.format("%Y-%m-%d"),
        );
        if nb == 0 {
            println!("aucune OB née sur la fenêtre");
            continue;
        }
        println!(
            "OB nées : {} | score max/zone : min {} · p25 {} · p50 {} · p75 {} · p90 {} · p95 {} · p99 {} · max {}",
            nb,
            tries[0],
            percentile(&tries, 0.25),
            percentile(&tries, 0.50),
            percentile(&tries, 0.75),
            percentile(&tries, 0.90),
            percentile(&tries, 0.95),
            percentile(&tries, 0.99),
            tries[nb - 1],
        );

        // Taux de forces selon les profils : historique de l'asset (si
        // calibré), défaut actuel, puis candidats par percentiles.
        let mut profils: Vec<(String, i32, i32, i32, i32)> = vec![
            ("défaut actuel (7/99/99/13)".into(), 7, 99, 99, 13),
        ];
        if let Some((a, sm, sf, si, sx)) = SEUILS_HISTORIQUES.iter().find(|t| t.0 == asset_str) {
            profils.insert(0, (format!("historique {} ({}/{}/{}/{})", a, sm, sf, si, sx), *sm, *sf, *si, *sx));
        }
        let (p50, p75, p90, p95) = (
            percentile(&tries, 0.50),
            percentile(&tries, 0.75),
            percentile(&tries, 0.90),
            percentile(&tries, 0.95),
        );
        if p50 >= 1 {
            profils.push((format!("candidat P50 ({}/{}/{}/{})", p50, p75, p90, tries[nb - 1]), p50, p75, p90, tries[nb - 1]));
            if p95 > p90 {
                profils.push((format!("candidat resserré ({}/{}/{}/{})", p50, p75, p95, tries[nb - 1]), p50, p75, p95, tries[nb - 1]));
            }
        }
        for (nom, sm, sf, si, sx) in &profils {
            let forces: Vec<i32> = tries.iter().map(|s| force10(*s, *sm, *sf, *si, *sx)).collect();
            let ge4 = forces.iter().filter(|f| **f >= 4).count();
            let ge5 = forces.iter().filter(|f| **f >= 5).count();
            let ge7 = forces.iter().filter(|f| **f >= 7).count();
            println!(
                "  {:<42} force≥4 {:>5.1}% · ≥5 {:>5.1}% · ≥7 {:>5.1}%",
                nom,
                100.0 * ge4 as f64 / nb as f64,
                100.0 * ge5 as f64 / nb as f64,
                100.0 * ge7 as f64 / nb as f64,
            );
        }
    }
    Ok(())
}
