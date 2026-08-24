//! Étape 3 (dette) — Setups en formation : la face APP des annonces
//! d'imminence. Le writer officiel enregistre chaque setup annoncé intrabar ;
//! l'API sert la liste vivante (en formation / confirmé / dissipé) au panneau
//! du dashboard et de la page Signaux. Telegram n'est plus le seul à savoir.

use actix_web::{web, HttpResponse, Responder};
use serde::Serialize;
use std::sync::{Mutex, OnceLock};

use crate::state::AppState;

#[derive(Debug, Clone, Serialize)]
pub struct SetupFormation {
    pub strategie: String,
    pub asset: String,
    pub tf: String,
    pub direction: String,
    pub force: i32,
    pub entree: f64,
    pub sl: f64,
    pub tps: Vec<f64>,
    pub cle: String,
    /// Début de la bougie en formation (epoch s).
    pub debut_barre: i64,
    /// Clôture attendue de la bougie (epoch s) — l'heure de vérité.
    pub cloture_barre: i64,
    pub ts_annonce: i64,
    /// EnFormation | Confirme | Dissipe (résolu paresseusement au lecture).
    pub statut: String,
}

fn etat() -> &'static Mutex<Vec<SetupFormation>> {
    static ETAT: OnceLock<Mutex<Vec<SetupFormation>>> = OnceLock::new();
    ETAT.get_or_init(|| Mutex::new(Vec::new()))
}

fn tf_en_secondes(tf: &str) -> i64 {
    match tf {
        "M1" => 60,
        "M5" => 300,
        "M15" => 900,
        "M30" => 1800,
        "H1" => 3600,
        "H4" => 14400,
        "D1" => 86400,
        "W1" => 604800,
        _ => 900,
    }
}

/// Enregistre une annonce intrabar (appelé par le writer officiel).
pub fn enregistrer_annonce(setup: SetupFormation) {
    let mut liste = etat().lock().unwrap_or_else(|e| e.into_inner());
    // Même clé = mise à jour (force/niveaux peuvent affiner en cours de barre).
    if let Some(existing) = liste
        .iter_mut()
        .find(|s| s.cle == setup.cle && s.strategie == setup.strategie)
    {
        *existing = setup;
    } else {
        liste.push(setup);
    }
    // Hygiène : on garde au plus 40 entrées (les plus récentes).
    if liste.len() > 40 {
        let excedent = liste.len() - 40;
        liste.drain(..excedent);
    }
}

/// Marque un setup confirmé (appelé quand la ligne officielle s'insère).
pub fn marquer_confirme(strategie: &str, cle: &str) {
    let mut liste = etat().lock().unwrap_or_else(|e| e.into_inner());
    if let Some(s) = liste
        .iter_mut()
        .find(|s| s.cle == cle && s.strategie == strategie)
    {
        s.statut = "Confirme".into();
    }
}

/// Résolution paresseuse : un setup encore « EnFormation » dont la clôture
/// de barre est passée (avec marge) sans confirmation est « Dissipe ».
fn resoudre(liste: &mut Vec<SetupFormation>) {
    let maintenant = chrono::Utc::now().timestamp();
    for s in liste.iter_mut() {
        if s.statut == "EnFormation" && maintenant > s.cloture_barre + 30 {
            s.statut = "Dissipe".into();
        }
    }
}

/// GET /api/setups-formation?strategie=SMC — liste vivante, la plus récente
/// d'abord : en formation d'abord, puis confirmés/dissipés de la dernière
/// heure (traçabilité des annonces).
pub async fn get_setups(state: web::Data<AppState>) -> impl Responder {
    let _ = &state; // homogénéité des signatures ; l'état est en mémoire.
    let mut liste = etat().lock().unwrap_or_else(|e| e.into_inner());
    resoudre(&mut liste);
    // Purge : entrées de plus de 2 h (le panneau n'a pas vocation à archiver).
    let maintenant = chrono::Utc::now().timestamp();
    liste.retain(|s| maintenant - s.ts_annonce < 2 * 3600);
    let mut copie = liste.clone();
    copie.sort_by(|a, b| {
        let ordre = |s: &SetupFormation| match s.statut.as_str() {
            "EnFormation" => 0,
            "Confirme" => 1,
            _ => 2,
        };
        ordre(a)
            .cmp(&ordre(b))
            .then(b.ts_annonce.cmp(&a.ts_annonce))
    });
    HttpResponse::Ok().json(copie)
}

/// Construit un SetupFormation depuis un SignalBrut d'annonce.
pub fn depuis_annonce(strategie: &str, s: &engine::types::SignalBrut) -> SetupFormation {
    let tf = s.tf.as_str().to_string();
    SetupFormation {
        strategie: strategie.to_string(),
        asset: s.asset.as_str().to_string(),
        tf,
        direction: format!("{:?}", s.direction),
        force: s.score.clamp(1, 10),
        entree: s.prix_entree,
        sl: s.stop_loss,
        tps: s.take_profits.clone(),
        cle: s.cle.clone(),
        debut_barre: s.debut_barre,
        cloture_barre: s.debut_barre + tf_en_secondes(s.tf.as_str()),
        ts_annonce: chrono::Utc::now().timestamp(),
        statut: "EnFormation".into(),
    }
}
