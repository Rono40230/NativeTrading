//! Étude C1 (Phase C, spec 08/10 § 7) — la confluence dorée (OB ∩ OTE swing)
//! a-t-elle une valeur prédictive sur l'issue des trades SMC v12 ?
//!
//! Méthode (lecture seule, moteur inchangé) : replay par asset sur le TF
//! demandé ; à chaque barre on capture l'OTE swing vivante + les zones
//! actives des DEUX moteurs (OB v11 + BSZones). Pour chaque trade rempli,
//! trois définitions de « confluence à l'entrée », mesurées à la barre de
//! fill :
//!   D1 « entrée dans l'OTE »   : entry ∈ [ote.bot ; ote.top] — le concept
//!      FIRE (« Require OTE confluence for entry ») au sens strict ;
//!   D2 « zone dorée même sens »: l'entrée est dans (ou au bord exact d')
//!      une zone du sens du trade qui chevauche l'OTE ;
//!   D3 « zone dorée tout sens »: idem sans condition de sens.
//! Comparaison avec/sans : win/SL/autre, R moyen réalisé (`realized_r`),
//! z-score (2 proportions). Même canevas que l'étude conviction.
//!
//! ```sh
//! cargo run -p api --bin etude_confluence -- --tf M15 --limit 5000
//! ```

use db::Database;
use smc::v12::trade::{Side, TradeState, Verdict};
use smc::v12::{chevauche, BarInput, SmcV12Engine, SwingOteZone};
use sqlx::Row;

/// Instantané par barre : OTE swing vivante + zones des deux moteurs.
#[derive(Clone)]
struct Snap {
    ote: Option<SwingOteZone>,
    /// (top, bot, bull) — OB v11 puis BSZones.
    zones: Vec<(f64, f64, bool)>,
}

#[derive(Default)]
struct Groupe {
    n: usize,
    tp: usize,
    sl: usize,
    autre: usize,
    somme_r: f64,
}

impl Groupe {
    fn ajouter(&mut self, v: Verdict, r: f64) {
        self.n += 1;
        match v {
            Verdict::Tp1 | Verdict::Tp2 | Verdict::Tp3 => self.tp += 1,
            Verdict::Sl => self.sl += 1,
            _ => self.autre += 1,
        }
        self.somme_r += r;
    }
    fn absorber(&mut self, o: &Groupe) {
        self.n += o.n;
        self.tp += o.tp;
        self.sl += o.sl;
        self.autre += o.autre;
        self.somme_r += o.somme_r;
    }
    fn win_rate(&self) -> f64 {
        self.tp as f64 / self.n as f64
    }
    fn sl_rate(&self) -> f64 {
        self.sl as f64 / self.n as f64
    }
    fn r_moyen(&self) -> f64 {
        self.somme_r / self.n as f64
    }
}

fn afficher(nom: &str, g: &Groupe) {
    if g.n == 0 {
        println!("  {:28} — aucun trade", nom);
        return;
    }
    println!(
        "  {:28} n={:4}  win {:5.1} %  SL {:5.1} %  autre {:4.1} %  R moyen {:+.3}",
        nom,
        g.n,
        100.0 * g.win_rate(),
        100.0 * g.sl_rate(),
        100.0 * g.autre as f64 / self_n(g) as f64,
        g.r_moyen(),
    );
}

fn self_n(g: &Groupe) -> usize {
    g.n
}

fn z_deux_props(p1: f64, n1: usize, p2: f64, n2: usize) -> f64 {
    if n1 == 0 || n2 == 0 {
        return 0.0;
    }
    let (a, b) = (n1 as f64, n2 as f64);
    let p = (p1 * a + p2 * b) / (a + b);
    let se = (p * (1.0 - p) * (1.0 / a + 1.0 / b)).sqrt();
    if se <= 0.0 { 0.0 } else { (p1 - p2) / se }
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let mut tf_str = "M15".to_string();
    let mut limit: i64 = 5000;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--tf" if i + 1 < args.len() => tf_str = args[i + 1].clone(),
            "--limit" if i + 1 < args.len() => limit = args[i + 1].parse().unwrap_or(5000),
            _ => {}
        }
        i += 1;
    }
    let db_path =
        std::env::var("DATABASE_PATH").unwrap_or_else(|_| "data/trading.db".to_string());
    let rt = tokio::runtime::Runtime::new()?;
    let db = std::sync::Arc::new(rt.block_on(Database::new(&db_path))?);
    let timeframe = common::Timeframe::try_from(tf_str.as_str())?;

    let assets: Vec<String> = {
        let rows = rt.block_on(
            sqlx::query("SELECT DISTINCT asset FROM bougies WHERE timeframe = ?")
                .bind(tf_str.clone())
                .fetch_all(db.pool()),
        )?;
        rows.into_iter()
            .map(|r| r.try_get::<String, _>("asset").unwrap_or_default())
            .filter(|a| !a.is_empty())
            .collect()
    };

    // Trois paires (avec, sans) : D1 entrée dans OTE, D2 zone dorée même sens,
    // D3 zone dorée tout sens. Plus le split par moteur (v11 / bszones).
    let mut pool = [(); 4].map(|_| (Groupe::default(), Groupe::default()));
    let mut pool_src = [(); 2].map(|_| (Groupe::default(), Groupe::default()));
    let mut n_assets = 0usize;

    for asset_str in &assets {
        let asset = common::Asset::try_from(asset_str.as_str())?;
        let bougies = rt.block_on(db.obtenir_bougies(&asset, &timeframe, limit))?;
        if (bougies.len() as i64) < limit / 2 {
            continue;
        }
        let mut engine = SmcV12Engine::new(asset_str, &tf_str);
        if let Some(premiere) = bougies.first() {
            let t0 = premiere.timestamp.timestamp();
            use common::Timeframe as Tf;
            use smc::v12::{agreger_mensuel, BarInput as BarMtf};
            let charger = |tf: Tf| -> Vec<BarMtf> {
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
            let (h1, h4, w1) = (charger(Tf::H1), charger(Tf::H4), charger(Tf::W1));
            let mn = agreger_mensuel(&charger(Tf::D1));
            engine.primer_mtf(&h1, &h4, &w1, &mn, t0);
        }

        let mut snaps: Vec<(i64, Snap)> = Vec::with_capacity(bougies.len());
        for b in &bougies {
            let bar = BarInput {
                timestamp: b.timestamp.timestamp(),
                open: b.open,
                high: b.high,
                low: b.low,
                close: b.close,
                volume: b.volume,
            };
            let out = engine.update(&bar);
            let mut zones: Vec<(f64, f64, bool)> = engine
                .order_blocks
                .bull_zones()
                .iter()
                .map(|z| (z.top, z.bot, true))
                .chain(engine.order_blocks.bear_zones().iter().map(|z| (z.top, z.bot, false)))
                .collect();
            zones.extend(
                engine
                    .scoring_bs
                    .bull_zones()
                    .iter()
                    .map(|z| (z.top, z.bot, true))
                    .chain(engine.scoring_bs.bear_zones().iter().map(|z| (z.top, z.bot, false))),
            );
            snaps.push((bar.timestamp, Snap { ote: out.swing_ote.zone, zones }));
        }
        let idx_ts: std::collections::HashMap<i64, usize> =
            snaps.iter().enumerate().map(|(i, (ts, _))| (*ts, i)).collect();

        let mut locaux = [(); 4].map(|_| (Groupe::default(), Groupe::default()));
        let mut locaux_src = [(); 2].map(|_| (Groupe::default(), Groupe::default()));
        let mut total = 0usize;
        for t in &engine.signals.trades {
            if t.state != TradeState::Closed || !t.filled {
                continue;
            }
            let Some(fill_ts) = t.fill_ts else { continue };
            let Some(&i) = idx_ts.get(&fill_ts) else { continue };
            let snap = &snaps[i].1;
            let long = t.side == Side::Buy;
            let (v, r) = (t.verdict(), t.realized_r());
            total += 1;

            let d1 = snap.ote.map_or(false, |o| t.entry >= o.bot && t.entry <= o.top);
            let zone_doree = |meme_sens: bool| {
                snap.ote.map_or(false, |ote| {
                    snap.zones.iter().any(|(top, bot, bull)| {
                        (!meme_sens || *bull == long)
                            && t.entry >= *bot
                            && t.entry <= *top
                            && chevauche(*top, *bot, Some(ote))
                    })
                })
            };
            let d2 = zone_doree(true);
            let d3 = zone_doree(false);
            // D4 : entrée dans l'OTE DANS LE SENS de la jambe (ICT strict :
            // short dans l'OTE baissière, long dans la haussière).
            let d4 = d1 && snap.ote.map_or(false, |o| o.bearish != long);
            let defs = [d1, d2, d3, d4];
            for (k, d) in defs.iter().enumerate() {
                let g = if *d { &mut locaux[k].0 } else { &mut locaux[k].1 };
                g.ajouter(v, r);
            }
            let k_src = usize::from(t.source != smc::v12::trade::TradeSource::Ob);
            let g = if d2 {
                &mut locaux_src[k_src].0
            } else {
                &mut locaux_src[k_src].1
            };
            g.ajouter(v, r);
        }
        if total < 5 {
            continue;
        }
        n_assets += 1;
        for k in 0..4 {
            pool[k].0.absorber(&locaux[k].0);
            pool[k].1.absorber(&locaux[k].1);
        }
        for k in 0..2 {
            pool_src[k].0.absorber(&locaux_src[k].0);
            pool_src[k].1.absorber(&locaux_src[k].1);
        }
        println!("═ {asset_str} {tf_str} — {total} trades ═");
        afficher("D1 entrée dans OTE", &locaux[0].0);
        afficher("D2 zone dorée même sens", &locaux[1].0);
        afficher("sans (base D2)", &locaux[1].1);
    }

    println!("\n══════ POOL — {n_assets} actifs, {tf_str}, {limit} barres ══════");
    let noms = [
        "D1 : entrée dans l'OTE",
        "D2 : zone dorée même sens",
        "D3 : zone dorée tout sens",
        "D4 : entrée dans l'OTE, sens jambe",
    ];
    for (k, nom) in noms.iter().enumerate() {
        println!("─ {nom} ─");
        afficher("avec confluence", &pool[k].0);
        afficher("sans confluence", &pool[k].1);
        let (a, b) = (&pool[k].0, &pool[k].1);
        if a.n > 1 && b.n > 1 {
            let zw = z_deux_props(a.win_rate(), a.n, b.win_rate(), b.n);
            let zs = z_deux_props(a.sl_rate(), a.n, b.sl_rate(), b.n);
            println!("  z(win avec − sans) = {zw:+.2}   z(SL avec − sans) = {zs:+.2}");
        }
    }
    println!("─ D2 par moteur ─");
    for (k, nom) in ["moteur v11 (OB)", "moteur BSZones"].iter().enumerate() {
        println!("  {nom} :");
        afficher("avec confluence", &pool_src[k].0);
        afficher("sans confluence", &pool_src[k].1);
    }
    Ok(())
}
