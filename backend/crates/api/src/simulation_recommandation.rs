//! Étape 12 (roadmap audit 05/10) — l'advisory honnête sur les balayages.
//!
//! Le labo mesure des configurations (essais) ; le propriataire règle la
//! production à la main. Ce module ferme la boucle : quelle configuration
//! MESURÉE bat la configuration ACTUELLE, de combien, et (seulement si les
//! paramètres de l'essai correspondent à des réglages réels) un bouton
//! d'activation au clic propriétaire — avec relecture-vérification après
//! écriture (leçon placement_sec : une écriture non relue est une écriture
//! aveugle).
//!
//! Critère owner 06/10 : meilleur essai = **R TOTAL maximal** à effectif
//! ≥ 30 trades (la règle des 30). Bouton actif directement (pas de
//! confirmation). Jamais automatique, jamais en fond.

use actix_web::{web, HttpResponse, Responder};
use sqlx::Row;

use crate::state::AppState;

/// Effectif minimal pour qu'un essai soit recommandable (règle des 30).
const EFFECTIF_MIN: i64 = 30;

#[derive(Debug, Clone)]
struct Essai {
    id: String,
    params: serde_json::Value,
    r_total: f64,
    nb_trades: i64,
    taux_reussite: f64,
}

/// Périmètre d'un essai : Some([X]) = essai PAR ASSET, None = essai global
/// (absent ou liste vide — essais d'avant la phase 2).
fn perimetre_essai(e: &Essai) -> Option<Vec<String>> {
    let v = e.params.get("assets")?.as_array()?;
    let liste: Vec<String> = v.iter().filter_map(|x| x.as_str().map(String::from)).collect();
    if liste.is_empty() { None } else { Some(liste) }
}

async fn charger_essais(db: &AppState, strategie: &str) -> Vec<Essai> {
    let rows = sqlx::query(
        "SELECT id, params_json, resultats_json FROM simulation_essais WHERE strategie = ?",
    )
    .bind(strategie)
    .fetch_all(db.db.pool())
    .await
    .unwrap_or_default();
    rows.iter()
        .filter_map(|r| {
            let params: serde_json::Value =
                serde_json::from_str(&r.try_get::<String, _>("params_json").ok()?).ok()?;
            let res: serde_json::Value =
                serde_json::from_str(&r.try_get::<String, _>("resultats_json").ok()?).ok()?;
            Some(Essai {
                id: r.try_get("id").ok()?,
                r_total: res["r_total"].as_f64()?,
                nb_trades: res["nb_trades"].as_i64()?,
                taux_reussite: res["taux_reussite"].as_f64().unwrap_or(0.0),
                params,
            })
        })
        .collect()
}

/// Les réglages de production, projetés dans l'espace des paramètres
/// d'essai (quand la correspondance existe). SMC : fractions + k trailing
/// (kv). KDJ : period/signal/amplitude/adx_min (table kdj_params).
/// Straddle/rockets : None — l'espace balayé est virtuel ou absent.
/// Config SMC globale (clés smc_* de la configuration).
async fn lire_config_smc_globale(db: &AppState) -> Option<serde_json::Value> {
    let lire = |cle: &str| {
        let pool = db.db.pool();
        let cle = cle.to_string();
        async move {
            sqlx::query_scalar::<_, String>("SELECT valeur FROM configuration WHERE cle = ?")
                .bind(cle)
                .fetch_optional(pool)
                .await
                .ok()
                .flatten()
        }
    };
    Some(serde_json::json!({
        "frac_tp1": lire("smc_frac_tp1").await?.parse::<f64>().ok()?,
        "frac_tp2": lire("smc_frac_tp2").await?.parse::<f64>().ok()?,
        "frac_tp3": lire("smc_frac_tp3").await?.parse::<f64>().ok()?,
        "tp1_mult": lire("smc_tp1_mult").await?.parse::<f64>().ok()?,
        "tp2_mult": lire("smc_tp2_mult").await?.parse::<f64>().ok()?,
        "tp3_mode": lire("smc_tp3_mode").await?,
        "tp3_rfixe": lire("smc_tp3_rfixe").await?.parse::<f64>().ok()?,
        "tp3_trailing": lire("smc_tp3_trailing").await?.parse::<f64>().ok()?,
    }))
}

/// Config actuelle, éventuellement PAR ASSET (fusion surcharge ⊕ défaut).
/// Pub pour l'analyste IA par asset (phase 3).
pub(crate) async fn config_actuelle_asset(
    db: &AppState,
    strategie: &str,
    asset: Option<&str>,
) -> Option<serde_json::Value> {
    match (strategie, asset) {
        ("SMC", Some(a)) => {
            // PAR ASSET : la « config actuelle » = les réglages FUSIONNÉS de
            // l'asset (une surcharge active peut rendre l'essai « actuel »).
            let surcharge = db::reglages_asset::lire_smc_surcharge(db.db.pool(), a)
                .await
                .unwrap_or_default();
            let global = lire_config_smc_globale(db).await?;
            Some(serde_json::json!({
                "frac_tp1": surcharge.frac_tp1.or(global["frac_tp1"].as_f64()),
                "frac_tp2": surcharge.frac_tp2.or(global["frac_tp2"].as_f64()),
                "frac_tp3": surcharge.frac_tp3.or(global["frac_tp3"].as_f64()),
                "tp1_mult": surcharge.tp1.or(global["tp1_mult"].as_f64()),
                "tp2_mult": surcharge.tp2.or(global["tp2_mult"].as_f64()),
                "tp3_mode": surcharge.tp3_mode.clone().or(global["tp3_mode"].as_str().map(String::from)),
                "tp3_rfixe": surcharge.tp3_rfixe.or(global["tp3_rfixe"].as_f64()),
                "tp3_trailing": surcharge.trailing_r.or(global["tp3_trailing"].as_f64()),
            }))
        }
        ("kdj_halftrend", Some(a)) => {
            let surcharge = db::reglages_asset::lire_kdj_surcharge(db.db.pool(), a)
                .await
                .unwrap_or_default();
            let g = db::kdj_params::lire_kdj_params(db.db.pool()).await;
            Some(serde_json::json!({
                "period": surcharge.period.unwrap_or(g.period),
                "signal": surcharge.signal.unwrap_or(g.signal),
                "amplitude": surcharge.amplitude.unwrap_or(g.amplitude),
                "adx_min": surcharge.adx_min.unwrap_or(g.adx_min),
            }))
        }
        ("SMC", None) => lire_config_smc_globale(db).await,
        ("kdj_halftrend", None) => {
            let p = db::kdj_params::lire_kdj_params(db.db.pool()).await;
            Some(serde_json::json!({
                "period": p.period, "signal": p.signal,
                "amplitude": p.amplitude, "adx_min": p.adx_min,
            }))
        }
        _ => None,
    }
}

/// Un essai correspond-il à la configuration actuelle ? Comparaison sur
/// les clés COMMUNES (l'essai peut être plus pauvre que la config).
fn essai_est_actuel(params: &serde_json::Value, actuel: &serde_json::Value) -> bool {
    let mut vues = 0usize;
    for (cle, valeur) in actuel.as_object().into_iter().flatten() {
        if let Some(p) = params.get(cle) {
            // null dans l'essai = « non applicable dans cet essai » (ex.
            // tp3_trailing absent des balayages d'avant septembre) : clé
            // ignorée, pas une différence.
            if p.is_null() {
                continue;
            }
            vues += 1;
            let egale = match (valeur.as_f64(), p.as_f64()) {
                (Some(a), Some(b)) => (a - b).abs() < 1e-9,
                _ => valeur == p,
            };
            if !egale {
                return false;
            }
        }
    }
    vues > 0
}

/// GET /api/strategies/{id}/recommandation — le meilleur essai (R total
/// max, effectif ≥ 30), l'essai à la config actuelle s'il existe, l'écart,
/// et la capacité d'activation.
pub async fn get_recommandation(
    state: web::Data<AppState>,
    path: web::Path<String>,
    q: web::Query<std::collections::HashMap<String, String>>,
) -> impl Responder {
    let id = path.into_inner();
    let asset = q.get("asset").cloned();
    let essais_tous = charger_essais(&state, &id).await;
    // PAR ASSET (phase 2) : ne garder que les essais DU périmètre demandé —
    // essais globaux pour la vue globale, essais tagués [asset] sinon.
    let essais: Vec<Essai> = match &asset {
        None => essais_tous.into_iter().filter(|e| perimetre_essai(e).is_none()).collect(),
        Some(a) => essais_tous
            .into_iter()
            .filter(|e| perimetre_essai(e).as_deref() == Some([a.to_string()].as_slice()))
            .collect(),
    };
    if essais.is_empty() {
        return HttpResponse::Ok().json(serde_json::json!({
            "strategie": id, "meilleur": null, "actuel": null,
            "delta_r": null, "activable": false,
            "message": "Aucun essai en base — lance un balayage pour nourrir la recommandation.",
        }));
    }

    let meilleur = essais
        .iter()
        .filter(|e| e.nb_trades >= EFFECTIF_MIN)
        .max_by(|a, b| a.r_total.total_cmp(&b.r_total));
    let actuel_cfg = config_actuelle_asset(&state, &id, asset.as_deref()).await;
    let actuel = actuel_cfg
        .as_ref()
        .and_then(|cfg| essais.iter().find(|e| essai_est_actuel(&e.params, cfg)));
    // Activation possible quand les paramètres de l'essai vivent dans les
    // réglages réels (SMC, KDJ) — l'espace straddle est virtuel : info seule.
    let activable = matches!(id.as_str(), "SMC" | "kdj_halftrend");

    let (meilleur_json, effectif_insuffisant) = match meilleur {
        Some(m) => (
            Some(serde_json::json!({
                "id": m.id, "params": m.params, "r_total": m.r_total,
                "nb_trades": m.nb_trades, "taux_reussite": m.taux_reussite,
            })),
            false,
        ),
        None => (None, true),
    };
    let delta_r = match (meilleur, actuel) {
        (Some(m), Some(a)) => Some((m.r_total - a.r_total).round() / 1.0),
        _ => None,
    };
    HttpResponse::Ok().json(serde_json::json!({
        "strategie": id,
        "asset": asset,
        "meilleur": meilleur_json,
        "actuel": actuel.map(|a| serde_json::json!({
            "id": a.id, "r_total": a.r_total, "nb_trades": a.nb_trades,
        })),
        "delta_r": delta_r,
        "activable": activable && meilleur.is_some(),
        "effectif_min": EFFECTIF_MIN,
        "effectif_insuffisant": effectif_insuffisant,
        "message": if effectif_insuffisant {
            format!("Effectif insuffisant ({} trades requis) — les données s'accumulent.", EFFECTIF_MIN)
        } else { String::new() },
    }))
}

/// Écart de balayage JUGEABLE d'un asset (règle des 30 satisfaite) —
/// consommé par le contexte LLM ET par les chiffres clés de la modale.
pub(crate) struct EcartBalayageAsset {
    pub params: String,
    pub r_total: f64,
    pub nb_trades: i64,
    /// R de l'essai à la config actuelle, quand il existe.
    pub r_actuel: Option<f64>,
}

impl EcartBalayageAsset {
    /// Écart meilleur − actuel (None sans essai actuel).
    pub fn delta(&self) -> Option<f64> {
        self.r_actuel.map(|a| self.r_total - a)
    }
}

/// Mesure l'écart de balayage d'un asset : None = pas d'essai, ou aucun
/// essai à effectif ≥ 30 (non jugeable — le contexte LLM le dit en texte).
pub(crate) async fn ecart_balayage_asset(
    db: &AppState,
    strategie: &str,
    asset: &str,
) -> Option<EcartBalayageAsset> {
    let essais_tous = charger_essais(db, strategie).await;
    let essais: Vec<Essai> = essais_tous
        .into_iter()
        .filter(|e| perimetre_essai(e).as_deref() == Some([asset.to_string()].as_slice()))
        .collect();
    let m = essais
        .iter()
        .filter(|e| e.nb_trades >= EFFECTIF_MIN)
        .max_by(|a, b| a.r_total.total_cmp(&b.r_total))?;
    let actuel_cfg = config_actuelle_asset(db, strategie, Some(asset)).await;
    let r_actuel = actuel_cfg
        .as_ref()
        .and_then(|cfg| essais.iter().find(|e| essai_est_actuel(&e.params, cfg)))
        .map(|a| a.r_total);
    Some(EcartBalayageAsset {
        params: params_compacts(&m.params),
        r_total: m.r_total,
        nb_trades: m.nb_trades,
        r_actuel,
    })
}

/// Résumé du balayage POUR UN ASSET, à destination de l'analyste IA par
/// asset (phase 3) — réutilise la sélection d'essais du périmètre, le
/// meilleur à effectif ≥ 30 et l'écart vs la config actuelle fusionnée.
/// Texte honnête : aucun essai / effectif insuffisant / écart chiffré.
pub(crate) async fn resume_balayage_asset(
    db: &AppState,
    strategie: &str,
    asset: &str,
) -> String {
    let essais_tous = charger_essais(db, strategie).await;
    let essais: Vec<Essai> = essais_tous
        .into_iter()
        .filter(|e| perimetre_essai(e).as_deref() == Some([asset.to_string()].as_slice()))
        .collect();
    if essais.is_empty() {
        return format!(
            "Aucun essai du labo pour {asset} seul — nourrir le labo (page Simulation, chip {asset} seule puis balayer) avant d'attendre un conseil chiffré du balayage."
        );
    }
    let Some(e) = ecart_balayage_asset(db, strategie, asset).await else {
        return format!(
            "Essais du labo présents pour {asset} mais aucun n'atteint {} trades (règle des 30 par asset) — écart non jugeable.",
            EFFECTIF_MIN
        );
    };
    match e.r_actuel {
        Some(r_actuel) => format!(
            "Meilleur essai du labo pour {asset} : {} → {:+.1} R total sur {} trades (re-jeu) ; essai à la config actuelle : {:+.1} R — écart {:+.1} R.",
            e.params, e.r_total, e.nb_trades, r_actuel, e.r_total - r_actuel
        ),
        None => format!(
            "Meilleur essai du labo pour {asset} : {} → {:+.1} R total sur {} trades (re-jeu) ; aucun essai ne correspond à la config actuelle — écart non mesurable.",
            e.params, e.r_total, e.nb_trades
        ),
    }
}

/// Paramètres d'un essai en forme compacte pour un contexte LLM :
/// clé=valeur, en sautant les clés de périmètre et les null.
fn params_compacts(p: &serde_json::Value) -> String {
    p.as_object()
        .map(|o| {
            o.iter()
                .filter(|(k, v)| !v.is_null() && k.as_str() != "assets" && k.as_str() != "tfs")
                .map(|(k, v)| match v.as_f64() {
                    Some(f) => format!("{}={}", k, coupe_zeros(f)),
                    None => format!("{k}={v}"),
                })
                .collect::<Vec<_>>()
                .join(" · ")
        })
        .unwrap_or_default()
}

/// 0.6000000000000001 → « 0.6 » (affichage compact, pas un arrondi de calcul).
fn coupe_zeros(f: f64) -> String {
    let s = format!("{f:.4}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

#[derive(serde::Deserialize)]
pub struct BodyActiver {
    pub essai_id: String,
}

/// POST /api/strategies/{id}/recommandation/activer — écrit les paramètres
/// de l'essai dans les réglages réels, RELIT et VÉRIFIE (écriture aveugle
/// interdite), puis répond. Clic propriétaire uniquement.
pub async fn activer(
    state: web::Data<AppState>,
    path: web::Path<String>,
    q: web::Query<std::collections::HashMap<String, String>>,
    body: web::Json<BodyActiver>,
) -> impl Responder {
    let id = path.into_inner();
    let asset = q.get("asset").cloned();
    let essais = charger_essais(&state, &id).await;
    let Some(essai) = essais.iter().find(|e| e.id == body.essai_id) else {
        return HttpResponse::NotFound().json(serde_json::json!({ "error": "Essai inconnu" }));
    };
    if essai.nb_trades < EFFECTIF_MIN {
        return HttpResponse::Conflict().json(serde_json::json!({
            "error": format!("Effectif insuffisant ({} < {})", essai.nb_trades, EFFECTIF_MIN)
        }));
    }
    let p = &essai.params;
    let mut appliques: Vec<String> = Vec::new();
    let result = match (id.as_str(), asset.as_deref()) {
        // PAR ASSET (phase 2) : l'activation écrit dans la SURCHARGE de
        // l'asset (jamais dans les réglages globaux) — hot-reload ≤ 60 s
        // sur les seuls couples de l'asset.
        ("SMC", Some(a)) => {
            let surcharge = db::reglages_asset::lire_smc_surcharge(state.db.pool(), a)
                .await
                .unwrap_or_default();
            let maj = db::reglages_asset::SmcSurcharge {
                tp1: p.get("tp1_mult").and_then(|v| v.as_f64()),
                tp2: p.get("tp2_mult").and_then(|v| v.as_f64()),
                tp3_mode: p
                    .get("tp3_mode")
                    .and_then(|v| v.as_str())
                    .map(|v| if v == "rfixe" { "fixe".to_string() } else { v.to_string() }),
                tp3_rfixe: p.get("tp3_rfixe").and_then(|v| v.as_f64()),
                trailing_r: p
                    .get("tp3_trailing")
                    .and_then(|v| v.as_f64()),
                frac_tp1: p.get("frac_tp1").and_then(|v| v.as_f64()),
                frac_tp2: p.get("frac_tp2").and_then(|v| v.as_f64()),
                frac_tp3: p.get("frac_tp3").and_then(|v| v.as_f64()),
                ..surcharge
            };
            db::reglages_asset::ecrire_smc_surcharge(state.db.pool(), a, &maj)
                .await
                .map(|_| ())
                .map_err(|e| anyhow::anyhow!("{e}"))
        }
        ("kdj_halftrend", Some(a)) => {
            let surcharge = db::reglages_asset::lire_kdj_surcharge(state.db.pool(), a)
                .await
                .unwrap_or_default();
            let (Some(period), Some(signal), Some(amplitude), Some(adx_min)) = (
                p.get("period").and_then(|v| v.as_i64()),
                p.get("signal").and_then(|v| v.as_i64()),
                p.get("amplitude").and_then(|v| v.as_i64()),
                p.get("adx_min").and_then(|v| v.as_f64()),
            ) else {
                return HttpResponse::BadRequest()
                    .json(serde_json::json!({ "error": "paramètres KDJ incomplets dans l'essai" }));
            };
            let maj = db::reglages_asset::KdjSurcharge {
                period: Some(period),
                signal: Some(signal),
                amplitude: Some(amplitude),
                adx_min: Some(adx_min),
                ..surcharge
            };
            db::reglages_asset::ecrire_kdj_surcharge(state.db.pool(), a, &maj)
                .await
                .map(|_| ())
                .map_err(|e| anyhow::anyhow!("{e}"))
        }
        ("SMC", None) => {
            let mut ok = true;
            for (cle_json, cle_kv, is_num) in [
                ("frac_tp1", "smc_frac_tp1", true),
                ("frac_tp2", "smc_frac_tp2", true),
                ("frac_tp3", "smc_frac_tp3", true),
                ("tp1_mult", "smc_tp1_mult", true),
                ("tp2_mult", "smc_tp2_mult", true),
                ("tp3_rfixe", "smc_tp3_rfixe", true),
                ("tp3_trailing", "smc_tp3_trailing", true),
                ("tp3_mode", "smc_tp3_mode", false),
            ] {
                let Some(v) = p.get(cle_json) else { continue };
                if v.is_null() { continue; } // absent de l'essai — PAS un zéro légitime
                let valeur = if is_num {
                    v.as_f64().unwrap_or(0.0).to_string()
                } else {
                    match v.as_str() { Some(s) => s.to_string(), None => continue }
                };
                ok &= state.db.ecrire_config(cle_kv, &valeur).await.is_ok();
                appliques.push(cle_kv.to_string());
            }
            if ok { Ok(()) } else { Err(anyhow::anyhow!("échec d'écriture kv SMC")) }
        }
        ("kdj_halftrend", None) => {
            let (Some(period), Some(signal), Some(amplitude), Some(adx_min)) = (
                p.get("period").and_then(|v| v.as_i64()),
                p.get("signal").and_then(|v| v.as_i64()),
                p.get("amplitude").and_then(|v| v.as_i64()),
                p.get("adx_min").and_then(|v| v.as_f64()),
            ) else {
                return HttpResponse::BadRequest()
                    .json(serde_json::json!({ "error": "paramètres KDJ incomplets dans l'essai" }));
            };
            let kp = db::kdj_params::KdjParams {
                period, signal, amplitude, ratio_risk: 2.0, adx_min,
            };
            db::kdj_params::sauvegarder_kdj_params(state.db.pool(), &kp)
                .await
                .map(|_| ())
                .map_err(|e| anyhow::anyhow!("{e}"))
        }
        _ => Err(anyhow::anyhow!("Stratégie sans activation (espace virtuel)")),
    };
    match result {
        Ok(()) => {
            // Relecture-vérification : l'écriture doit être relue identique.
            let relu = config_actuelle_asset(&state, &id, asset.as_deref()).await;
            let verifie = relu
                .map(|cfg| essai_est_actuel(p, &cfg))
                .unwrap_or(false);
            if verifie {
                HttpResponse::Ok().json(serde_json::json!({
                    "ok": true, "appliques": appliques, "verifie": true,
                }))
            } else {
                HttpResponse::InternalServerError().json(serde_json::json!({
                    "error": "Écriture non confirmée par relecture — inspecter les réglages",
                }))
            }
        }
        _ => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": "Échec d'écriture des réglages",
        })),
    }
}
