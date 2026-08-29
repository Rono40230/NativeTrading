//! Sonde R3 — la confirmation LTF vaut-elle d'être construite ?
//! (docs/ETAPE3_VERIFICATION_SCORING_LOGIQUE_ENTREES.md, recommandation R3,
//! approche « sonde d'abord » validée par le propriétaire 29/08.)
//!
//! Hypothèse canonique : un shift structurel LTF (MSS/CHOCH directionnel)
//! survient DANS la zone pendant que l'ordre limite en attente vit — filler
//! seulement après lui améliorerait le taux de réussite.
//!
//! Méthode : rejoue l'historique par le moteur nu (production) et classe
//! chaque trade v11 REMPLI selon qu'un shift directionnel est survenu dans
//! la zone avant le fill :
//!   - confirmés   : trades gardés par un mode confirmation (+ leur R) ;
//!   - non-confirmés : trades PERDUS par un mode confirmation (+ leur R) ;
//!   - jamais fillés : identiques dans les deux modes (comptés à part).
//!
//! Définition sonde de « dans la zone » (approximation documentée) : barre
//! dont le range [low, high] contient le niveau d'entrée (= bord proximal de
//! la zone), ou close à ≤ 0.25×ATR de l'entrée.
//!
//! LIMITE : la sonde mesure l'effet FILTRE uniquement — en mode confirmation
//! les fills gardés se feraient à un prix différent (dégradation d'entrée non
//! mesurée ici). Verdict : si le R des non-confirmés est déjà positif, le
//! filtrage coûte → R3 mort-né sans construction.
//!
//! Usage : cargo run --release -p api --bin probe_confirmation

use std::sync::Arc;

use common::{Asset, Timeframe};
use smc::v12::trade::{Side, TradeSource};
use smc::v12::types::BarInput;
use smc::v12::SmcV12Engine;

const ASSETS: &[&str] = &["BTC", "XAUUSD", "XAGUSD", "NAS100", "SP500", "DAX"];
const FENETRES: &[(&str, i64)] = &[("M15", 30_000), ("M5", 26_000), ("M1", 30_000)];

/// Snapshots par barre : nécessaire au post-traitement des shifts.
struct Barre {
    ts: i64,
    high: f64,
    low: f64,
    close: f64,
    atr: f64,
    mss_h: bool,
    mss_b: bool,
    choch_h: bool,
    choch_b: bool,
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

    for asset_id in ASSETS {
        let Ok(asset) = Asset::try_from(*asset_id) else { continue };
        for (tf_id, barres) in FENETRES {
            let Ok(tf) = Timeframe::try_from(*tf_id) else { continue };
            let bougies = db
                .obtenir_bougies(&asset, &tf, *barres)
                .await
                .unwrap_or_default();
            if bougies.len() < 500 {
                continue;
            }
            let mut moteur = SmcV12Engine::new(asset_id, tf_id);
            let mut snapshots: Vec<Barre> = Vec::with_capacity(bougies.len());
            for b in &bougies {
                let bar = BarInput {
                    timestamp: b.timestamp.timestamp(),
                    open: b.open,
                    high: b.high,
                    low: b.low,
                    close: b.close,
                    volume: b.volume,
                };
                let out = moteur.update(&bar);
                snapshots.push(Barre {
                    ts: bar.timestamp,
                    high: bar.high,
                    low: bar.low,
                    close: bar.close,
                    atr: out.atr14,
                    mss_h: out.mss.mss_haussier,
                    mss_b: out.mss.mss_baissier,
                    choch_h: out.mss.choch_haussier,
                    choch_b: out.mss.choch_baissier,
                });
            }
            analyser(asset_id, tf_id, &moteur, &snapshots);
        }
    }
    Ok(())
}

/// Classe les trades v11 remplis : confirmés (shift directionnel dans la zone
/// avant le fill) vs non-confirmés, avec la somme de R de chaque sous-ensemble
/// — le R des non-confirmés = ce qu'un mode confirmation PERDrait.
fn analyser(asset: &str, tf: &str, moteur: &SmcV12Engine, s: &[Barre]) {
    let ts_vers_idx = |ts: i64| -> Option<usize> {
        s.binary_search_by(|b| b.ts.cmp(&ts)).ok()
    };

    let (mut n_conf, mut r_conf) = (0usize, 0.0f64);
    let (mut n_non, mut r_non) = (0usize, 0.0f64);
    let mut n_jamais = 0usize;
    let mut latences: Vec<usize> = Vec::new();
    let mut n_v11 = 0usize;

    for t in &moteur.signals.trades {
        if t.source != TradeSource::Ob {
            continue;
        }
        n_v11 += 1;
        if !t.filled {
            n_jamais += 1;
            continue;
        }
        let Some(fill_idx) = t.fill_ts.and_then(ts_vers_idx) else {
            continue;
        };
        let dir_haussier = t.side == Side::Buy;
        // Premier shift directionnel dans la zone, entre la création (incluse)
        // et le fill (exclu — « avant le fill » strict).
        let mut conf: Option<usize> = None;
        for j in t.bar_created..fill_idx {
            let b = &s[j];
            let shift = if dir_haussier {
                b.mss_h || b.choch_h
            } else {
                b.mss_b || b.choch_b
            };
            if !shift {
                continue;
            }
            let dans_zone = (b.low <= t.entry && b.high >= t.entry)
                || (b.close - t.entry).abs() <= 0.25 * b.atr;
            if dans_zone {
                conf = Some(j);
                break;
            }
        }
        let r = t.close_r.unwrap_or(0.0);
        match conf {
            Some(j) => {
                n_conf += 1;
                r_conf += r;
                latences.push(fill_idx.saturating_sub(j));
            }
            None => {
                n_non += 1;
                r_non += r;
            }
        }
    }

    let lat_moy = if latences.is_empty() {
        0.0
    } else {
        latences.iter().sum::<usize>() as f64 / latences.len() as f64
    };
    let pct_conf = if n_conf + n_non > 0 {
        100.0 * n_conf as f64 / (n_conf + n_non) as f64
    } else {
        0.0
    };
    println!(
        "{} {} : v11={} (remplis {}, jamais {}) · confirmés AVANT fill = {} ({:.0}%) R={:+.1} · NON-confirmés remplis = {} R={:+.1} · latence moy {:.1} bars",
        asset,
        tf,
        n_v11,
        n_conf + n_non,
        n_jamais,
        n_conf,
        pct_conf,
        r_conf,
        n_non,
        r_non,
        lat_moy
    );
}
