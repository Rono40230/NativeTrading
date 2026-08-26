//! Comparatif des variantes du BE forcé sur BOS opposé — décision par les
//! chiffres (méthode Gate, 26/08). Constat : 20 BE contre 1 TP2 sur les
//! trades SMC remplis (~95 %) — la règle Pine v12 étrangle la stratégie
//! sur M1/M5 où les micro-BOS sont du bruit.
//!
//! Rejoue l'historique DB (amorce MTF identique au live, ticks intrabar
//! simulés) sous 4 modes :
//!   - Classique : SL → entrée au BOS opposé brut (production Pine v12)
//!   - Marche    : clôture immédiate au prix du tick du BOS opposé
//!   - Supprime  : aucune action — le trade vit jusqu'à SL/TP
//!   - Qualifie  : BE uniquement sur MSS opposé (displacement)
//!
//! Usage : cargo run --release -p api --bin comparatif_be
//! Sortie : tableau console + data/comparatif_be.txt

use std::collections::HashMap;
use std::sync::Arc;

use common::{Asset, Timeframe};

const ASSETS: &[&str] = &["BTC", "XAUUSD", "XAGUSD", "NAS100", "SP500", "DAX"];

/// Fenêtres : le problème étant M1/M5, l'échantillon y est large ; M15 en
/// profondeur pour la stabilité. (barres depuis la DB)
const FENETRES: &[(&str, i64)] = &[("M15", 30_000), ("M5", 26_000), ("M1", 30_000)];

#[derive(Default, Clone)]
struct Stats {
    fermes: usize,
    /// Clôtures de trades REMPLIS (avec Fill) — les seules qui comptent.
    remplis: usize,
    r_total: f64,
    r_moyen: f64,
    wr: f64,
    max_dd: f64,
    verdicts: HashMap<String, usize>,
}

fn nom_mode(m: smc::v12::lifecycle::ModeBeForce) -> &'static str {
    use smc::v12::lifecycle::ModeBeForce::*;
    match m {
        Classique => "Classique",
        Marche => "Marché",
        Supprime => "Supprimé",
        Qualifie => "Qualifié",
    }
}

/// Amorce MTF (H1/H4/W1 + MN agrégée de D1) — même logique que le
/// binaire principal (`smc_v12_handlers::charger_amorce_mtf`), les bins ne
/// partageant pas ses modules.
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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let db_path = std::env::var("DATABASE_PATH")
        .unwrap_or_else(|_| "data/trading.db".to_string());
    let db = Arc::new(db::Database::new(&db_path).await?);
    db.run_migrations().await?;

    use smc::v12::lifecycle::ModeBeForce;
    let modes = [ModeBeForce::Classique, ModeBeForce::Marche, ModeBeForce::Supprime, ModeBeForce::Qualifie];

    // stats[mode][(tf)] et stats[mode]["TOTAL"]
    let mut stats: HashMap<String, HashMap<String, Stats>> = HashMap::new();
    for m in modes {
        stats.insert(nom_mode(m).to_string(), HashMap::new());
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

            for mode in modes {
                let res = engine_v12::replay::rejouer_bougies_mode(
                    asset.clone(), tf, &bougies, true, amorce.clone(), mode,
                );
                // Joindre les événements par trade : Fill = rempli, Cloture
                // « verdict|R » = issue.
                let mut remplies: std::collections::HashSet<String> = std::collections::HashSet::new();
                let mut clotures: Vec<(String, f64)> = Vec::new();
                for e in &res.evenements {
                    use engine::types::TypeEvenementTrade as T;
                    match e.evenement {
                        T::Fill => { remplies.insert(e.cle_trade.clone()); }
                        T::Cloture => {
                            let mut it = e.detail.split('|');
                            let verdict = it.next().unwrap_or("?").to_string();
                            let r = it.next().and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
                            clotures.push((verdict, r));
                        }
                        _ => {}
                    }
                }
                let _ = &remplies; // toutes les clôtures du replay viennent de trades ouverts (remplis ou annulés) — on compte tout ce qui a un Fill
                let cle_mode = nom_mode(mode).to_string();
                let s = stats
                    .entry(cle_mode)
                    .or_default()
                    .entry(tf_id.to_string())
                    .or_default();
                s.fermes += clotures.len();
                let mut cumul = 0.0;
                let mut pic = 0.0_f64;
                for (verdict, r) in clotures {
                    s.remplis += 1;
                    s.r_total += r;
                    if r > 0.0 { /* wr compté après */ }
                    *s.verdicts.entry(verdict).or_insert(0) += 1;
                    cumul += r;
                    pic = pic.max(cumul);
                    s.max_dd = s.max_dd.max(pic - cumul);
                }
            }
        }
    }

    // ── Agrégation finale + affichage ──
    let mut lignes: Vec<String> = Vec::new();
    lignes.push("═".repeat(96).into());
    lignes.push("COMPARATIF BE FORCÉ (BOS opposé) — R total / trades remplis par mode × TF".into());
    lignes.push("═".repeat(96).into());
    let entete = format!("{:<12}", "Mode");
    let mut tfs: Vec<&str> = FENETRES.iter().map(|(t, _)| *t).collect();
    tfs.push("TOTAL");
    let entete = entete + &tfs.iter().map(|t| format!("{:>18}", t)).collect::<String>();
    lignes.push(entete);
    lignes.push("─".repeat(96).into());

    // Totaux par mode calculés AVANT l'impression (colonne TOTAL remplie).
    for m in modes {
        let nom = nom_mode(m);
        let mut total = Stats::default();
        for (tf, s) in stats.get(nom).into_iter().flatten() {
            if tf == "TOTAL" { continue; }
            total.fermes += s.fermes;
            total.remplis += s.remplis;
            total.r_total += s.r_total;
            total.max_dd = total.max_dd.max(s.max_dd);
            for (v, n) in &s.verdicts { *total.verdicts.entry(v.clone()).or_insert(0) += n; }
        }
        if let Some(h) = stats.get_mut(nom) { h.insert("TOTAL".into(), total); }
    }

    for m in modes {
        let nom = nom_mode(m);
        let mut ligne = format!("{:<12}", nom);
        for tf in &tfs {
            let vide = Stats::default();
            let s = stats.get(nom).and_then(|h| h.get(*tf)).unwrap_or(&vide);
            ligne += &format!("{:>18}", format!("{:+.1}R / {}", s.r_total, s.remplis));
        }
        lignes.push(ligne);
        // Détail du mode (total relu depuis la map).
        let vide = Stats::default();
        let total = stats.get(nom).and_then(|h| h.get("TOTAL")).unwrap_or(&vide);
        lignes.push(format!(
            "{:<12}  R moyen {:+.3} · max DD {:.1}R · verdicts : {}",
            "",
            if total.remplis > 0 { total.r_total / total.remplis as f64 } else { 0.0 },
            total.max_dd,
            total.verdicts.iter().map(|(v, n)| format!("{}×{}", n, v)).collect::<Vec<_>>().join(", "),
        ));
    }
    lignes.push("─".repeat(96).into());
    lignes.push("Lecture : R total = somme des R réalisés · remplis = clôtures de trades".into());
    lignes.push("ayant vu un Fill · Marché = R au prix du BOS (partiel, souvent négatif).".into());

    let sortie = lignes.join("\n");
    println!("\n{}", sortie);
    std::fs::create_dir_all("data")?;
    std::fs::write("data/comparatif_be.txt", &sortie)?;
    println!("\n📄 Écrit : data/comparatif_be.txt");
    Ok(())
}
