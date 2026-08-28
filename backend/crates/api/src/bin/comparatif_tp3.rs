//! Comparatif TP3 : DoL (production Pine v12) vs 3R fixe — Module G,
//! Phase 2 AMELIORATIONS_SMC_V12.md. Le DoL-TP3 est DÉJÀ en production
//! (Pine `_bsDolTarget` + Rust `nearest_liq`, commit « décisions trading ») ;
//! cette étude mesure son apport réel contre le contre-factuel 3R fixe.
//!
//! Rejoue l'historique DB (BE = Supprimé = production) sous 2 modes :
//!   - Dol    : TP3 = liquidité la plus proche (EQH/PDH/PWH ± AsianHL v11),
//!              fallback 3R si trop proche de TP2 (production).
//!   - Fixe3R : TP3 = entry ± 3R toujours (l'ancien comportement).
//!
//! Usage : cargo run --release -p api --bin comparatif_tp3
//! Sortie : tableau console + data/comparatif_tp3.txt

use std::collections::HashMap;
use std::sync::Arc;

use common::{Asset, Timeframe};

const ASSETS: &[&str] = &["BTC", "XAUUSD", "XAGUSD", "NAS100", "SP500", "DAX"];

/// Fenêtres identiques à l'étude BE (comparabilité des échantillons).
const FENETRES: &[(&str, i64)] = &[("M15", 30_000), ("M5", 26_000), ("M1", 30_000)];

#[derive(Default, Clone)]
struct Stats {
    fermes: usize,
    r_total: f64,
    max_dd: f64,
    verdicts: HashMap<String, usize>,
}

fn main() -> anyhow::Result<()> {
    // Amorce + replay identiques à comparatif_be — voir ce binaire pour le
    // détail (les bins ne partagent pas leurs modules).
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(executer())
}

async fn executer() -> anyhow::Result<()> {
    let db_path = std::env::var("DATABASE_PATH")
        .unwrap_or_else(|_| "data/trading.db".to_string());
    let db = Arc::new(db::Database::new(&db_path).await?);
    db.run_migrations().await?;

    use smc::v12::signals::ModeTp3;
    let noms = [("DoL", ModeTp3::Dol), ("Fixe3R", ModeTp3::Fixe3R), ("DoL≤3R", ModeTp3::DolCappe3R)];

    // stats[mode][asset][tf]
    let mut stats: HashMap<&str, HashMap<String, HashMap<String, Stats>>> = HashMap::new();
    for (nom, _) in noms {
        stats.insert(nom, HashMap::new());
    }

    for asset_id in ASSETS {
        let Ok(asset) = Asset::try_from(*asset_id) else { continue };
        let amorce = charger_amorce(&db, &asset).await;

        for (tf_id, barres) in FENETRES {
            let Ok(tf) = Timeframe::try_from(*tf_id) else { continue };
            let bougies = db.obtenir_bougies(&asset, &tf, *barres).await.unwrap_or_default();
            if bougies.len() < 500 {
                eprintln!("⚠ {} {} : {} bougies (skip)", asset_id, tf_id, bougies.len());
                continue;
            }
            println!("→ {} {} ({} bougies)", asset_id, tf_id, bougies.len());

            for (nom, mode) in noms {
                let res = engine_v12::replay::rejouer_bougies_tp3(
                    asset.clone(), tf, &bougies, true, amorce.clone(), mode,
                );
                let mut s = Stats::default();
                let mut cumul = 0.0;
                let mut pic = 0.0_f64;
                for e in &res.evenements {
                    let engine::types::TypeEvenementTrade::Cloture = e.evenement else {
                        continue;
                    };
                    let mut it = e.detail.split('|');
                    let verdict = it.next().unwrap_or("?").to_string();
                    let r = it.next().and_then(|x| x.parse::<f64>().ok()).unwrap_or(0.0);
                    s.fermes += 1;
                    s.r_total += r;
                    *s.verdicts.entry(verdict).or_insert(0) += 1;
                    cumul += r;
                    pic = pic.max(cumul);
                    s.max_dd = s.max_dd.max(pic - cumul);
                }
                stats
                    .entry(nom)
                    .or_default()
                    .entry(asset_id.to_string())
                    .or_default()
                    .entry(tf_id.to_string())
                    .or_insert(s);
            }
        }
    }

    // ── Agrégation + affichage ──
    let mut lignes: Vec<String> = Vec::new();
    lignes.push("═".repeat(100).into());
    lignes.push("COMPARATIF TP3 : DoL (production) vs 3R fixe — BE = Supprimé (production)".into());
    lignes.push("R total / clôtures par asset × TF × mode".into());
    lignes.push("═".repeat(100).into());

    for asset_id in ASSETS {
        lignes.push(format!("── {} ──", asset_id));
        for tf in FENETRES.iter().map(|(t, _)| *t) {
            let mut ligne = format!("  {:<5}", tf);
            for (nom, _) in noms {
                let vide = Stats::default();
                let s = stats
                    .get(nom)
                    .and_then(|h| h.get(*asset_id))
                    .and_then(|h| h.get(tf))
                    .unwrap_or(&vide);
                ligne += &format!("  {:>22}", format!("{:+.1}R / {}", s.r_total, s.fermes));
            }
            lignes.push(ligne);
        }
    }

    lignes.push("─".repeat(100).into());
    for (nom, _) in noms {
        let mut total = Stats::default();
        for per_asset in stats.get(nom).into_iter().flatten() {
            for s in per_asset.1.values() {
                total.fermes += s.fermes;
                total.r_total += s.r_total;
                total.max_dd = total.max_dd.max(s.max_dd);
                for (v, n) in &s.verdicts {
                    *total.verdicts.entry(v.clone()).or_insert(0) += n;
                }
            }
        }
        lignes.push(format!(
            "{:<7} TOTAL {:+.1}R · {} clôtures · R moyen {:+.3} · max DD {:.1}R",
            nom,
            total.r_total,
            total.fermes,
            if total.fermes > 0 { total.r_total / total.fermes as f64 } else { 0.0 },
            total.max_dd,
        ));
        lignes.push(format!(
            "{:<7} verdicts : {}",
            "",
            total
                .verdicts
                .iter()
                .map(|(v, n)| format!("{}×{}", n, v))
                .collect::<Vec<_>>()
                .join(", "),
        ));
    }
    lignes.push("─".repeat(100).into());
    lignes.push("Lecture : DoL = TP3 liquidité (production) · Fixe3R = contre-factuel.".into());
    lignes.push("Le delta R total = apport réel du Module G sur l'échantillon.".into());

    let sortie = lignes.join("\n");
    println!("\n{}", sortie);
    std::fs::create_dir_all("data")?;
    std::fs::write("data/comparatif_tp3.txt", &sortie)?;
    println!("\n📄 Écrit : data/comparatif_tp3.txt");
    Ok(())
}

/// Amorce MTF (H1/H4/W1 + MN agrégée de D1) — copie de comparatif_be.
async fn charger_amorce(db: &db::Database, asset: &Asset) -> smc::v12::AmorceMtf {
    use smc::v12::types::BarInput;
    const MAX_BARS: i64 = 600;
    let vers_bars = |bougies: Vec<common::Candle>| -> Vec<BarInput> {
        bougies
            .into_iter()
            .map(|b| BarInput {
                timestamp: b.timestamp.timestamp(),
                open: b.open, high: b.high, low: b.low, close: b.close, volume: b.volume,
            })
            .collect()
    };
    let h1 = db.obtenir_bougies(asset, &Timeframe::H1, MAX_BARS).await.unwrap_or_default();
    let h4 = db.obtenir_bougies(asset, &Timeframe::H4, MAX_BARS).await.unwrap_or_default();
    let w1 = db.obtenir_bougies(asset, &Timeframe::W1, MAX_BARS).await.unwrap_or_default();
    let d1 = db.obtenir_bougies(asset, &Timeframe::D1, 2000).await.unwrap_or_default();
    smc::v12::AmorceMtf {
        h1: vers_bars(h1),
        h4: vers_bars(h4),
        w1: vers_bars(w1),
        mn: smc::v12::agreger_mensuel(&vers_bars(d1)),
    }
}
