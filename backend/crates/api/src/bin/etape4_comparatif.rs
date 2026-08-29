//! Étape 4 — calcul des trades : SL / TP / BE automatique.
//! Objectifs propriétaire : (1) SL au mieux par paire avec maximums,
//! (2) TP au mieux par paire, (3) BE auto si le prix atteint X% du chemin
//! de TP1 sans le toucher.
//!
//! 7 branches A/B (production = référence) :
//!   - production : SL offset _autoSlMode (clamps [slMin,slMax] conservés
//!     dans TOUTES les branches = maximums à ne pas dépasser), TP1=1R,
//!     TP2=2R, TP3=DoL≤3R, BE uniquement à TP1 touché ;
//!   - SL×0.75 / SL×1.25 : offset SL resserré/élargi (clamps inchangés) ;
//!   - TP1=0.8R : premier objectif plus proche ;
//!   - TP2=2.5R : second objectif plus loin ;
//!   - BEauto50 / BEauto70 : BE armé à 50%/70% du chemin de TP1 (MFE),
//!     passage SL→entry au retour à l'entrée.
//!
//! Usage : cargo run --release -p api --bin etape4_comparatif
//! Sortie : console + data/etape4_affinage.txt

use std::collections::HashMap;
use std::sync::Arc;

use common::{Asset, Timeframe};

const ASSETS: &[&str] = &["BTC", "XAUUSD", "XAGUSD", "NAS100", "SP500", "DAX"];
const FENETRES: &[(&str, i64)] = &[("M15", 30_000), ("M5", 26_000), ("M1", 30_000)];

#[derive(Default, Clone)]
struct Stats {
    fermes: usize,
    r_total: f64,
    max_dd: f64,
    verdicts: HashMap<String, usize>,
}

/// Une branche d'étude : libellé + mutation de ModesEtude.
struct Branche {
    nom: &'static str,
    sl: f64,
    tp1: f64,
    tp2: f64,
    be_auto: Option<f64>,
}

fn branches() -> Vec<Branche> {
    // Passe finale (29/08) : recherche du retournement du gradient TP1
    // (0.6R+SL0.75 = +1029.9R, gradient encore descendant à 0.6).
    vec![
        Branche { nom: "production", sl: 1.0, tp1: 1.0, tp2: 2.0, be_auto: None },
        Branche { nom: "0.6R+SL0.75", sl: 0.75, tp1: 0.6, tp2: 2.0, be_auto: None },
        Branche { nom: "0.5R+SL0.75", sl: 0.75, tp1: 0.5, tp2: 2.0, be_auto: None },
        Branche { nom: "0.4R+SL0.75", sl: 0.75, tp1: 0.4, tp2: 2.0, be_auto: None },
        Branche { nom: "0.5R+SL0.65", sl: 0.65, tp1: 0.5, tp2: 2.0, be_auto: None },
        Branche { nom: "0.6R+SL0.65", sl: 0.65, tp1: 0.6, tp2: 2.0, be_auto: None },
    ]}

fn main() -> anyhow::Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(executer())
}

async fn executer() -> anyhow::Result<()> {
    let db_path = std::env::var("DATABASE_PATH")
        .unwrap_or_else(|_| "data/trading.db".to_string());
    let db = Arc::new(db::Database::new(&db_path).await?);
    db.run_migrations().await?;

    let brs = branches();
    // stats[branche][asset][tf]
    let mut stats: HashMap<&str, HashMap<String, HashMap<String, Stats>>> = HashMap::new();
    for b in &brs {
        stats.insert(b.nom, HashMap::new());
    }

    for asset_id in ASSETS {
        let Ok(asset) = Asset::try_from(*asset_id) else { continue };
        let amorce = charger_amorce(&db, &asset).await;

        for (tf_id, barres) in FENETRES {
            let Ok(tf) = Timeframe::try_from(*tf_id) else { continue };
            let bougies = db
                .obtenir_bougies(&asset, &tf, *barres)
                .await
                .unwrap_or_default();
            if bougies.len() < 500 {
                eprintln!("⚠ {} {} : {} bougies (skip)", asset_id, tf_id, bougies.len());
                continue;
            }
            println!("→ {} {} ({} bougies)", asset_id, tf_id, bougies.len());

            for b in &brs {
                let modes = engine_v12::replay::ModesEtude {
                    sl_mult: b.sl,
                    tp1_mult: b.tp1,
                    tp2_mult: b.tp2,
                    be_auto: b.be_auto,
                    ..modes_production()
                };
                let res = engine_v12::replay::rejouer_bougies_modes(
                    asset.clone(), tf, &bougies, true, amorce.clone(), &modes,
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
                    .entry(b.nom)
                    .or_default()
                    .entry(asset_id.to_string())
                    .or_default()
                    .entry(tf_id.to_string())
                    .or_insert(s);
            }
        }
    }

    // ── Sortie ──
    let mut lignes: Vec<String> = Vec::new();
    lignes.push("═".repeat(120).into());
    lignes.push("ÉTAPE 4 — CALCUL DES TRADES : SL (offset, clamps conservés) · TP1/TP2 · BE auto (MFE sans TP1)".into());
    lignes.push("R total / clôtures par asset × TF × branche".into());
    lignes.push("═".repeat(120).into());

    for asset_id in ASSETS {
        lignes.push(format!("── {} ──", asset_id));
        for tf in FENETRES.iter().map(|(t, _)| *t) {
            let mut ligne = format!("  {:<5}", tf);
            for b in &brs {
                let vide = Stats::default();
                let s = stats
                    .get(b.nom)
                    .and_then(|h| h.get(*asset_id))
                    .and_then(|h| h.get(tf))
                    .unwrap_or(&vide);
                ligne += &format!("  {:>18}", format!("{:+.1}R/{}", s.r_total, s.fermes));
            }
            lignes.push(ligne);
        }
    }

    lignes.push("─".repeat(120).into());
    for b in &brs {
        let mut total = Stats::default();
        for per_asset in stats.get(b.nom).into_iter().flatten() {
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
            "{:<11} TOTAL {:+.1}R · {} clôtures · R moyen {:+.3} · max DD {:.1}R · verdicts : {}",
            b.nom,
            total.r_total,
            total.fermes,
            if total.fermes > 0 { total.r_total / total.fermes as f64 } else { 0.0 },
            total.max_dd,
            total
                .verdicts
                .iter()
                .map(|(v, n)| format!("{}×{}", n, v))
                .collect::<Vec<_>>()
                .join(", "),
        ));
    }
    lignes.push("─".repeat(120).into());
    lignes.push("Lecture : delta R total vs production = apport de chaque variante (règle 30 trades par cellule).".into());

    let sortie = lignes.join("\n");
    println!("\n{}", sortie);
    std::fs::create_dir_all("data")?;
    std::fs::write("data/etape4_retenue.txt", &sortie)?;
    println!("\n📄 Écrit : data/etape4_retenue.txt");
    Ok(())
}

/// Base production (BE Supprime + TP3 DoL≤3R) — miroir de l'interne replay.
fn modes_production() -> engine_v12::replay::ModesEtude {
    engine_v12::replay::ModesEtude {
        be: smc::v12::lifecycle::ModeBeForce::Supprime,
        ..Default::default()
    }
}

/// Amorce MTF (H1/H4/W1 + MN agrégée de D1) — copie des binaires d'étude.
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
