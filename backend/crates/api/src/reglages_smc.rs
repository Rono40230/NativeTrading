//! Lecteurs des réglages SMC (config kv) — partagés entre l'armement du
//! runtime (runtime_tick) et le re-jeu paramétrique (smc_rejeu). Extraits de
//! runtime_tick.rs (limite 600 lignes, pre-commit).

/// TP3 réglable : mode (true = liquidité lointaine, défaut) + R fixe/repli
/// (défaut 3.0, borné 3-10 — validation > TP2 côté carte).
pub async fn lire_tp3_reglage(db: &db::Database) -> smc::v12::signals::Tp3Reglage {
    let lointaine = db
        .lire_config("smc_tp3_mode")
        .await
        .ok()
        .flatten()
        .map(|v| !v.trim().eq_ignore_ascii_case("rfixe"))
        .unwrap_or(true);
    let rfixe = db
        .lire_config("smc_tp3_rfixe")
        .await
        .ok()
        .flatten()
        .and_then(|v| v.trim().parse::<f64>().ok())
        .map(|v| v.clamp(3.0, 10.0))
        .unwrap_or(3.0);
    smc::v12::signals::Tp3Reglage { lointaine, rfixe }
}

/// Trailing stop après TP2 (défaut inactif — mesuré par le re-jeu avant
/// d'en faire éventuellement un défaut).
pub async fn lire_trailing_reglage(db: &db::Database) -> Option<f64> {
    let actif = db
        .lire_config("smc_tp3_trailing")
        .await
        .ok()
        .flatten()
        .map(|v| v.trim() == "1")
        .unwrap_or(false);
    if !actif {
        return None;
    }
    Some(
        db.lire_config("smc_tp3_trailing_r")
            .await
            .ok()
            .flatten()
            .and_then(|v| v.trim().parse::<f64>().ok())
            .map(|v| v.clamp(0.1, 1.0))
            .unwrap_or(0.5),
    )
}

use std::collections::{HashMap, HashSet};

/// TF sur lesquels le moteur v12 GÉNÈRE des signaux (décision 04/09 sur le
/// comparatif 24 mois : H1 désarmé — il reste COLLECTÉ pour l'amorce MTF,
/// mais ne génère plus jamais de signaux).
pub const TF_GENERATEURS: [&str; 4] = ["M1", "M5", "M15", "M30"];

/// Clé kv de l'armement SMC par couple — JSON `{"XAUUSD":["M1","M15"],...}`.
/// Asset absent de la map = TOUS les TF_GENERATEURS armés (défaut plein,
/// compatibilité : sans clé, comportement d'avant moins H1).
const CLE_COUPLES_ARMES: &str = "smc_couples_armes";

/// Armement SMC lu de la config. Vide/illisible = défauts.
pub async fn lire_couples_armes(db: &db::Database) -> HashMap<String, HashSet<String>> {
    db.lire_config(CLE_COUPLES_ARMES)
        .await
        .ok()
        .flatten()
        .and_then(|v| serde_json::from_str(&v).ok())
        .unwrap_or_default()
}

/// Le couple (asset, tf) génère-t-il des signaux SMC ?
/// Hors TF_GENERATEURS (H1…) → jamais ; asset non configuré → tout armé.
pub fn est_arme(armes: &HashMap<String, HashSet<String>>, asset: &str, tf: &str) -> bool {
    if !TF_GENERATEURS.contains(&tf) {
        return false;
    }
    match armes.get(asset) {
        Some(set) => set.contains(tf),
        None => true,
    }
}

/// Empreinte stable de l'armement — invalide le cache du re-jeu quand la
/// liste bouge (les métriques suivent le périmètre armé).
pub fn empreinte_couples(armes: &HashMap<String, HashSet<String>>) -> String {
    let mut lignes: Vec<String> = armes
        .iter()
        .map(|(a, set)| {
            let mut tfs: Vec<&str> = set.iter().map(|s| s.as_str()).collect();
            tfs.sort_unstable();
            format!("{a}:{}", tfs.join("+"))
        })
        .collect();
    lignes.sort_unstable();
    lignes.join("|")
}

/// Écrit l'armement (JSON trié, TF non générateurs écartés).
pub async fn ecrire_couples_armes(
    db: &db::Database,
    armes: &HashMap<String, Vec<String>>,
) -> anyhow::Result<()> {
    let triees: HashMap<String, Vec<String>> = armes
        .iter()
        .map(|(a, tfs)| {
            let mut v: Vec<String> = tfs
                .iter()
                .filter(|t| TF_GENERATEURS.contains(&t.as_str()))
                .cloned()
                .collect();
            v.sort_unstable();
            (a.clone(), v)
        })
        .collect();
    db.ecrire_config(CLE_COUPLES_ARMES, &serde_json::to_string(&triees)?)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))
}

/// Retire du runtime les couples dont l'armement SMC vient de CHANGER
/// (armé→désarmé ou l'inverse) — la boucle d'ajouts de synchroniser_config
/// les réinscrit immédiatement avec les moteurs voulus (v12 si armé,
/// straddle M1 toujours ; replay + réconciliation au réarmement).
/// Signature persistée entre synchronisations : au boot elle est vide, donc
/// rien n'est retiré ; ensuite seul le différentiel bouge.
pub fn retirer_changements_armement(
    runtime: &mut engine::Runtime,
    cibles: &std::collections::HashSet<(common::Asset, common::Timeframe)>,
    armes: &HashMap<String, HashSet<String>>,
) {
    static SIG: std::sync::OnceLock<std::sync::Mutex<HashSet<String>>> = std::sync::OnceLock::new();
    let vivante: HashSet<String> = cibles
        .iter()
        .filter(|(a, t)| est_arme(armes, a.as_str(), t.as_str()))
        .map(|(a, t)| format!("{}:{}", a.as_str(), t.as_str()))
        .collect();
    let garde = SIG.get_or_init(|| std::sync::Mutex::new(HashSet::new()));
    let ancienne = {
        let mut g = garde.lock().unwrap_or_else(|e| e.into_inner());
        std::mem::replace(&mut *g, vivante.clone())
    };
    for cle in ancienne.symmetric_difference(&vivante) {
        let Some((a, t)) = cle.split_once(':') else {
            continue;
        };
        if let (Ok(asset), Ok(tf)) = (common::Asset::try_from(a), common::Timeframe::try_from(t)) {
            if runtime.cles().contains(&(asset.clone(), tf)) {
                runtime.retirer(asset.clone(), tf);
                tracing::info!("Runtime tick: {a} {t} réinscrit (armement SMC changé)");
            }
        }
    }
}

pub async fn lire_tp2_reglage(db: &db::Database) -> f64 {
    db.lire_config("smc_tp2_mult")
        .await
        .ok()
        .flatten()
        .and_then(|v| v.trim().parse::<f64>().ok())
        .map(|v| v.clamp(1.0, 4.0))
        .unwrap_or(2.0)
}

pub async fn lire_tp1_reglage(db: &db::Database) -> f64 {
    db.lire_config("smc_tp1_mult")
        .await
        .ok()
        .flatten()
        .and_then(|v| v.trim().parse::<f64>().ok())
        .map(|v| v.clamp(0.2, 1.5))
        .unwrap_or(0.6)
}


/// Fractions des ventes partielles (config smc_frac_tp1/tp2/tp3, défauts
/// 50/30/20). Robustesse : valeurs invalides ou Σ ≠ 100 % → défauts (la
/// validation stricte vit côté carte SMC).
pub async fn lire_fractions(db: &db::Database) -> crate::smc_pondere::Fractions {
    let f1 = db
        .lire_config("smc_frac_tp1")
        .await
        .ok()
        .flatten()
        .and_then(|v| v.trim().parse::<f64>().ok())
        .unwrap_or(0.5);
    let f2 = db
        .lire_config("smc_frac_tp2")
        .await
        .ok()
        .flatten()
        .and_then(|v| v.trim().parse::<f64>().ok())
        .unwrap_or(0.3);
    let f3 = db
        .lire_config("smc_frac_tp3")
        .await
        .ok()
        .flatten()
        .and_then(|v| v.trim().parse::<f64>().ok())
        .unwrap_or(0.2);
    let somme = f1 + f2 + f3;
    if (somme - 1.0).abs() > 1e-6 || f1 < 0.0 || f2 < 0.0 || f3 < 0.0 {
        return crate::smc_pondere::Fractions::default();
    }
    crate::smc_pondere::Fractions { tp1: f1, tp2: f2, tp3: f3 }
}

// ── Endpoints de l'outil « Timeframes par asset » (Paramètres › SMC) ─────────

use actix_web::{web, HttpResponse};
use crate::state::AppState;

/// GET /api/smc/couples — grille d'armement : périmètre, TF générateurs et
/// armement EFFECTIF par asset (défauts appliqués).
pub async fn get_couples(state: web::Data<AppState>) -> impl actix_web::Responder {
    let armes = lire_couples_armes(&state.db).await;
    let assets = crate::runtime_tick::assets_runtime(&state.db).await;
    let effective: std::collections::BTreeMap<String, Vec<&'static str>> = assets
        .iter()
        .map(|a| {
            let tfs: Vec<&'static str> = TF_GENERATEURS
                .iter()
                .filter(|t| est_arme(&armes, a.as_str(), t))
                .copied()
                .collect();
            (a.as_str().to_string(), tfs)
        })
        .collect();
    HttpResponse::Ok().json(serde_json::json!({
        "tfs": TF_GENERATEURS,
        "assets": effective.keys().collect::<Vec<_>>(),
        "armes": effective,
    }))
}

/// PUT /api/smc/couples — enregistre l'armement et relance le re-jeu
/// (métriques = périmètre armé). Le runtime resynchronise ses couples en
/// ≤ 60 s (RELECTURE_CONFIG_SEC).
pub async fn put_couples(
    state: web::Data<AppState>,
    body: web::Json<HashMap<String, Vec<String>>>,
) -> impl actix_web::Responder {
    let assets = crate::runtime_tick::assets_runtime(&state.db).await;
    let inconnus: Vec<&String> = body
        .keys()
        .filter(|a| !assets.iter().any(|x| x.as_str() == a.as_str()))
        .collect();
    if !inconnus.is_empty() {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": format!("Asset(s) hors périmètre : {}", inconnus.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", "))
        }));
    }
    if let Err(e) = ecrire_couples_armes(&state.db, &body).await {
        return HttpResponse::InternalServerError()
            .json(serde_json::json!({ "error": e.to_string() }));
    }
    crate::smc_rejeu::lancer_si_necessaire(state.db.clone()).await;
    tracing::info!(
        "Armement SMC enregistré ({} asset(s)) — rejeu relancé, resynchro runtime ≤ 60 s",
        body.len()
    );
    HttpResponse::Ok().json(serde_json::json!({ "ok": true }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(v: &[&str]) -> HashSet<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn h1_jamais_generateur() {
        let mut armes = HashMap::new();
        armes.insert("XAUUSD".to_string(), set(&["M1", "H1"]));
        assert!(est_arme(&armes, "XAUUSD", "M1"));
        assert!(!est_arme(&armes, "XAUUSD", "H1"), "H1 hors générateurs même armé explicitement");
        assert!(!est_arme(&HashMap::new(), "XAUUSD", "H1"));
    }

    #[test]
    fn defaut_asset_absent_tout_arme() {
        let mut armes = HashMap::new();
        armes.insert("NAS100".to_string(), set(&["M15"]));
        assert!(est_arme(&armes, "XAUUSD", "M1"), "asset absent = défaut plein");
        assert!(!est_arme(&armes, "NAS100", "M1"), "config NAS100 retire M1");
        assert!(est_arme(&armes, "NAS100", "M15"));
    }

    #[test]
    fn empreinte_depend_du_contenu_pas_de_l_ordre() {
        let mut a = HashMap::new();
        a.insert("X".to_string(), set(&["M5", "M1"]));
        let mut b = HashMap::new();
        b.insert("X".to_string(), set(&["M1", "M5"]));
        assert_eq!(empreinte_couples(&a), empreinte_couples(&b));
        b.insert("X".to_string(), set(&["M1"]));
        assert_ne!(empreinte_couples(&a), empreinte_couples(&b));
    }
}
