//! Étude SL fixe (owner 19/10, doctrine trading manuel) — labo sur
//! historique. Pour un couple asset×TF : replay baseline production puis
//! candidats SL fixe (base absolue × paliers score × garde de zone).
//! Agrégats JSON sur stdout (une ligne par config).
//!
//! Usage :
//!   etude_sl_fixe --asset XAUUSD --tf M15 [--bases 6,8,10]
//!                 [--gardes aucune,skip,serre] [--paliers 0.7,1.3]
//!                 [--semaines 104] [--proche 3]
//!
//! `--bases` en unités prix absolues ; par défaut = R médian du baseline
//! × {0,6 ; 1,0 ; 1,4}. `--proche` : fenêtre barres pour le « proximité »
//! du score (non utilisé ici, réservé).

use std::collections::HashMap;
use std::sync::Arc;

use common::{Asset, Candle, Timeframe};
use engine::TypeEvenementTrade as T;
use smc::v12::signals::{GardeSlFixe, SlFixeEtude};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut asset_str = String::new();
    let mut tf_str = String::new();
    let mut bases: Vec<f64> = Vec::new();
    let mut gardes = vec!["aucune", "skip", "serre"];
    let mut mf = 0.7_f64;
    let mut mfort = 1.3_f64;
    let mut semaines = 104_i64;
    let args: Vec<String> = std::env::args().collect();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--asset" if i + 1 < args.len() => asset_str = args[i + 1].clone(),
            "--tf" if i + 1 < args.len() => tf_str = args[i + 1].clone(),
            "--bases" if i + 1 < args.len() => {
                bases = args[i + 1].split(',').filter_map(|s| s.trim().parse().ok()).collect()
            }
            "--gardes" if i + 1 < args.len() => {
                gardes = args[i + 1].split(',').map(|s| s.trim()).collect()
            }
            "--paliers" if i + 1 < args.len() => {
                let p: Vec<f64> =
                    args[i + 1].split(',').filter_map(|s| s.trim().parse().ok()).collect();
                if p.len() == 2 {
                    mf = p[0];
                    mfort = p[1];
                }
            }
            "--semaines" if i + 1 < args.len() => {
                semaines = args[i + 1].parse().unwrap_or(104).clamp(4, 120)
            }
            autre => anyhow::bail!("argument inconnu : {autre}"),
        }
        i += 2;
    }
    if asset_str.is_empty() || tf_str.is_empty() {
        anyhow::bail!("usage : etude_sl_fixe --asset XAUUSD --tf M15 [--bases ..] [--gardes ..] [--paliers f,fort] [--semaines N]");
    }
    let asset = Asset::try_from(asset_str.as_str())?;
    let tf = Timeframe::try_from(tf_str.as_str())?;

    let db_path =
        std::env::var("DATABASE_PATH").unwrap_or_else(|_| "data/trading.db".to_string());
    let db = Arc::new(db::Database::new(&db_path).await?);
    db.run_migrations().await?;

    let barres = semaines * 7 * 1440 / tf.minutes() as i64;
    let bougies = db.obtenir_bougies(&asset, &tf, barres).await?;
    if bougies.len() < 500 {
        anyhow::bail!("historique trop court ({} bougies)", bougies.len());
    }
    let amorce = amorce_mtf(&db, &asset).await;
    eprintln!(
        "{} {} : {} bougies ({} → {})",
        asset_str,
        tf_str,
        bougies.len(),
        bougies[0].timestamp,
        bougies[bougies.len() - 1].timestamp
    );

    let mut modes = engine_v12::replay::modes_production();
    // (Plus de baseline ATR : chemin supprimé du moteur le 19/10 — les
    // chiffres « avant » vivent dans le rapport d'étude initial.)
    let ligne = |nom: &str, s: &Stats| {
        serde_json::json!({
            "config": nom, "asset": asset_str, "tf": tf_str,
            "n": s.n, "r_total": (s.r_total * 100.0).round() / 100.0,
            "r_par_trade": (s.r_moyen * 1000.0).round() / 1000.0,
            "wr": (s.wr * 1000.0).round() / 1000.0,
            "expire": s.expire, "sl": s.sl, "tp": s.tp,
            "r_faible": (s.r_faible * 100.0).round() / 100.0,
            "r_moyen_palier": (s.r_moyen_palier * 100.0).round() / 100.0,
            "r_fort": (s.r_fort * 100.0).round() / 100.0,
        })
        .to_string()
    };
    if bases.is_empty() {
        anyhow::bail!("--bases requis (ex. --bases 5,7,10 — distances prix absolues)");
    }

    // ── Balayage ───────────────────────────────────────────────────────
    for base in &bases {
        if *base <= 0.0 {
            continue;
        }
        for g in &gardes {
            let garde = match *g {
                "skip" => GardeSlFixe::SkipZoneLarge,
                "serre" => GardeSlFixe::SerreBord,
                _ => GardeSlFixe::Aucune,
            };
            modes.sl_fixe = Some(SlFixeEtude {
                base: *base,
                mult_faible: mf,
                mult_fort: mfort,
                garde,
            });
            let res = engine_v12::replay::rejouer_bougies_modes(
                asset.clone(),
                tf,
                &bougies,
                false,
                amorce.clone(),
                &modes,
            );
            let (stats, _) = agreger(&res);
            println!("{}", ligne(&format!("fixe_{base:.2}_{g}"), &stats));
        }
    }
    Ok(())
}

// ── Agrégation ──────────────────────────────────────────────────────────

struct Stats {
    n: usize,
    r_total: f64,
    r_moyen: f64,
    wr: f64,
    expire: usize,
    sl: usize,
    tp: usize,
    r_faible: f64,
    r_moyen_palier: f64,
    r_fort: f64,
}

/// Clôtures des signaux confirmés + R médian des signaux (distance SL).
fn agreger(res: &engine_v12::replay::ResultatReplay) -> (Stats, f64) {
    let scores: HashMap<&str, i32> = res
        .signaux
        .iter()
        .filter(|s| !s.annonce)
        .map(|s| (s.cle.as_str(), s.score))
        .collect();
    let mut distances: Vec<f64> = res
        .signaux
        .iter()
        .filter(|s| !s.annonce)
        .map(|s| (s.prix_entree - s.stop_loss).abs())
        .filter(|d| *d > 0.0)
        .collect();
    distances.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let r_median = if distances.is_empty() {
        0.0
    } else {
        distances[distances.len() / 2]
    };

    let (mut n, mut r_total, mut expire, mut sl, mut tp) = (0_usize, 0.0, 0_usize, 0_usize, 0_usize);
    let (mut gagnants, mut denominateur) = (0_usize, 0_usize);
    let (mut r_faible, mut r_moyen, mut r_fort) = (0.0, 0.0, 0.0);
    for e in &res.evenements {
        if !matches!(e.evenement, T::Cloture) {
            continue;
        }
        let verdict = e.detail.split('|').next().unwrap_or("Expire");
        let v = verdict.to_lowercase();
        let r = e.detail.split('|').nth(1).and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
        n += 1;
        r_total += r;
        match v.as_str() {
            "expire" | "cancel" => expire += 1,
            "sl" => sl += 1,
            "tp1" | "tp2" | "tp3" | "ts" | "be" => tp += 1,
            _ => {}
        }
        if v != "expire" && v != "cancel" {
            denominateur += 1;
            if r > 0.0 {
                gagnants += 1;
            }
        }
        match scores.get(e.cle_trade.as_str()).copied().unwrap_or(10) {
            s if s < 8 => r_faible += r,
            s if s >= 12 => r_fort += r,
            _ => r_moyen += r,
        }
    }
    (
        Stats {
            n,
            r_total,
            r_moyen: if n > 0 { r_total / n as f64 } else { 0.0 },
            wr: if denominateur > 0 { gagnants as f64 / denominateur as f64 } else { 0.0 },
            expire,
            sl,
            tp,
            r_faible,
            r_moyen_palier: r_moyen,
            r_fort,
        },
        r_median,
    )
}

// ── Amorce MTF (même construction que replay_v12) ──────────────────────

async fn amorce_mtf(db: &db::Database, asset: &Asset) -> smc::v12::AmorceMtf {
    use smc::v12::{agreger_mensuel, AmorceMtf, BarInput as BarMtf};
    async fn charger(db: &db::Database, asset: &Asset, tf: Timeframe) -> Vec<BarMtf> {
        let bougies: Vec<Candle> =
            db.obtenir_bougies(asset, &tf, 600).await.unwrap_or_default();
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
    }
    let d1 = charger(db, asset, Timeframe::try_from("D1").ok().unwrap_or(Timeframe::H4)).await;
    AmorceMtf {
        h1: charger(db, asset, Timeframe::try_from("H1").ok().unwrap_or(Timeframe::M30)).await,
        h4: charger(db, asset, Timeframe::try_from("H4").ok().unwrap_or(Timeframe::H1)).await,
        w1: charger(db, asset, Timeframe::try_from("W1").ok().unwrap_or(Timeframe::D1)).await,
        mn: agreger_mensuel(&d1),
    }
}
