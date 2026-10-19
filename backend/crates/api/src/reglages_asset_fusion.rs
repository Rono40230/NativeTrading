//! Fusion surcharge PAR ASSET ⊕ défaut global (spec docs/spec_reglages_par_asset.md,
//! 0121). Source unique consommée par le montage des moteurs (runtime_tick),
//! l'empreinte du hot-reload, l'API réglages et (phases 2-3) labo/advisory/LLM.

use smc::v12::signals::Tp3Reglage;

/// Réglages SMC FUSIONNÉS pour un asset (défauts globaux ⊕ surcharge).
#[derive(Debug, Clone)]
pub struct ReglagesSmcAsset {
    pub tp1: f64,
    pub tp2: f64,
    pub tp3: Tp3Reglage,
    pub trailing: Option<f64>,
    /// SL fixe en unités PRIX (owner 19/10) : sl_pips × taille_pip de
    /// asset_params — source unique Gestion du risque/modale. None = pas
    /// de ligne (le moteur ne génère alors AUCUN signal, échec visible).
    pub sl_fixe: Option<f64>,
    /// Paliers score option C : multiplicateur faible (<8) / fort (≥12).
    pub palier_faible: f64,
    pub palier_fort: f64,
}

/// Lit les réglages globaux puis applique la surcharge de l'asset champ à champ.
pub async fn reglages_smc(db: &db::Database, asset: &str) -> ReglagesSmcAsset {
    let tp1 = crate::reglages_smc::lire_tp1_reglage(db).await;
    let tp2 = crate::reglages_smc::lire_tp2_reglage(db).await;
    let tp3_global = crate::reglages_smc::lire_tp3_reglage(db).await;
    let trailing = crate::reglages_smc::lire_trailing_reglage(db).await;
    let s = db::reglages_asset::lire_smc_surcharge(db.pool(), asset)
        .await
        .unwrap_or_default();
    let sl_fixe = db::asset_params::lire_sl_fixe_prix(db.pool(), asset)
        .await
        .ok()
        .flatten();
    let palier_faible = crate::reglages_smc::lire_palier(db, "smc_sl_palier_faible", 0.7).await;
    let palier_fort = crate::reglages_smc::lire_palier(db, "smc_sl_palier_fort", 1.3).await;
    ReglagesSmcAsset {
        tp1: s.tp1.unwrap_or(tp1),
        tp2: s.tp2.unwrap_or(tp2),
        tp3: Tp3Reglage {
            lointaine: match s.tp3_mode.as_deref() {
                Some("lointaine") => true,
                // "rfixe" = orthographe des essais/config, "fixe" = surcharge.
                Some("fixe") | Some("rfixe") => false,
                _ => tp3_global.lointaine,
            },
            rfixe: s.tp3_rfixe.unwrap_or(tp3_global.rfixe),
        },
        trailing: s.trailing_r.or(trailing),
        sl_fixe,
        palier_faible,
        palier_fort,
    }
}

/// Fusion PURE (tick : globals lus une fois, fusion en mémoire par couple).
/// `sl_fixe` = valeur de CET asset (asset_params) — le global est construit
/// avec l'asset vide, sa valeur serait toujours None (bug 19/10 : l'empreinte
/// hot-reload était aveugle aux changements de sl_pips).
pub fn fusionner_smc(
    global: ReglagesSmcAsset,
    s: &db::reglages_asset::SmcSurcharge,
    sl_fixe: Option<f64>,
) -> ReglagesSmcAsset {
    ReglagesSmcAsset {
        tp1: s.tp1.unwrap_or(global.tp1),
        tp2: s.tp2.unwrap_or(global.tp2),
        tp3: Tp3Reglage {
            lointaine: match s.tp3_mode.as_deref() {
                Some("lointaine") => true,
                Some("fixe") | Some("rfixe") => false,
                _ => global.tp3.lointaine,
            },
            rfixe: s.tp3_rfixe.unwrap_or(global.tp3.rfixe),
        },
        trailing: s.trailing_r.or(global.trailing),
        sl_fixe,
        palier_faible: global.palier_faible,
        palier_fort: global.palier_fort,
    }
}

/// Fusion PURE straddle (globals → ParamsStraddle construit une fois).
pub fn fusionner_straddle(global: &straddle::ParamsStraddle, s: &db::reglages_asset::StraddleSurcharge) -> straddle::ParamsStraddle {
    straddle::ParamsStraddle {
        sl_atr: s.sl_mult.unwrap_or(global.sl_atr),
        trailing_r: s.trailing_r.unwrap_or(global.trailing_r),
        placement_avant_sec: s.placement_sec.map(|x| x as i64).unwrap_or(global.placement_avant_sec),
        ..Default::default()
    }
}

/// Fusion PURE KDJ (empreinte du tick).
pub fn empreinte_kdj_fusionnee(
    g: &db::kdj_params::KdjParams,
    s: &db::reglages_asset::KdjSurcharge,
) -> String {
    format!(
        "p={};s={};a={};r={:.4};adx={:.4}",
        s.period.unwrap_or(g.period),
        s.signal.unwrap_or(g.signal),
        s.amplitude.unwrap_or(g.amplitude),
        s.ratio_risk.unwrap_or(g.ratio_risk),
        s.adx_min.unwrap_or(g.adx_min),
    )
}

/// Construit le réglage moteur du SL fixe depuis la fusion (runtime).
pub fn sl_fixe_etude(r: &ReglagesSmcAsset) -> Option<smc::v12::signals::SlFixeEtude> {
    r.sl_fixe.map(|base| smc::v12::signals::SlFixeEtude {
        base,
        mult_faible: r.palier_faible,
        mult_fort: r.palier_fort,
        garde: smc::v12::signals::GardeSlFixe::SerreBord,
    })
}

/// Empreinte texte des réglages SMC fusionnés d'un asset (hot-reload).
pub fn empreinte_smc(r: &ReglagesSmcAsset) -> String {
    format!(
        "tp1={:.4};tp2={:.4};tp3={}:{};trail={};slfixe={};paliers={}",
        r.tp1,
        r.tp2,
        if r.tp3.lointaine { "liq" } else { "fixe" },
        r.tp3.rfixe,
        r.trailing.map(|k| format!("{k:.4}")).unwrap_or_else(|| "off".into()),
        r.sl_fixe.map(|v| format!("{v:.4}")).unwrap_or_else(|| "ABSENT".into()),
        format!("{};{}", r.palier_faible, r.palier_fort),
    )
}

/// ParamsStraddle FUSIONNÉS pour un asset (défauts globaux ⊕ surcharge).
pub async fn straddle_params(db: &db::Database, asset: &str) -> straddle::ParamsStraddle {
    let p = db::strategies_params::lire_straddle_params(db.pool()).await;
    let s = db::reglages_asset::lire_straddle_surcharge(db.pool(), asset)
        .await
        .unwrap_or_default();
    straddle::ParamsStraddle {
        sl_atr: s.sl_mult.unwrap_or(p.sl_mult),
        trailing_r: s.trailing_r.unwrap_or(p.trailing_r),
        placement_avant_sec: s.placement_sec.unwrap_or(p.placement_sec) as i64,
        ..Default::default()
    }
}

/// Empreinte texte des params straddle fusionnés d'un asset (hot-reload).
pub fn empreinte_straddle(p: &straddle::ParamsStraddle) -> String {
    format!("sl={:.4};trail={:.4};place={}", p.sl_atr, p.trailing_r, p.placement_avant_sec)
}

/// Fusion PURE KDJ (globals + surcharge) → ParamsKdj du moteur — source
/// unique du montage (`moteur_kdj`) ; l'empreinte du tick reflète les
/// mêmes champs champ à champ.
pub fn params_kdj_fusionnes(
    g: &db::kdj_params::KdjParams,
    s: &db::reglages_asset::KdjSurcharge,
) -> kdj_halftrend::ParamsKdj {
    kdj_halftrend::ParamsKdj {
        period: s.period.unwrap_or(g.period).max(2) as usize,
        signal: s.signal.unwrap_or(g.signal).max(2) as usize,
        amplitude: s.amplitude.unwrap_or(g.amplitude).max(1) as usize,
        ratio_risk: s.ratio_risk.unwrap_or(g.ratio_risk).clamp(0.1, 10.0),
        adx_min: match s.adx_min.unwrap_or(g.adx_min) {
            v if v >= 0.0 => Some(v),
            _ => None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empreinte_inclut_le_sl_fixe() {
        // Régression 19/10 : l'empreinte doit refléter le sl_fixe de l'asset —
        // sinon le hot-reload ne voit pas un changement de sl_pips.
        let mut r = reglages_pour_test();
        r.sl_fixe = Some(5.0);
        assert!(empreinte_smc(&r).contains("slfixe=5.0000"));
        r.sl_fixe = Some(5.5);
        assert!(empreinte_smc(&r).contains("slfixe=5.5000"));
    }

    fn reglages_pour_test() -> ReglagesSmcAsset {
        ReglagesSmcAsset {
            tp1: 0.6,
            tp2: 2.0,
            tp3: Tp3Reglage { lointaine: true, rfixe: 3.0 },
            trailing: None,
            sl_fixe: None,
            palier_faible: 0.7,
            palier_fort: 1.3,
        }
    }
}
