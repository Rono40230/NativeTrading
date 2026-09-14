//! CLI du rejeu KDJ/Halftrend (phase 7.B — le vrai backtest).
//!
//! Usage :
//! ```sh
//! cargo run -p api --bin replay_kdj -- \
//!   --assets XAUUSD,XAGUSD,NAS100,SP500,DAX,BTC --tfs D1,H1 \
//!   --amplitudes 2,3,4 --ratios 1.5,2,2.5,3 --fenetre 730 \
//!   --commission 0.0005 --slippage 0
//! ```
//! `--fenetre` : en jours, mesurée sur l'entrée des trades (0 = tout
//! l'historique chargé). Règle projet : une tranche sous 30 trades clôturés
//! est marquée « sous-30 » (non concluante).

use std::sync::Arc;

use common::{Asset, Timeframe};
use db::Database;
use kdj_halftrend::{appliquer_frais, mesurer, rejouer, Frais, ParamsKdj};

fn liste<'a>(args: &'a [String], drapeau: &str, defaut: &'a str) -> Vec<String> {
    let mut it = args.iter();
    while let Some(a) = it.next() {
        if a == drapeau {
            if let Some(v) = it.next() {
                return v.split(',').filter(|s| !s.is_empty()).map(String::from).collect();
            }
        }
    }
    defaut.split(',').filter(|s| !s.is_empty()).map(String::from).collect()
}

fn scalaire_f64(args: &[String], drapeau: &str, defaut: f64) -> f64 {
    let mut it = args.iter();
    while let Some(a) = it.next() {
        if a == drapeau {
            if let Some(v) = it.next() {
                return v.parse().unwrap_or(defaut);
            }
        }
    }
    defaut
}

fn scalaire_i64(args: &[String], drapeau: &str, defaut: i64) -> i64 {
    let mut it = args.iter();
    while let Some(a) = it.next() {
        if a == drapeau {
            if let Some(v) = it.next() {
                return v.parse().unwrap_or(defaut);
            }
        }
    }
    defaut
}

fn scalaire_str(args: &[String], drapeau: &str) -> Option<String> {
    let mut it = args.iter();
    while let Some(a) = it.next() {
        if a == drapeau {
            if let Some(v) = it.next() {
                return Some(v.clone());
            }
        }
    }
    None
}

/// Bougies au format dumpé par l'EA (ts,open,high,low,close — heure serveur).
fn charger_bougies_csv(chemin: &str) -> anyhow::Result<Vec<common::Candle>> {
    let contenu = std::fs::read_to_string(chemin)?;
    let mut bougies = Vec::new();
    for ligne in contenu.lines().skip(1) {
        let champs: Vec<&str> = ligne.split(',').collect();
        if champs.len() < 5 {
            continue;
        }
        let (Ok(ts), Ok(o), Ok(h), Ok(l), Ok(c)) = (
            champs[0].trim().parse::<i64>(),
            champs[1].trim().parse::<f64>(),
            champs[2].trim().parse::<f64>(),
            champs[3].trim().parse::<f64>(),
            champs[4].trim().parse::<f64>(),
        ) else {
            continue;
        };
        let Some(timestamp) = chrono::DateTime::from_timestamp(ts, 0) else {
            continue;
        };
        bougies.push(common::Candle { timestamp, open: o, high: h, low: l, close: c, volume: 0.0 });
    }
    if bougies.is_empty() {
        anyhow::bail!("aucune bougie lisible dans {chemin}");
    }
    Ok(bougies)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--aide" || a == "-h") {
        println!("replay_kdj --assets A,B --tfs D1,H1 --amplitudes 2,3,4 --ratios 1.5,2,2.5,3 --fenetre 730 --commission 0.0005 --slippage 0");
        return Ok(());
    }
    let assets = liste(&args, "--assets", "XAUUSD,XAGUSD,NAS100,SP500,DAX,BTC");
    let tfs = liste(&args, "--tfs", "D1,H1");
    let amplitudes = liste(&args, "--amplitudes", "2,3,4");
    let ratios = liste(&args, "--ratios", "1.5,2,2.5,3");
    let fenetre_jours = scalaire_i64(&args, "--fenetre", 730);
    let commission = scalaire_f64(&args, "--commission", 0.0005);
    let slippage = scalaire_f64(&args, "--slippage", 0.0);
    // --dump chemin : exporte les trades d'UNE cellule (les 1ers éléments des
    // listes) au format de l'EA (diff 3 voies) et sort.
    let dump = scalaire_str(&args, "--dump");
    // --bougies fichier.csv : rejouer sur les bougies dumpées par l'EA
    // (parité au centime : mêmes barres que le Strategy Tester).
    let bougies_fichier = scalaire_str(&args, "--bougies");
    // --barres N : rejouer sur les N dernières barres chargées (état from
    // scratch) — aligne les seeds avec InpFenetre de l'EA pour la diff.
    let barres_max = scalaire_i64(&args, "--barres", 0);
    // --adx-min X : filtre tendance franche (scanner 7.E) — absent = étalon.
    let adx_min = scalaire_f64(&args, "--adx-min", -1.0);

    // Base : DATABASE_PATH, sinon chemins usuels (racine ou backend/).
    let candidats = [
        std::env::var("DATABASE_PATH").unwrap_or_default(),
        "data/trading.db".into(),
        "../data/trading.db".into(),
    ];
    let db_path = candidats
        .iter()
        .find(|p| !p.is_empty() && std::path::Path::new(p).exists())
        .cloned()
        .unwrap_or_else(|| "data/trading.db".into());
    let db = Arc::new(Database::new(&db_path).await?);
    db.run_migrations().await?;

    let frais = Frais { commission, slippage };
    println!(
        "# rejeu kdj_halftrend — base {} | fenêtre {} j | commission {:.4}%/ordre | slippage {}/ordre",
        db_path,
        if fenetre_jours > 0 { fenetre_jours.to_string() } else { "full".into() },
        commission * 100.0,
        slippage
    );
    println!(
        "{:<8} {:<3} {:>3} {:>5} {:>7} {:<11} {:>6} {:>6} {:>7} {:>6}",
        "asset", "tf", "amp", "ratio", "trades", "TP/SL/RT/Ov", "PF_R", "PF_%", "P_moy%", "DD_%"
    );

    // ── Mode --dump : une cellule, export CSV au format de l'EA, puis sortie ──
    if let Some(chemin) = dump {
        if assets.len() > 1 || tfs.len() > 1 || amplitudes.len() > 1 || ratios.len() > 1 {
            println!("! --dump exporte la 1re cellule : {} {} amp {} ratio {}",
                assets[0], tfs[0], amplitudes[0], ratios[0]);
        }
        let asset = Asset::try_from(assets[0].as_str())
            .map_err(|e| anyhow::anyhow!("asset inconnu '{}': {:?}", assets[0], e))?;
        let tf = Timeframe::try_from(tfs[0].as_str())
            .map_err(|e| anyhow::anyhow!("timeframe inconnu '{}': {:?}", tfs[0], e))?;
        let amp: usize = amplitudes[0].parse().unwrap_or(2);
        let ratio: f64 = ratios[0].parse().unwrap_or(2.0);
        let bougies: Vec<common::Candle> = if let Some(f) = &bougies_fichier {
            charger_bougies_csv(f)?
        } else {
            db.obtenir_bougies(&asset, &tf, 20_000).await?
        };
        let bougies: Vec<common::Candle> = if barres_max > 0 && (bougies.len() as i64) > barres_max {
            bougies[bougies.len() - barres_max as usize..].to_vec()
        } else {
            bougies
        };
        let params = ParamsKdj {
            amplitude: amp,
            ratio_risk: ratio,
            adx_min: if adx_min >= 0.0 { Some(adx_min) } else { None },
            ..ParamsKdj::default()
        };
        let trades = rejouer(&bougies, &params);
        let mut contenu = String::from("ts_entree,dir,open_entree,fill_entree,sl_niveau,tp_niveau,ts_sortie,open_sortie,fill_sortie,verdict\n");
        for t in &trades {
            let ts_s = |x: Option<chrono::DateTime<chrono::Utc>>| {
                x.map(|d| d.timestamp()).unwrap_or(0)
            };
            let (ts_sortie, prix_sortie) = (ts_s(t.ts_sortie), t.prix_sortie.unwrap_or(0.0));
            contenu.push_str(&format!(
                "{},{},{:.5},{:.5},{:.5},{:.5},{},{:.5},{:.5},{}\n",
                t.ts_entree.timestamp(), t.direction, t.prix_entree, t.prix_entree,
                t.niveau_sl, t.niveau_tp, ts_sortie, prix_sortie, prix_sortie,
                t.verdict.as_str()
            ));
        }
        std::fs::write(&chemin, contenu)?;
        println!("dump : {} trades → {}", trades.len(), chemin);
        return Ok(());
    }

    for asset_str in &assets {
        let asset = Asset::try_from(asset_str.as_str())
            .map_err(|e| anyhow::anyhow!("asset inconnu '{}': {:?}", asset_str, e))?;
        for tf_str in &tfs {
            let Ok(tf) = Timeframe::try_from(tf_str.as_str()) else {
                println!("!! timeframe inconnu : {tf_str} — ignoré");
                continue;
            };
            let bougies = db.obtenir_bougies(&asset, &tf, 20_000).await?;
            if bougies.is_empty() {
                println!("{:<8} {:<3} — aucune bougie en base", asset_str, tf_str);
                continue;
            }
            let derniere = bougies[bougies.len() - 1].timestamp;
            let cutoff = if fenetre_jours > 0 {
                Some(derniere - chrono::Duration::days(fenetre_jours))
            } else {
                None
            };
            for a in &amplitudes {
                let Ok(amp) = a.parse::<usize>() else { continue };
                for r in &ratios {
                    let Ok(ratio) = r.parse::<f64>() else { continue };
                    let params = ParamsKdj {
                        amplitude: amp,
                        ratio_risk: ratio,
                        adx_min: if adx_min >= 0.0 { Some(adx_min) } else { None },
                        ..ParamsKdj::default()
                    };
                    let mut trades = rejouer(&bougies, &params);
                    if let Some(cut) = cutoff {
                        trades.retain(|t| t.ts_entree >= cut);
                    }
                    appliquer_frais(&mut trades, frais);
                    let m = mesurer(&trades);
                    let marque = if m.nb_clotures < 30 { " †sous-30" } else { "" };
                    let pf = |v: f64| if v.is_infinite() { "∞".to_string() } else { format!("{v:.2}") };
                    println!(
                        "{:<8} {:<3} {:>3} {:>5} {:>7} {:<11} {:>6} {:>6} {:>7} {:>6}{}",
                        asset_str,
                        tf_str,
                        amp,
                        format!("{ratio:.1}"),
                        m.nb_clotures,
                        format!("{}/{}/{}/{}", m.nb_tp, m.nb_sl, m.nb_retournement, m.nb_ouvert),
                        pf(m.pf_net),
                        pf(m.pf_pct),
                        format!("{:+.3}", m.p_moyen_pct),
                        format!("{:.1}", m.dd_pct),
                        marque
                    );
                }
            }
        }
    }
    Ok(())
}
