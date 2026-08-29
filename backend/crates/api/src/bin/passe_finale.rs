//! Passe finale étape 3 — R4, R5, R6, R7, R8 en une exécution
//! (docs/ETAPE3_VERIFICATION_SCORING_LOGIQUE_ENTREES.md).
//!
//! - **R4** (A/B) : confluences MTF sur HTF CLÔTURÉ seul vs live — mesure du
//!   coût du repaint documenté (Phase 3.4).
//! - **R5** (A/B) : confluences MTF à containment DIRECTIONNEL vs
//!   agnostique+existence — correctif de l'impureté logique.
//! - **R6** (post-hoc) : inducement — EQL non consommé sous l'entrée (bull) /
//!   EQH au-dessus (bear) à ≤ N×ATR : R des trades avec vs sans inducement.
//! - **R7** (post-hoc) : risque gradué par force — R pondéré (0,5 sous le
//!   seuil) pour seuils 5/6/7/8 : le R des trades faibles est-il négatif ?
//! - **R8** (post-hoc) : biais D1 — dernier jour UTC clôturé haussier/
//!   baissier : R des trades alignés vs en contradiction.
//!
//! R6/R7/R8 sont mesurés POST-HOC sur le replay production (aucun changement
//! de stratégie — si un verdict est positif, la construction se décide ensuite).
//!
//! Usage : cargo run --release -p api --bin passe_finale
//! Sortie : console + data/passe_finale.txt

use std::collections::HashMap;
use std::sync::Arc;

use common::{Asset, Timeframe};
use smc::v12::scoring_v11::ScoringV11;
use smc::v12::trade::{Side, TradeSource};
use smc::v12::types::BarInput;
use smc::v12::{AmorceMtf, SmcV12Engine};
use smc::v12::lifecycle::ModeBeForce;
use smc::v12::signals::ModeTp3;

const ASSETS: &[&str] = &["BTC", "XAUUSD", "XAGUSD", "NAS100", "SP500", "DAX"];
const FENETRES: &[(&str, i64)] = &[("M15", 30_000), ("M5", 26_000), ("M1", 30_000)];

#[derive(Default, Clone)]
struct Stats {
    fermes: usize,
    r_total: f64,
}

impl Stats {
    fn cumule(&mut self, r: f64) {
        self.fermes += 1;
        self.r_total += r;
    }
}

fn main() -> anyhow::Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(executer())
}

async fn executer() -> anyhow::Result<()> {
    let db_path = std::env::var("DATABASE_PATH")
        .unwrap_or_else(|_| "data/trading.db".to_string());
    let db = Arc::new(db::Database::new(&db_path).await?);
    db.run_migrations().await?;

    // (branche, asset, tf) → stats
    let mut ab: HashMap<&str, HashMap<String, HashMap<String, Stats>>> = HashMap::new();
    for nom in ["production", "R4-cloture", "R5-directionnel"] {
        ab.insert(nom, HashMap::new());
    }
    // Post-hoc : (asset, tf) → accumulateurs R6/R7/R8.
    let mut post_hoc: HashMap<String, HashMap<String, PostHoc>> = HashMap::new();

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
                continue;
            }
            println!("→ {} {} ({} bougies)", asset_id, tf_id, bougies.len());

            // ── Les 3 branches A/B (R4, R5) ──
            for (nom, (cloture, directionnel)) in [
                ("production", (false, false)),
                ("R4-cloture", (true, false)),
                ("R5-directionnel", (false, true)),
            ] {
                let mut moteur = SmcV12Engine::new(asset_id, tf_id)
                    .avec_mode_be_force(ModeBeForce::Supprime)
                    .avec_mode_tp3(ModeTp3::DolCappe3R)
                    .avec_mtf_cloture(cloture)
                    .avec_mtf_directionnel(directionnel);
                moteur.primer_mtf_amorce(&amorce, bougies[0].timestamp.timestamp());
                let mut s = Stats::default();
                for b in &bougies {
                    let out = moteur.update(&bougie_vers_bar(b));
                    let _ = out;
                }
                for t in &moteur.signals.trades {
                    if let Some(r) = t.close_r {
                        s.cumule(r);
                    }
                }
                ab.entry(nom)
                    .or_default()
                    .entry(asset_id.to_string())
                    .or_default()
                    .entry(tf_id.to_string())
                    .or_insert(s);
            }

            // ── Post-hoc R6/R7/R8 sur le replay production ──
            let mut moteur = SmcV12Engine::new(asset_id, tf_id)
                .avec_mode_be_force(ModeBeForce::Supprime)
                .avec_mode_tp3(ModeTp3::DolCappe3R);
            moteur.primer_mtf_amorce(&amorce, bougies[0].timestamp.timestamp());
            let mut ph = PostHoc::default();
            let mut jours: Vec<(i64, f64, f64)> = Vec::new(); // (jour_utc, open, close) clôturés
            let (mut j_cur, mut j_open, mut j_close) = (0i64, 0f64, 0f64);
            let mut snaps: Vec<(f64, Option<f64>, Option<f64>)> = Vec::new(); // (atr, eqh, eql)

            for b in &bougies {
                let bar = bougie_vers_bar(b);
                let out = moteur.update(&bar);
                snaps.push((out.atr14, out.liquidite.dernier_eqh_level, out.liquidite.dernier_eql_level));
                let jour = bar.timestamp.div_euclid(86_400);
                if jour != j_cur {
                    if j_cur != 0 {
                        jours.push((j_cur, j_open, j_close));
                    }
                    j_cur = jour;
                    j_open = bar.open;
                }
                j_close = bar.close;
            }
            let cal = &moteur.calibration;
            for t in &moteur.signals.trades {
                if t.source != TradeSource::Ob || !t.filled {
                    continue;
                }
                let r = t.close_r.unwrap_or(0.0);
                let force = ScoringV11::force(t.score, cal);
                let (atr, eqh, eql) = snaps.get(t.bar_created).copied().unwrap_or((0.0, None, None));

                // R7 : R pondéré selon la force (seuils 5/6/7/8).
                for seuil in [5, 6, 7, 8] {
                    if force >= seuil {
                        ph.r7_plein[seuil_index(seuil)] += r;
                    } else {
                        ph.r7_demi[seuil_index(seuil)] += r;
                    }
                }

                // R6 : inducement = liquidité opposée non consommée à ≤ N×ATR
                // du côté « sous l'entrée » pour un achat (EQL) / au-dessus pour une vente (EQH).
                for (k, mult) in [0.5f64, 1.0, 2.0].iter().enumerate() {
                    let present = match t.side {
                        Side::Buy => eql.is_some_and(|l| t.entry - l <= mult * atr && t.entry > l),
                        Side::Sell => eqh.is_some_and(|h| h - t.entry <= mult * atr && h > t.entry),
                    };
                    if present {
                        ph.r6_avec[k] += r;
                        ph.n6_avec[k] += 1;
                    } else {
                        ph.r6_sans[k] += r;
                        ph.n6_sans[k] += 1;
                    }
                }

                // R8 : biais D1 = dernier jour UTC CLÔTURÉ (open→close).
                let jour_trade = snaps.len() as i64; // placeholder (inutilisé)
                let _ = jour_trade;
                let ts = t.open_ts;
                let jour = ts.div_euclid(86_400);
                let dernier_ferme = jours.iter().rev().find(|(j, _, _)| *j < jour);
                if let Some((_, o, c)) = dernier_ferme {
                    let aligne = match t.side {
                        Side::Buy => *c >= *o,   // achat aligné si jour haussier (ou neutre)
                        Side::Sell => *c <= *o,  // vente alignée si jour baissier (ou neutre)
                    };
                    if aligne {
                        ph.r8_aligne += r;
                        ph.n8_aligne += 1;
                    } else {
                        ph.r8_contraire += r;
                        ph.n8_contraire += 1;
                    }
                }
            }
            post_hoc
                .entry(asset_id.to_string())
                .or_default()
                .entry(tf_id.to_string())
                .or_insert(ph);
        }
    }

    // ── Sortie ──
    let mut lignes: Vec<String> = Vec::new();
    lignes.push("═".repeat(110).into());
    lignes.push("PASSE FINALE ÉTAPE 3 : R4 (MTF clôturé) · R5 (containment directionnel) — A/B production".into());
    lignes.push("═".repeat(110).into());
    for asset_id in ASSETS {
        lignes.push(format!("── {} ──", asset_id));
        for tf in FENETRES.iter().map(|(t, _)| *t) {
            let vide = Stats::default();
            let mut ligne = format!("  {:<5}", tf);
            for nom in ["production", "R4-cloture", "R5-directionnel"] {
                let s = ab
                    .get(nom)
                    .and_then(|h| h.get(*asset_id))
                    .and_then(|h| h.get(tf))
                    .unwrap_or(&vide);
                ligne += &format!("  {:>22}", format!("{:+.1}R / {}", s.r_total, s.fermes));
            }
            lignes.push(ligne);
        }
    }
    lignes.push("─".repeat(110).into());
    for nom in ["production", "R4-cloture", "R5-directionnel"] {
        let mut total = Stats::default();
        for per_asset in ab.get(nom).into_iter().flatten() {
            for s in per_asset.1.values() {
                total.fermes += s.fermes;
                total.r_total += s.r_total;
            }
        }
        lignes.push(format!(
            "{:<16} TOTAL {:+.1}R · {} clôtures",
            nom, total.r_total, total.fermes
        ));
    }

    lignes.push("".into());
    lignes.push("═".repeat(110).into());
    lignes.push("POST-HOC (replay production) — R6 inducement · R7 risque gradué · R8 biais D1".into());
    lignes.push("═".repeat(110).into());
    // Agrégats globaux.
    let mut g = PostHoc::default();
    for per_tf in post_hoc.values() {
        for ph in per_tf.values() {
            g.ajoute(ph);
        }
    }
    for (k, mult) in [0.5f64, 1.0, 2.0].iter().enumerate() {
        lignes.push(format!(
            "R6 inducement ≤ {:.1}×ATR : AVEC = {} trades {:+.1}R · SANS = {} trades {:+.1}R",
            mult, g.n6_avec[k], g.r6_avec[k], g.n6_sans[k], g.r6_sans[k]
        ));
    }
    for (i, seuil) in [5, 6, 7, 8].iter().enumerate() {
        lignes.push(format!(
            "R7 seuil F≥{} : demi-risque → TOTAL pondéré {:+.1}R (pleins {:+.1}R + demi {:+.1}R)",
            seuil,
            g.r7_plein[i] + 0.5 * g.r7_demi[i],
            g.r7_plein[i],
            g.r7_demi[i]
        ));
    }
    lignes.push(format!(
        "R8 biais D1 : alignés = {} trades {:+.1}R · contraires = {} trades {:+.1}R",
        g.n8_aligne, g.r8_aligne, g.n8_contraire, g.r8_contraire
    ));

    let sortie = lignes.join("\n");
    println!("\n{}", sortie);
    std::fs::create_dir_all("data")?;
    std::fs::write("data/passe_finale.txt", &sortie)?;
    println!("\n📄 Écrit : data/passe_finale.txt");
    Ok(())
}

#[derive(Default, Clone)]
struct PostHoc {
    r6_avec: [f64; 3],
    n6_avec: [usize; 3],
    r6_sans: [f64; 3],
    n6_sans: [usize; 3],
    r7_plein: [f64; 4],
    r7_demi: [f64; 4],
    r8_aligne: f64,
    n8_aligne: usize,
    r8_contraire: f64,
    n8_contraire: usize,
}

impl PostHoc {
    fn ajoute(&mut self, o: &PostHoc) {
        for k in 0..3 {
            self.r6_avec[k] += o.r6_avec[k];
            self.n6_avec[k] += o.n6_avec[k];
            self.r6_sans[k] += o.r6_sans[k];
            self.n6_sans[k] += o.n6_sans[k];
        }
        for k in 0..4 {
            self.r7_plein[k] += o.r7_plein[k];
            self.r7_demi[k] += o.r7_demi[k];
        }
        self.r8_aligne += o.r8_aligne;
        self.n8_aligne += o.n8_aligne;
        self.r8_contraire += o.r8_contraire;
        self.n8_contraire += o.n8_contraire;
    }
}

fn seuil_index(seuil: i32) -> usize {
    (seuil - 5) as usize
}

fn bougie_vers_bar(b: &common::Candle) -> BarInput {
    BarInput {
        timestamp: b.timestamp.timestamp(),
        open: b.open,
        high: b.high,
        low: b.low,
        close: b.close,
        volume: b.volume,
    }
}

/// Amorce MTF (H1/H4/W1 + MN agrégée de D1) — copie des binaires d'étude.
async fn charger_amorce(db: &db::Database, asset: &Asset) -> AmorceMtf {
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
    AmorceMtf {
        h1: vers_bars(h1),
        h4: vers_bars(h4),
        w1: vers_bars(w1),
        mn: smc::v12::agreger_mensuel(&vers_bars(d1)),
    }
}
