//! ATR14(H1) d'un asset — étalon du R des passes straddle.
//!
//! Calculé depuis les bougies H1 en DB (24 mois d'historique Axi/Bybit) au
//! moment de l'armement du moteur, puis auto-rafraîchi en live par le moteur
//! lui-même (clôtures horaires reconstituées du flux M1). Pourquoi H1 :
//! l'ATR M1 pré-annonce mesure la COMPRESSION avant la nouvelle — R
//! microscopique face au spike (constat Gate 3 26/08 : XAU R = 1,15 pt
//! pour un spike de 18,6 pts = 16 R).

use db::Database;

/// ATR14 (RMA Wilder, même formule que le moteur) sur les dernières H1.
pub async fn atr_h1(db: &Database, asset: &str) -> Option<f64> {
    let bougies = db
        .obtenir_bougies(&common::Asset::from(asset), &common::Timeframe::H1, 20)
        .await
        .unwrap_or_default();
    if bougies.len() < 5 {
        return None;
    }
    let mut valeur = 0.0_f64;
    let mut precedent_close: Option<f64> = None;
    let mut n = 0_u32;
    for b in &bougies {
        let tr = match precedent_close {
            Some(pc) => (b.high - b.low).max((b.high - pc).abs()).max((b.low - pc).abs()),
            None => b.high - b.low,
        };
        precedent_close = Some(b.close);
        n += 1;
        valeur = if n <= 14 {
            valeur + (tr - valeur) / n as f64 // moyenne cumulée
        } else {
            (valeur * 13.0 + tr) / 14.0 // RMA
        };
    }
    (valeur > 0.0).then_some(valeur)
}
