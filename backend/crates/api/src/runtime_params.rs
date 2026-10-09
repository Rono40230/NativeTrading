//! Hot-reload des paramètres moteur (09/10, design validé propriétaire) —
//! un changement de paramètre en base (carte Paramètres de l'app) s'applique
//! aux moteurs sous 60 s, sans redémarrage ni re-armement manuel.
//!
//! Mécanique = généralisation de `reglages_smc::retirer_changements_armement` :
//! chaque tick compare l'EMPREINTE des params de chaque couple aux familles
//! qu'il héberge (SMC / straddle / KDJ) ; une diff ⇒ le couple est retiré
//! puis réinscrit par la boucle d'ajouts existante (replay complet, chauffe
//! KDJ 600 H1 incluse). Zéro nouveau chemin d'exécution.
//!
//! Garde straddle : une passe dans sa fenêtre active (annonce à moins de
//! 30 min dans le passé, ou il y a moins de 8 h — couvre placement +
//! time-stop) DIFFÈRE le ré-armement au tick suivant ; l'empreinte
//! précédente est conservée pour re-test.
//!
//! Hors scope volontaire : capital, risque, fractions — le vécu est
//! immuable par conception (décision 24/09).

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use common::{Asset, Timeframe};

/// Fenêtre active d'une passe straddle : [T − 30 min ; T + 8 h].
pub const FENETRE_AVANT_SEC: i64 = 30 * 60;
pub const FENETRE_APRES_SEC: i64 = 8 * 3600;

/// Une annonce est-elle dans sa fenêtre de passe active à `maintenant` ?
pub fn fenetre_active(ts_annonce: i64, maintenant: i64) -> bool {
    maintenant >= ts_annonce - FENETRE_AVANT_SEC && maintenant <= ts_annonce + FENETRE_APRES_SEC
}

/// Familles de params d'un couple, concaténées en une empreinte.
pub fn empreinte_couple(
    smc: Option<String>,
    straddle: Option<String>,
    kdj: Option<String>,
) -> String {
    let mut s = String::new();
    if let Some(x) = smc {
        s.push_str(&format!("smc[{x}]"));
    }
    if let Some(x) = straddle {
        s.push_str(&format!("str[{x}]"));
    }
    if let Some(x) = kdj {
        s.push_str(&format!("kdj[{x}]"));
    }
    s
}

/// Décision PURE (testable) : couples à retirer pour ré-armement.
///
/// `precedente`/`voulues` : empreintes par clé "ASSET:TF". Les clés des
/// couples M1 dont une annonce est en fenêtre active voient leur empreinte
/// voulue REMPLACÉE par la précédente (différé — aucun retrait).
pub fn couples_a_retirer(
    precedente: &HashMap<String, String>,
    voulues: &HashMap<String, String>,
    fenetres: &HashMap<String, Vec<i64>>,
    maintenant: i64,
) -> (Vec<String>, HashMap<String, String>) {
    let mut effective: HashMap<String, String> = voulues.clone();
    for cle in voulues.keys() {
        let Some((asset, tf)) = cle.split_once(':') else { continue };
        if tf != "M1" {
            continue;
        }
        let en_fenetre = fenetres
            .get(asset)
            .map_or(false, |ts| ts.iter().any(|&t| fenetre_active(t, maintenant)));
        if en_fenetre {
            if let Some(p) = precedente.get(cle) {
                effective.insert(cle.clone(), p.clone());
            }
        }
    }
    let mut a_retirer = Vec::new();
    for (cle, e) in &effective {
        let change = precedente.get(cle).map_or(false, |p| p != e);
        if change {
            a_retirer.push(cle.clone());
        }
    }
    (a_retirer, effective)
}

/// Retire du runtime les couples dont l'empreinte params a changé (la boucle
/// d'ajouts du même tick les reconstruit avec les nouveaux params).
/// `fenetres_straddle` : timestamps d'annonces (tier 1 + événements) par asset.
pub fn retirer_changements_params(
    runtime: &mut engine::Runtime,
    voulues: HashMap<(Asset, Timeframe), String>,
    fenetres_straddle: &HashMap<String, Vec<i64>>,
) {
    static MEM: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
    let garde = MEM.get_or_init(|| Mutex::new(HashMap::new()));

    let voulues_cles: HashMap<String, String> = voulues
        .iter()
        .map(|((a, t), e)| (format!("{}:{}", a.as_str(), t.as_str()), e.clone()))
        .collect();
    let maintenant = chrono::Utc::now().timestamp();

    let (a_retirer, effective) = {
        let mut g = garde.lock().unwrap_or_else(|e| e.into_inner());
        let precedente = g.clone();
        let (a_retirer, effective) =
            couples_a_retirer(&precedente, &voulues_cles, fenetres_straddle, maintenant);
        *g = effective;
        (a_retirer, ())
    };
    let _ = effective;

    for cle in a_retirer {
        let Some((a, t)) = cle.split_once(':') else { continue };
        if let (Ok(asset), Ok(tf)) = (Asset::try_from(a), Timeframe::try_from(t)) {
            if runtime.cles().contains(&(asset.clone(), tf)) {
                runtime.retirer(asset.clone(), tf);
                tracing::info!("Runtime tick: {a} {t} réinscrit (paramètres moteur modifiés — hot-reload)");
            }
        }
    }
}

/// Construit la carte des fenêtres d'annonces par asset du périmètre straddle
/// (tier 1 + événements armés).
pub async fn fenetres_straddle(
    db: &std::sync::Arc<db::Database>,
    perimetre: &[String],
) -> HashMap<String, Vec<i64>> {
    let mut carte: HashMap<String, Vec<i64>> = HashMap::new();
    let tier1 = crate::runtime_amorces::annonces_tier1(db).await;
    for a in perimetre {
        let mut ts: Vec<i64> = tier1.iter().map(|x| x.ts).collect();
        ts.extend(crate::evenements_armement::annonces_evenements(db, a).await.iter().map(|x| x.ts));
        carte.insert(a.clone(), ts);
    }
    carte
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Format flottant stable pour les empreintes (évite les surprises de
    /// représentation entre ticks).
    fn fmt(x: f64) -> String {
        format!("{x:.4}")
    }

    fn hm(paires: &[(&str, &str)]) -> HashMap<String, String> {
        paires.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect()
    }

    #[test]
    fn fenetre_active_bornes() {
        let t = 1_000_000i64;
        assert!(fenetre_active(t, t), "à l'instant de l'annonce");
        assert!(fenetre_active(t, t - FENETRE_AVANT_SEC), "borne T−30 min incluse");
        assert!(!fenetre_active(t, t - FENETRE_AVANT_SEC - 1), "avant la fenêtre");
        assert!(fenetre_active(t, t + FENETRE_APRES_SEC), "borne T+8 h incluse");
        assert!(!fenetre_active(t, t + FENETRE_APRES_SEC + 1), "après la fenêtre");
    }

    #[test]
    fn empreinte_stable_et_sensible() {
        let a = empreinte_couple(
            Some(format!("tp1={}", fmt(1.0))),
            Some("sl=0.5".into()),
            None,
        );
        let b = empreinte_couple(
            Some(format!("tp1={}", fmt(1.0))),
            Some("sl=0.5".into()),
            None,
        );
        assert_eq!(a, b, "mêmes params ⇒ même empreinte");
        let c = empreinte_couple(
            Some(format!("tp1={}", fmt(1.1))),
            Some("sl=0.5".into()),
            None,
        );
        assert_ne!(a, c, "un paramètre change ⇒ empreinte différente");
        assert!(empreinte_couple(None, None, None).is_empty());
    }

    #[test]
    fn retrait_sur_changement_uniquement() {
        let prec = hm(&[("BTC:M15", "e1"), ("ETH:H1", "e2")]);
        let voulu = hm(&[("BTC:M15", "e1"), ("ETH:H1", "e3"), ("XRP:M15", "e9")]);
        let (retirer, _) = couples_a_retirer(&prec, &voulu, &HashMap::new(), 0);
        assert_eq!(retirer, vec!["ETH:H1".to_string()], "seul ETH a changé (XRP est nouveau : pas dans le runtime)");
    }

    #[test]
    fn defere_passe_en_fenetre_active() {
        let prec = hm(&[("DAX:M1", "e1")]);
        let voulu = hm(&[("DAX:M1", "e2")]);
        let now = 1_000_000i64;
        let fenetres = HashMap::from([("DAX".to_string(), vec![now + 600i64])]); // annonce dans 10 min
        let (retirer, effective) = couples_a_retirer(&prec, &voulu, &fenetres, now);
        assert!(retirer.is_empty(), "passe imminente ⇒ retrait différé");
        assert_eq!(effective.get("DAX:M1").unwrap(), "e1", "l'ancienne empreinte est mémorisée pour re-test");
        // Fenêtre refermée (annonce il y a plus de 8 h) ⇒ le retrait part.
        let fenetres_passees = HashMap::from([("DAX".to_string(), vec![now - FENETRE_APRES_SEC - 10])]);
        let (retirer, _) = couples_a_retirer(&prec, &voulu, &fenetres_passees, now);
        assert_eq!(retirer, vec!["DAX:M1".to_string()]);
    }

    #[test]
    fn non_m1_ne_depend_pas_des_fenetres() {
        let prec = hm(&[("BTC:H1", "e1")]);
        let voulu = hm(&[("BTC:H1", "e2")]);
        let now = 1_000_000i64;
        let fenetres = HashMap::from([("BTC".to_string(), vec![now])]);
        let (retirer, _) = couples_a_retirer(&prec, &voulu, &fenetres, now);
        assert_eq!(retirer, vec!["BTC:H1".to_string()], "la garde ne concerne que le rail M1 straddle");
    }
}
