//! Analyse IA PAR ASSET — POST /api/analyses/{strategie}/ia/asset/{asset}.
//!
//! Phase 3 des réglages par asset (spec docs/spec_reglages_par_asset.md § 5) :
//! l'analyste local (Ollama) reçoit, pour UN asset, trois blocs chiffrés —
//! vécu (R DISTANCE + $), réglages courants (surcharge ⊕ défaut global) et
//! écart mesuré au balayage du labo — et formule un conseil ACTIONNABLE.
//! L'IA propose, ne décide jamais (constitution du 24/08) : aucun chemin
//! d'écriture ici, l'activation reste au labo avec ses garde-fous.
//!
//! À la demande (bouton 🤖 du classement des assets), jamais en fond.
//! Cache du jour par (stratégie, asset, date) — même discipline que
//! l'analyste global (analyses_ia.rs).

use crate::analyses::{analyser, AnalyseStrategie, CategorieAnalyse};
use crate::analyses_ia::confiance_normalisee;
use crate::state::AppState;
use actix_web::{web, HttpResponse};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use tokio::sync::RwLock;

/// Effectif minimal par asset (règle des 30 — affichée partout).
const EFFECTIF_MIN: usize = 30;

/// Stratégies conseillables : celles qui ont une table de surcharge (0121).
/// Rockets n'en a pas — pas de conseil de réglage par asset.
const CONSEILLABLES: [&str; 3] = ["SMC", "straddle", "kdj_halftrend"];

/// Un chiffre clé de la modale : la valeur + sa phrase d'explication.
/// Fabriqués par le MOTEUR (déterministes, même source que le contexte) —
/// le LLM ne les rédige pas : pas de chiffre inventé ni mal expliqué.
#[derive(Debug, Clone, Serialize)]
pub struct ChiffreCle {
    pub chiffre: String,
    pub explication: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AnalyseIaAsset {
    pub asset: String,
    /// État de l'asset (2-3 phrases).
    pub etat: String,
    /// Le conseil actionnable — texte, jamais appliqué.
    pub conseil: String,
    /// 2-4 chiffres décisoires avec explication (fabriqués côté moteur).
    pub chiffres_cles: Vec<ChiffreCle>,
    /// Règle des 30 PAR ASSET.
    pub jugeable: bool,
    pub effectif: usize,
    /// Confiance de l'analyste, 0-100.
    pub confiance: u32,
    pub generee_le: i64,
}

#[derive(Deserialize)]
struct ReponseLlm {
    etat: String,
    #[serde(default)]
    conseil: String,
    #[serde(default)]
    confiance: f64,
}

static CACHE: OnceLock<RwLock<HashMap<String, Arc<AnalyseIaAsset>>>> = OnceLock::new();

fn cache() -> &'static RwLock<HashMap<String, Arc<AnalyseIaAsset>>> {
    CACHE.get_or_init(|| RwLock::new(HashMap::new()))
}

/// Clé de cache du jour : {strategie}:{asset}-{date locale}.
fn cle_du_jour(id: &str, asset: &str) -> String {
    format!("{}:{}-{}", id, asset, chrono::Local::now().format("%Y-%m-%d"))
}

/// POST /api/analyses/{strategie}/ia/asset/{asset} — génère (ou sert le
/// cache du jour) le conseil IA d'un asset.
pub async fn post_analyse_ia_asset(
    state: web::Data<AppState>,
    path: web::Path<(String, String)>,
) -> impl actix_web::Responder {
    let (id, asset) = path.into_inner();
    if !crate::registre_strategies::MANIFESTES.iter().any(|m| m.id == id) {
        return HttpResponse::NotFound()
            .json(serde_json::json!({ "error": "Stratégie inconnue" }));
    }
    if !CONSEILLABLES.contains(&id.as_str()) {
        return HttpResponse::NotFound().json(serde_json::json!({
            "error": "Stratégie sans réglages par asset (rockets) — pas de conseil par asset"
        }));
    }
    if asset.trim().is_empty() {
        return HttpResponse::BadRequest().json(serde_json::json!({ "error": "Asset manquant" }));
    }
    let cle = cle_du_jour(&id, &asset);
    if let Some(c) = cache().read().await.get(&cle) {
        return HttpResponse::Ok().json(serde_json::json!({ "en_cache": true, "analyse": *c.clone() }));
    }

    // Vécu de l'asset, depuis la même source que le rapport d'activité.
    let a = analyser(&state.db, &id).await;
    let vecu = a.assets.iter().find(|c| c.label == asset);
    let Some(vecu) = vecu else {
        // Honnêteté : rien à juger — réponse immédiate, sans LLM ni cache.
        return HttpResponse::Ok().json(serde_json::json!({
            "en_cache": false,
            "analyse": AnalyseIaAsset {
                asset: asset.clone(),
                etat: format!("Aucune clôture vécue sur {asset} pour cette stratégie — l'analyste n'a rien à juger."),
                conseil: String::new(),
                chiffres_cles: vec![],
                jugeable: false,
                effectif: 0,
                confiance: 0,
                generee_le: chrono::Utc::now().timestamp(),
            }
        }));
    };

    // Chiffres clés DÉTERMINISTES (fabriqués par le moteur, expliqués un à un).
    let surcharges = champs_surcharges(&state, &id, &asset).await;
    let meilleur_tf = a
        .par_asset_tf
        .iter()
        .find(|pa| pa.asset == asset)
        .and_then(|pa| (pa.tfs.len() > 1).then(|| pa.tfs.iter().max_by(|x, y| x.r.total_cmp(&y.r))))
        .flatten();
    let ecart = crate::simulation_recommandation::ecart_balayage_asset(&state, &id, &asset).await;
    let chiffres = construire_chiffres(vecu, &asset, meilleur_tf, &surcharges, ecart.as_ref());

    let prompt = format!(
        "{}\n\n{}",
        llm::prompt_effectif("analyse_rapport_asset"),
        contexte_asset(&state, &id, &asset, &a, vecu).await
    );
    match llm::ollama::interroger(&prompt).await {
        Ok(texte) => {
            let analyse = parser(texte, asset.clone(), vecu.n, chiffres);
            let arc = Arc::new(analyse);
            cache().write().await.insert(cle, arc.clone());
            HttpResponse::Ok().json(serde_json::json!({ "en_cache": false, "analyse": *arc }))
        }
        Err(e) => HttpResponse::ServiceUnavailable()
            .json(serde_json::json!({ "error": format!("Analyste indisponible : {e}") })),
    }
}

/// Contexte chiffré compact servi à l'analyste pour UN asset : vécu,
/// réglages courants (surcharge ⊕ défaut), écart balayage. Deux voix
/// (R DISTANCE / $), effectif vs règle des 30.
async fn contexte_asset(
    state: &AppState,
    id: &str,
    asset: &str,
    a: &AnalyseStrategie,
    vecu: &CategorieAnalyse,
) -> String {
    let mut l = Vec::new();
    l.push(format!(
        "STRATÉGIE {id} — ASSET {asset} — analyse PAR ASSET (source : {})",
        if a.source == "rejeu" { "re-jeu paramétrique" } else { "clôtures vécues" }
    ));
    // Vécu + règle des 30 par asset.
    l.push(format!(
        "VÉCU {asset} : {} clôtures · Σ R DISTANCE {:+.1} · {:.0} % ok · {:+.0} $ composés",
        vecu.n, vecu.r, vecu.wr * 100.0, vecu.dollars
    ));
    l.push(if vecu.n >= EFFECTIF_MIN {
        format!("Effectif {} ≥ {EFFECTIF_MIN} — JUGEABLE, conclusions chiffrées permises.", vecu.n)
    } else {
        format!(
            "Effectif {} < {EFFECTIF_MIN} — NON JUGEABLE (règle des 30 par asset) : reste descriptif, propose de rester au défaut global.",
            vecu.n
        )
    });
    // Détail par TF (SMC uniquement — le croisé asset×TF du rapport).
    if id == "SMC" {
        if let Some(pa) = a.par_asset_tf.iter().find(|pa| pa.asset == asset) {
            let tfs: Vec<String> = pa
                .tfs
                .iter()
                .map(|t| {
                    format!("{} ×{} ({:+.1} R dist, {:.0} % ok, {:+.0} $)", t.label, t.n, t.r, t.wr * 100.0, t.dollars)
                })
                .collect();
            if !tfs.is_empty() {
                l.push(format!("Par timeframe : {}", tfs.join(" ; ")));
            }
        }
    }
    l.push(format!("RÉGLAGES COURANTS : {}", lignes_reglages(state, id, asset).await));
    l.push(format!(
        "BALAYAGE LABO : {}",
        crate::simulation_recommandation::resume_balayage_asset(state, id, asset).await
    ));
    l.push(
        "Rappel des deux voix : R DISTANCE juge les entrées et TP ; $ est le résultat réel composé (réglages compris).".into(),
    );
    l.join("\n")
}

/// Réglages courants de l'asset en lignes compactes : chaque champ avec sa
/// valeur FUSIONNÉE, et « (surcharge — défaut X) » quand l'asset s'écarte
/// du défaut global. Rockets exclu en amont.
async fn lignes_reglages(state: &AppState, id: &str, asset: &str) -> String {
    match id {
        "SMC" => {
            let fusee = crate::simulation_recommandation::config_actuelle_asset(state, "SMC", Some(asset)).await;
            let global = crate::simulation_recommandation::config_actuelle_asset(state, "SMC", None).await;
            let s = db::reglages_asset::lire_smc_surcharge(state.db.pool(), asset)
                .await
                .unwrap_or_default();
            let mut parts: Vec<String> = ["tp1_mult", "tp2_mult", "tp3_mode", "tp3_rfixe", "tp3_trailing", "frac_tp1", "frac_tp2", "frac_tp3"]
                .iter()
                .filter_map(|k| ligne_reglage_json(*k, fusee.as_ref(), global.as_ref()))
                .collect();
            // Champ propre à la surcharge (pas de clé config équivalente).
            if let Some(m) = s.sl_max {
                parts.push(format!("sl_max={m}×ATR (surcharge — défaut : étalon par classe)"));
            }
            if parts.is_empty() { "réglages illisibles".into() } else { parts.join(" · ") }
        }
        "straddle" => {
            let g = db::strategies_params::lire_straddle_params(state.db.pool()).await;
            let s = db::reglages_asset::lire_straddle_surcharge(state.db.pool(), asset)
                .await
                .unwrap_or_default();
            [
                ligne_reglage("sl_mult", s.sl_mult, g.sl_mult),
                ligne_reglage("trailing_r", s.trailing_r, g.trailing_r),
                ligne_reglage("placement_sec", s.placement_sec.map(|v| v as f64), g.placement_sec as f64),
            ]
            .join(" · ")
        }
        _ => {
            // kdj_halftrend
            let g = db::kdj_params::lire_kdj_params(state.db.pool()).await;
            let s = db::reglages_asset::lire_kdj_surcharge(state.db.pool(), asset)
                .await
                .unwrap_or_default();
            [
                ligne_reglage("period", s.period.map(|v| v as f64), g.period as f64),
                ligne_reglage("signal", s.signal.map(|v| v as f64), g.signal as f64),
                ligne_reglage("amplitude", s.amplitude.map(|v| v as f64), g.amplitude as f64),
                ligne_reglage("ratio_risk", s.ratio_risk, g.ratio_risk),
                ligne_reglage("adx_min", s.adx_min, g.adx_min),
            ]
            .join(" · ")
        }
    }
}

/// `tp1_mult=0.6` ou `tp3_trailing=0.2 (surcharge — défaut 0.1)` — depuis
/// les config JSON (fused vs global) du module de recommandation.
fn ligne_reglage_json(
    cle: &str,
    fusee: Option<&serde_json::Value>,
    global: Option<&serde_json::Value>,
) -> Option<String> {
    let f = fusee?.get(cle)?;
    let g = global.and_then(|g| g.get(cle));
    let surcharge = g.map(|g| g != f).unwrap_or(false);
    Some(match g {
        Some(gv) if surcharge => format!("{cle}={} (surcharge — défaut {})", compact_json(f), compact_json(gv)),
        _ => format!("{cle}={}", compact_json(f)),
    })
}

/// `ligne_reglage` pour les champs typés f64 des souches straddle/KDJ.
fn ligne_reglage(nom: &str, surcharge: Option<f64>, defaut: f64) -> String {
    match surcharge {
        Some(v) => format!("{nom}={} (surcharge — défaut {})", coupe(v), coupe(defaut)),
        None => format!("{nom}={}", coupe(defaut)),
    }
}

/// Valeur JSON compacte : 0.6 (pas 0.6000001), chaîne telle quelle.
fn compact_json(v: &serde_json::Value) -> String {
    match v.as_f64() {
        Some(f) => coupe(f),
        None => v.to_string(),
    }
}

/// 0.6000000000000001 → « 0.6 » (affichage compact, pas un arrondi de calcul).
fn coupe(f: f64) -> String {
    let s = format!("{f:.4}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// Champs RÉELLEMENT surchargés de l'asset : "nom valeur (défaut X)".
/// Vide = aucun — l'asset vit au défaut global.
async fn champs_surcharges(state: &AppState, id: &str, asset: &str) -> Vec<String> {
    match id {
        "SMC" => {
            let fusee = crate::simulation_recommandation::config_actuelle_asset(state, "SMC", Some(asset)).await;
            let global = crate::simulation_recommandation::config_actuelle_asset(state, "SMC", None).await;
            let mut v: Vec<String> = [
                ("tp1_mult", "TP1"), ("tp2_mult", "TP2"), ("tp3_mode", "TP3"),
                ("tp3_rfixe", "TP3 R fixe"), ("tp3_trailing", "trailing"),
                ("frac_tp1", "vente à TP1"), ("frac_tp2", "vente à TP2"), ("frac_tp3", "solde TP3"),
            ]
            .iter()
            .filter_map(|(k, nom)| {
                let f = fusee.as_ref()?.get(*k)?;
                let g = global.as_ref().and_then(|g| g.get(*k))?;
                (g != f).then(|| format!("{nom} {} (défaut {})", compact_json(f), compact_json(g)))
            })
            .collect();
            // sl_max : champ propre à la surcharge (pas de clé config).
            if let Ok(s) = db::reglages_asset::lire_smc_surcharge(state.db.pool(), asset).await {
                if let Some(m) = s.sl_max {
                    v.push(format!("SL max {m}×ATR (défaut : étalon par classe)"));
                }
            }
            v
        }
        "straddle" => {
            let g = db::strategies_params::lire_straddle_params(state.db.pool()).await;
            let s = db::reglages_asset::lire_straddle_surcharge(state.db.pool(), asset)
                .await
                .unwrap_or_default();
            [
                diff_surcharge("SL", s.sl_mult, g.sl_mult),
                diff_surcharge("trailing", s.trailing_r, g.trailing_r),
                diff_surcharge("placement", s.placement_sec.map(|v| v as f64), g.placement_sec as f64),
            ]
            .into_iter()
            .flatten()
            .collect()
        }
        _ => {
            // kdj_halftrend
            let fusee = crate::simulation_recommandation::config_actuelle_asset(state, "kdj_halftrend", Some(asset)).await;
            let global = crate::simulation_recommandation::config_actuelle_asset(state, "kdj_halftrend", None).await;
            let mut v: Vec<String> = [
                ("period", "period"), ("signal", "signal"), ("amplitude", "amplitude"), ("adx_min", "ADX min"),
            ]
            .iter()
            .filter_map(|(k, nom)| {
                let f = fusee.as_ref()?.get(*k)?;
                let g = global.as_ref().and_then(|g| g.get(*k))?;
                (g != f).then(|| format!("{nom} {} (défaut {})", compact_json(f), compact_json(g)))
            })
            .collect();
            if let Ok(s) = db::reglages_asset::lire_kdj_surcharge(state.db.pool(), asset).await {
                if let (Some(rr), g) = (s.ratio_risk, db::kdj_params::lire_kdj_params(state.db.pool()).await) {
                    if let Some(d) = diff_surcharge("ratio risque", Some(rr), g.ratio_risk) {
                        v.push(d);
                    }
                }
            }
            v
        }
    }
}

/// "nom v (défaut d)" si surchargé et différent du défaut, rien sinon.
fn diff_surcharge(nom: &str, surcharge: Option<f64>, defaut: f64) -> Option<String> {
    surcharge.filter(|v| (*v - defaut).abs() > 1e-9).map(|v| {
        format!("{nom} {} (défaut {})", coupe(v), coupe(defaut))
    })
}

/// Les chiffres clés de la modale — un à un, chacun avec sa phrase.
/// Pure : testée isolément, aucune surprise de LLM ici.
fn construire_chiffres(
    vecu: &CategorieAnalyse,
    asset: &str,
    meilleur_tf: Option<&CategorieAnalyse>,
    surcharges: &[String],
    ecart: Option<&crate::simulation_recommandation::EcartBalayageAsset>,
) -> Vec<ChiffreCle> {
    let mut v = vec![ChiffreCle {
        chiffre: format!(
            "{} clôtures · {:+.1} R distance · {:.0} % gagnants · {:+.0} $",
            vecu.n, vecu.r, vecu.wr * 100.0, vecu.dollars
        ),
        explication: format!(
            "Tout l'historique fermé de {asset} : le R distance ({:+.1}) dit si les entrées et les objectifs sont bien placés ; « gagnants » = part des trades clôturés en gain ; le $ est l'argent réellement encaissé, réglages compris.",
            vecu.r
        ),
    }];
    if let Some(tf) = meilleur_tf {
        v.push(ChiffreCle {
            chiffre: format!("{} : {:+.1} R distance sur {} clôtures ({:.0} % gagnants)", tf.label, tf.r, tf.n, tf.wr * 100.0),
            explication: format!("Le vécu de {asset} détaillé par timeframe — celui-ci en est le principal poste."),
        });
    }
    if !surcharges.is_empty() {
        v.push(ChiffreCle {
            chiffre: surcharges.join(" · "),
            explication: format!("Réglage propre à {asset} — les autres assets gardent la valeur par défaut."),
        });
    }
    if let Some(e) = ecart {
        v.push(ChiffreCle {
            chiffre: format!(
                "Labo : {} → {:+.1} R{} ({} trades rejoués)",
                e.params,
                e.r_total,
                e.r_actuel.map(|a| format!(" contre {a:+.1} R en réglages actuels")).unwrap_or_default(),
                e.nb_trades
            ),
            explication: e.delta()
                .map(|d| format!("Les MÊMES trades, rejoués au labo avec cette configuration : {d:+.1} R de mieux qu'avec les réglages actuels — une indication d'étude, pas une promesse."))
                .unwrap_or_else(|| "Les MÊMES trades rejoués au labo avec cette configuration — le labo n'a pas d'essai aux réglages actuels pour comparer.".into()),
        });
    }
    v
}

/// Extraction robuste, même discipline que l'analyste global : premier
/// `{` dernier `}` ; repli = texte brut dans « etat ». Les chiffres clés
/// sont fabriqués par le moteur et passés tels quels au résultat.
fn parser(texte: String, asset: String, effectif: usize, chiffres: Vec<ChiffreCle>) -> AnalyseIaAsset {
    let nettoyee = texte.trim().to_string();
    let jugeable = effectif >= EFFECTIF_MIN;
    let base = |etat: String, conseil: String, confiance: u32| AnalyseIaAsset {
        asset: asset.clone(),
        etat,
        conseil,
        chiffres_cles: chiffres.clone(),
        jugeable,
        effectif,
        confiance,
        generee_le: chrono::Utc::now().timestamp(),
    };
    let debut = nettoyee.find('{');
    let fin = nettoyee.rfind('}');
    if let (Some(d), Some(f)) = (debut, fin) {
        if f > d {
            if let Ok(vue) = serde_json::from_str::<ReponseLlm>(&nettoyee[d..=f]) {
                return base(vue.etat, vue.conseil, confiance_normalisee(vue.confiance));
            }
        }
    }
    base(nettoyee.chars().take(2000).collect(), String::new(), 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chiffre_test() -> Vec<ChiffreCle> {
        vec![ChiffreCle {
            chiffre: "45 clôtures · +13.3 R distance".into(),
            explication: "Tout l'historique fermé de XAUUSD.".into(),
        }]
    }

    #[test]
    fn parse_json_environne_de_texte() {
        let a = parser(
            "Voici le conseil :\n{\"etat\":\"asset solide\",\"conseil\":\"garder le défaut\",\"confiance\":70}\nCordialement".into(),
            "XAUUSD".into(),
            45,
            chiffre_test(),
        );
        assert_eq!(a.asset, "XAUUSD");
        assert_eq!(a.etat, "asset solide");
        assert_eq!(a.conseil, "garder le défaut");
        assert_eq!(a.confiance, 70);
        assert!(a.jugeable);
        assert_eq!(a.effectif, 45);
        // Les chiffres passent tels quels (fabriqués par le moteur).
        assert_eq!(a.chiffres_cles.len(), 1);
        assert_eq!(a.chiffres_cles[0].chiffre, "45 clôtures · +13.3 R distance");
    }

    #[test]
    fn non_jugeable_sous_30_et_repli_texte_brut() {
        let a = parser("Le vécu de BTC est trop maigre pour conclure.".into(), "BTC".into(), 12, vec![]);
        assert!(!a.jugeable);
        assert_eq!(a.effectif, 12);
        assert!(a.etat.contains("maigre"));
        assert_eq!(a.conseil, "");
        assert_eq!(a.confiance, 0);
        assert!(a.chiffres_cles.is_empty());
    }

    #[test]
    fn confiance_decimale_normalisee() {
        let a = parser(
            "{\"etat\":\"x\",\"conseil\":\"\",\"confiance\":0.6}".into(),
            "DAX".into(),
            60,
            vec![],
        );
        assert_eq!(a.confiance, 60);
    }

    #[test]
    fn reglage_surcharge_marquee_avec_defaut() {
        let fusee = serde_json::json!({"tp3_trailing": 0.2, "tp1_mult": 0.6});
        let global = serde_json::json!({"tp3_trailing": 0.1, "tp1_mult": 0.6});
        assert_eq!(
            ligne_reglage_json("tp3_trailing", Some(&fusee), Some(&global)).unwrap(),
            "tp3_trailing=0.2 (surcharge — défaut 0.1)"
        );
        // Identique au défaut : pas de marque de surcharge.
        assert_eq!(
            ligne_reglage_json("tp1_mult", Some(&fusee), Some(&global)).unwrap(),
            "tp1_mult=0.6"
        );
    }

    #[test]
    fn chiffres_toujours_expliques_un_a_un() {
        let vecu = CategorieAnalyse { label: "XAUUSD".into(), n: 45, dollars: 850.0, r: 13.3, wr: 0.58 };
        let tf = CategorieAnalyse { label: "M5".into(), n: 30, dollars: 620.0, r: 11.3, wr: 0.70 };
        let ecart = crate::simulation_recommandation::EcartBalayageAsset {
            params: "frac_tp1=1".into(),
            r_total: 12.4,
            nb_trades: 45,
            r_actuel: Some(11.2),
        };
        let cs = construire_chiffres(&vecu, "XAUUSD", Some(&tf), &["trailing 0.2 (défaut 0.1)".to_string()], Some(&ecart));
        assert_eq!(cs.len(), 4);
        for c in &cs {
            assert!(!c.chiffre.is_empty(), "chiffre fourni");
            assert!(!c.explication.is_empty(), "chaque chiffre porte son explication");
        }
        assert!(cs[0].chiffre.contains("45 clôtures"));
        assert!(cs[0].explication.contains("R distance"));
        assert!(cs[1].chiffre.contains("M5"));
        assert!(cs[2].chiffre.contains("trailing 0.2"));
        assert!(cs[2].explication.contains("Réglage propre"));
        assert!(cs[3].chiffre.contains("contre +11.2 R"));
        assert!(cs[3].explication.contains("+1.2 R"));
    }

    #[test]
    fn chiffres_minimaux_sans_tf_ni_surcharge_ni_ecart() {
        let vecu = CategorieAnalyse { label: "BNB".into(), n: 3, dollars: -40.0, r: -1.2, wr: 0.33 };
        let cs = construire_chiffres(&vecu, "BNB", None, &[], None);
        assert_eq!(cs.len(), 1, "vécu seul — pas de remplissage artificiel");
    }
}
