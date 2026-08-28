//! Comparatif BPR : bonus de scoring Module 6b ACTIF vs INACTIF — Module A,
//! Phase 3 AMELIORATIONS_SMC_V12.md.
//!
//! Le BPR (Balanced Price Range) est désormais dans l'étalon Pine (MODULE 6b :
//! détection + affichage + bonus +4/+3/+1 sur la qualification OB v11 et
//! BSZones). Cette étude mesure l'apport RÉEL du bonus contre le
//! contre-factuel « détection/affichage conservés, scoring coupé » :
//!
//! Rejoue l'historique DB (BE = Supprimé + TP3 = DoL≤3R = production) sous
//! 2 branches :
//!   - BPR ON  : bonus de scoring actif (étalon Pine, production).
//!   - BPR OFF : bonus coupé (contre-factuel — la détection tourne toujours,
//!               seul le greffon de score est neutralisé).
//!
//! Règle des 30 trades : un asset×TF dont l'une des branches ferme < 30
//! trades est signalé (échantillon insuffisant — décision non significative).
//!
//! Usage : cargo run --release -p api --bin comparatif_bpr
//! Sortie : tableau console + data/comparatif_bpr.txt

use std::collections::HashMap;
use std::sync::Arc;

use common::{Asset, Timeframe};

const ASSETS: &[&str] = &["BTC", "XAUUSD", "XAGUSD", "NAS100", "SP500", "DAX"];

/// Fenêtres identiques aux études BE/TP3 (comparabilité des échantillons).
const FENETRES: &[(&str, i64)] = &[("M15", 30_000), ("M5", 26_000), ("M1", 30_000)];

#[derive(Default, Clone)]
struct Stats {
    fermes: usize,
    r_total: f64,
    max_dd: f64,
    verdicts: HashMap<String, usize>,
}

fn main() -> anyhow::Result<()> {
    // Amorce + replay identiques à comparatif_tp3 — voir ce binaire pour le
    // détail (les bins ne partagent pas leurs modules).
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(executer())
}

async fn executer() -> anyhow::Result<()> {
    let db_path = std::env::var("DATABASE_PATH")
        .unwrap_or_else(|_| "data/trading.db".to_string());
    let db = Arc::new(db::Database::new(&db_path).await?);
    db.run_migrations().await?;

    let noms: [(&str, bool); 2] = [("BPR ON", true), ("BPR OFF", false)];

    // stats[branche][asset][tf]
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

            for (nom, scoring_bpr) in noms {
                let res = engine_v12::replay::rejouer_bougies_bpr(
                    asset.clone(), tf, &bougies, true, amorce.clone(), scoring_bpr,
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
    lignes.push("COMPARATIF BPR : bonus Module 6b ON vs OFF — BE=Supprimé · TP3=DoL≤3R (production)".into());
    lignes.push("R total / clôtures par asset × TF × branche".into());
    lignes.push("═".repeat(100).into());

    for asset_id in ASSETS {
        lignes.push(format!("── {} ──", asset_id));
        for tf in FENETRES.iter().map(|(t, _)| *t) {
            let mut ligne = format!("  {:<5}", tf);
            let mut faible = false;
            for (nom, _) in noms {
                let vide = Stats::default();
                let s = stats
                    .get(nom)
                    .and_then(|h| h.get(*asset_id))
                    .and_then(|h| h.get(tf))
                    .unwrap_or(&vide);
                if s.fermes < 30 {
                    faible = true;
                }
                ligne += &format!("  {:>22}", format!("{:+.1}R / {}", s.r_total, s.fermes));
            }
            if faible {
                ligne += "  ⚠ <30 trades";
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
            "{:<8} TOTAL {:+.1}R · {} clôtures · R moyen {:+.3} · max DD {:.1}R",
            nom,
            total.r_total,
            total.fermes,
            if total.fermes > 0 { total.r_total / total.fermes as f64 } else { 0.0 },
            total.max_dd,
        ));
        lignes.push(format!(
            "{:<8} verdicts : {}",
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
    lignes.push("Lecture : le delta R total (ON − OFF) = apport réel du bonus BPR.".into());
    lignes.push("Si ≤ 0 : le BPR reste affiché mais le scoring est retiré (règle replay).".into());

    let sortie = lignes.join("\n");
    println!("\n{}", sortie);
    std::fs::create_dir_all("data")?;
    std::fs::write("data/comparatif_bpr.txt", &sortie)?;
    println!("\n📄 Écrit : data/comparatif_bpr.txt");
    Ok(())
}

/// Amorce MTF (H1/H4/W1 + MN agrégée de D1) — copie de comparatif_tp3.
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
