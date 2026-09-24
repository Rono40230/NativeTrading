//! Balayage de calibration KDJ — laboratoire (7.G, décision 24/09) : le
//! rejeu 24 mois H1 VALIDÉ (parité live↔rejeur, frais 0,05 %/ordre) est
//! rejoué avec des variantes des 5 paramètres, UN À LA FOIS autour du
//! réglage actuel. Calibrer AVANT la mesure 30 trades (7.F) — leçon du
//! labo SMC : le balayage quantifie ce que le vécu ne montrerait qu'en
//! le perdant.

use crate::simulation::RequeteBalayageSmc;
use crate::state::AppState;
use actix_web::{web, HttpResponse};
use kdj_halftrend::ParamsKdj;

/// Grilles (un paramètre varie, les autres = réglage actuel).
const PERIODES: [i64; 4] = [9, 14, 20, 28];
const SIGNAUX: [i64; 4] = [3, 5, 7, 10];
const AMPLITUDES: [i64; 4] = [2, 3, 4, 6];
const RATIOS: [f64; 4] = [1.5, 2.0, 2.5, 3.0];
/// < 0 = filtre tendance désactivé (convention kdj_params).
const ADX: [f64; 4] = [-1.0, 20.0, 25.0, 30.0];
/// 24 mois + chauffe des indicateurs (SMA100, EMA200, ADX14).
const FENETRE_JOURS: u32 = 30 * 24 + 60;
const FRAIS: kdj_halftrend::Frais = kdj_halftrend::Frais { commission: 0.0005, slippage: 0.0 };

/// POST /api/strategies/kdj_halftrend/simulation/balayage — chaque ligne =
/// un rejeu complet multi-actifs. Métriques : Σ R net (frais inclus),
/// n clôturés, WR, PF ; le ratio R net/trade sert à la comparaison
/// inter-configurations (outil d'étude, pas affichage vécu).
pub async fn balayage(
    state: web::Data<AppState>,
    body: web::Json<RequeteBalayageSmc>,
) -> HttpResponse {
    let db = state.db.clone();
    let reglages = db::kdj_params::lire_kdj_params(db.pool()).await;
    let base = ParamsKdj {
        period: reglages.period.max(2) as usize,
        signal: reglages.signal.max(2) as usize,
        amplitude: reglages.amplitude.max(1) as usize,
        ratio_risk: reglages.ratio_risk.clamp(0.1, 10.0),
        adx_min: if reglages.adx_min >= 0.0 { Some(reglages.adx_min) } else { None },
    };

    // Périmètre : actifs du runtime disposant d'un historique H1 suffisant
    // (filtre d'étude optionnel). Bougies chargées UNE fois, réutilisées
    // par toutes les configurations du balayage.
    let filtre = body.assets.clone();
    let mut bougies_par_asset: Vec<(String, Vec<common::Candle>)> = Vec::new();
    for asset in crate::runtime_tick::assets_runtime(&db).await {
        if let Some(f) = &filtre {
            if !f.iter().any(|a| a == asset.as_str()) {
                continue;
            }
        }
        if let Ok(b) = db
            .obtenir_bougies_depuis_jours(&asset, &common::Timeframe::H1, FENETRE_JOURS)
            .await
        {
            if b.len() >= 250 {
                bougies_par_asset.push((asset.as_str().to_string(), b));
            }
        }
    }
    if bougies_par_asset.is_empty() {
        return HttpResponse::Ok()
            .json(serde_json::json!({ "erreur": "aucun actif avec historique H1 suffisant" }));
    }

    // (paramètre balayé, [(valeur affichée, config)]).
    let variantes: Vec<(&str, Vec<(serde_json::Value, ParamsKdj)>)> = vec![
        (
            "period",
            PERIODES
                .iter()
                .map(|&p| {
                    (
                        serde_json::json!(p),
                        ParamsKdj { period: p.max(2) as usize, ..base },
                    )
                })
                .collect(),
        ),
        (
            "signal",
            SIGNAUX
                .iter()
                .map(|&s| {
                    (
                        serde_json::json!(s),
                        ParamsKdj { signal: s.max(2) as usize, ..base },
                    )
                })
                .collect(),
        ),
        (
            "amplitude",
            AMPLITUDES
                .iter()
                .map(|&a| {
                    (
                        serde_json::json!(a),
                        ParamsKdj { amplitude: a.max(1) as usize, ..base },
                    )
                })
                .collect(),
        ),
        (
            "ratio_risk",
            RATIOS
                .iter()
                .map(|&r| (serde_json::json!(r), ParamsKdj { ratio_risk: r, ..base }))
                .collect(),
        ),
        (
            "adx_min",
            ADX.iter()
                .map(|&a| {
                    (
                        if a >= 0.0 { serde_json::json!(a) } else { serde_json::json!("off") },
                        ParamsKdj {
                            adx_min: if a >= 0.0 { Some(a) } else { None },
                            ..base
                        },
                    )
                })
                .collect(),
        ),
    ];

    let mut balayages = serde_json::Map::new();
    for (nom, configs) in &variantes {
        if !body.cible.as_deref().map_or(true, |c| c == *nom) {
            continue;
        }
        let lignes: Vec<serde_json::Value> = configs
            .iter()
            .map(|(valeur, p)| {
                let actuel = p.period == base.period
                    && p.signal == base.signal
                    && p.amplitude == base.amplitude
                    && p.ratio_risk == base.ratio_risk
                    && p.adx_min == base.adx_min;
                mesurer_config(&bougies_par_asset, p, valeur, actuel)
            })
            .collect();
        balayages.insert((*nom).to_string(), serde_json::Value::Array(lignes));
    }

    HttpResponse::Ok().json(serde_json::json!({
        "parametres_actuels": {
            "period": reglages.period, "signal": reglages.signal,
            "amplitude": reglages.amplitude, "ratio_risk": reglages.ratio_risk,
            "adx_min": if reglages.adx_min >= 0.0 { serde_json::json!(reglages.adx_min) } else { serde_json::json!("off") },
        },
        "frais": "0,05 %/ordre", "fenetre_mois": 24,
        "actifs": bougies_par_asset.iter().map(|(a, _)| a.clone()).collect::<Vec<_>>(),
        "balayages": balayages,
    }))
}

/// Un rejeu multi-actifs pour une configuration, agrégé global + par actif.
fn mesurer_config(
    bougies_par_asset: &[(String, Vec<common::Candle>)],
    p: &ParamsKdj,
    valeur: &serde_json::Value,
    actuel: bool,
) -> serde_json::Value {
    let mut par_asset = Vec::new();
    let mut tous = Vec::new();
    for (asset, bougies) in bougies_par_asset {
        let mut trades = kdj_halftrend::rejouer(bougies, p);
        kdj_halftrend::appliquer_frais(&mut trades, FRAIS);
        let m = kdj_halftrend::mesurer(&trades);
        tous.extend(trades);
        par_asset.push(serde_json::json!({
            "asset": asset, "trades": m.nb_clotures, "ouverts": m.nb_ouvert,
            "r_net_total": arrondi(m.r_moyen_net * m.nb_clotures as f64),
        }));
    }
    let m = kdj_halftrend::mesurer(&tous);
    serde_json::json!({
        "valeur": valeur,
        "actuel": actuel,
        "trades": m.nb_clotures,
        "wr": arrondi(m.win_rate * 100.0),
        "pf": if m.pf_net.is_finite() { arrondi2(m.pf_net) } else { 99.9 },
        "r_net_total": arrondi(m.r_moyen_net * m.nb_clotures as f64),
        "r_net_par_trade": arrondi4(m.r_moyen_net),
        "par_asset": par_asset,
    })
}

fn arrondi(v: f64) -> f64 {
    (v * 10.0).round() / 10.0
}
fn arrondi2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}
fn arrondi4(v: f64) -> f64 {
    (v * 10_000.0).round() / 10_000.0
}
