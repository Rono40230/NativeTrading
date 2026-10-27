//! STRUCT P1 (owner 27/10) — synthèse « structure du marché » par TF.
//!
//! AGGREGATEUR pur : rejeu du moteur v12 EXISTANT sur M15→W1 (même moteur,
//! mêmes sorties que /api/smc/v12/analyse — ZÉRO nouvelle détection). Sert
//! la table fusionnée Tendance & Structure : par TF, l'état structurel
//! (HH/HL vs LH/LL), le dernier événement (BOS/MSS/CHoCH/sweep), le
//! dernier pivot, la position premium/discount et les liquidités les plus
//! proches (PDH/PWH au-dessus, PDL/PWL en dessous) — plus le verdict
//! d'alignement MTF (biais HTF × contexte H4 × déclencheur LTF, lecture
//! ICT). Cache mémoire 60 s par asset : 6 rejeus de ~600 barres restent
//! légers, mais inutiles à chaque poll front.

use std::collections::HashMap;
use std::sync::{Arc, OnceLock, Mutex};

use actix_web::{web, HttpResponse, Responder};
use smc::v12::{BarInput, SmcV12Engine};

use crate::state::AppState;
use crate::utils::parse_asset;

/// TF synthétisés — M1/M5 exclus par design (bruit structurel, moteur v12
/// armé M15+), W1 inclus (structure hebdomadaire = vrai biais ICT, la
/// tendance KAS l'ignore par règle owner 15/10 « pas chez Kasper »).
const TFS: [(&str, common::Timeframe); 6] = [
    ("M15", common::Timeframe::M15),
    ("M30", common::Timeframe::M30),
    ("H1", common::Timeframe::H1),
    ("H4", common::Timeframe::H4),
    ("D1", common::Timeframe::D1),
    ("W1", common::Timeframe::W1),
];

/// Fenêtre de rejeu par TF (600 barres suffisent à la structure ; W1 en a
/// souvent moins — accepté, l'état W1 reste lisible dès ~30 barres).
const FENETRE: i64 = 600;

// ── Sorties ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize, PartialEq)]
pub struct EvenementStructure {
    pub genre: String, // "BOS" | "MSS" | "CHoCH" | "Sweep"
    pub dir: String,   // "bull" | "bear"
    pub level: f64,
    pub ts: i64,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq)]
pub struct PivotStructure {
    pub ptype: String, // "HH" | "HL" | "LH" | "LL"
    pub price: f64,
    pub ts: i64,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Default)]
pub struct LigneStructure {
    pub tf: String,
    /// "haussiere" | "baissiere" | "neutre" — la même que sert l'analyse
    /// v12 (séquence de pivots du moteur).
    pub etat: String,
    pub dernier_evenement: Option<EvenementStructure>,
    pub dernier_pivot: Option<PivotStructure>,
    pub in_premium: bool,
    pub in_discount: bool,
    /// Liquidité la plus proche AU-DESSUS (buy-side : PDH/PWH actifs).
    pub bsl: Option<f64>,
    /// Liquidité la plus proche EN DESSOUS (sell-side : PDL/PWL actifs).
    pub ssl: Option<f64>,
    /// Horodatage de la dernière bougie rejouée — ancre la fraîcheur du
    /// déclencheur (5 barres du TF d'origine). Interne, jamais sérialisé.
    #[serde(skip)]
    pub ts_derniere: i64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ReponseStructureMtf {
    pub asset: String,
    pub lignes: Vec<LigneStructure>,
    pub verdict: VerdictStructure,
    pub genere_le: i64,
}

// ── Verdict (pur, testé) — échelle actionnable owner 27/10 (6 états,
// fraîcheur 5 barres). Module dédié : garde-fou 600 lignes/fichier. ──────
#[path = "structure_verdict.rs"]
mod verdict_ict;

pub use verdict_ict::{verdict, VerdictStructure};
// ── Rejeu par TF ─────────────────────────────────────────────────────────────

/// Rejoue le moteur v12 sur une fenêtre et n'extrait que le structurel.
/// Même moteur et mêmes champs que l'analyse officielle — en pratique un
/// extrait de sa boucle : état par barre, événements, dernier pivot, puis
/// états finaux premium/discount et liquidités.
async fn ligne_structure(
    db: &Arc<db::Database>,
    asset: &common::Asset,
    tf_nom: &str,
    tf: common::Timeframe,
) -> LigneStructure {
    let bougies = db
        .obtenir_bougies(asset, &tf, FENETRE)
        .await
        .unwrap_or_default();
    let mut ligne = LigneStructure {
        tf: tf_nom.to_string(),
        ..Default::default()
    };
    if bougies.len() < 30 {
        ligne.etat = "neutre".into();
        return ligne; // fenêtre trop courte (W1 jeune) : ligne muette
    }
    let mut engine = SmcV12Engine::new(asset.as_str(), tf_nom)
        .avec_mode_be_force(smc::v12::lifecycle::ModeBeForce::Supprime);
    let mut etat = "neutre".to_string();
    let mut evenements: Vec<EvenementStructure> = Vec::new();
    let mut dernier_pivot: Option<PivotStructure> = None;
    let mut dernier_ts = 0i64;
    let mut dernier_close = 0f64;

    for b in &bougies {
        let bar = BarInput {
            timestamp: b.timestamp.timestamp(),
            open: b.open,
            high: b.high,
            low: b.low,
            close: b.close,
            volume: b.volume,
        };
        dernier_ts = bar.timestamp;
        dernier_close = bar.close;
        let out = engine.update(&bar);
        if out.structure.tendance_haussiere {
            etat = "haussiere".into();
        } else if out.structure.tendance_baissiere {
            etat = "baissiere".into();
        }
        if (out.bos.bullish || out.bos.bearish) && out.bos.level.is_some() {
            evenements.push(EvenementStructure {
                genre: "BOS".into(),
                dir: if out.bos.bullish { "bull".into() } else { "bear".into() },
                level: out.bos.level.unwrap_or(0.0),
                ts: bar.timestamp,
            });
        }
        if (out.mss.mss_haussier || out.mss.mss_baissier) && out.mss.mss_level.is_some() {
            let mss = out.mss.mss_haussier;
            evenements.push(EvenementStructure {
                genre: "MSS".into(),
                dir: if mss { "bull".into() } else { "bear".into() },
                level: out.mss.mss_level.unwrap_or(0.0),
                ts: bar.timestamp,
            });
        }
        if (out.mss.choch_haussier || out.mss.choch_baissier) && out.mss.choch_level.is_some() {
            evenements.push(EvenementStructure {
                genre: "CHoCH".into(),
                dir: if out.mss.choch_haussier { "bull".into() } else { "bear".into() },
                level: out.mss.choch_level.unwrap_or(0.0),
                ts: bar.timestamp,
            });
        }
        if out.sweep.sweep_haussier && out.sweep.sweep_h_level.is_some() {
            evenements.push(EvenementStructure {
                genre: "Sweep".into(),
                dir: "bull".into(), // pairing moteur : sweep_haussier = bull
                level: out.sweep.sweep_h_level.unwrap_or(0.0),
                ts: bar.timestamp,
            });
        }
        if out.sweep.sweep_baissier && out.sweep.sweep_b_level.is_some() {
            evenements.push(EvenementStructure {
                genre: "Sweep".into(),
                dir: "bull".into(),
                level: out.sweep.sweep_b_level.unwrap_or(0.0),
                ts: bar.timestamp,
            });
        }
        if let (Some(price), Some(bi)) = (out.pivot.pivot_high_price, out.pivot.pivot_bar_index) {
            if out.structure.is_hh || out.structure.is_lh {
                if let Some(ts) = bougies.get(bi).map(|c| c.timestamp.timestamp()) {
                    dernier_pivot = Some(PivotStructure {
                        ptype: if out.structure.is_hh { "HH" } else { "LH" }.into(),
                        price,
                        ts,
                    });
                }
            }
        }
        if let (Some(price), Some(bi)) = (out.pivot.pivot_low_price, out.pivot.pivot_bar_index) {
            if out.structure.is_hl || out.structure.is_ll {
                if let Some(ts) = bougies.get(bi).map(|c| c.timestamp.timestamp()) {
                    dernier_pivot = Some(PivotStructure {
                        ptype: if out.structure.is_hl { "HL" } else { "LL" }.into(),
                        price,
                        ts,
                    });
                }
            }
        }
    }

    let pd = engine.premium_discount.last_event();
    let liq = engine.liquidites.last_event();
    ligne.etat = etat;
    ligne.dernier_evenement = evenements.last().cloned();
    ligne.dernier_pivot = dernier_pivot;
    ligne.in_premium = pd.in_premium;
    ligne.in_discount = pd.in_discount;
    // Liquidités actives les plus proches du dernier close (PDH/PWH dessus,
    // PDL/PWL dessous — prix ≤ 0 = niveau absent côté moteur).
    ligne.bsl = [liq.pdh, liq.pwh]
        .into_iter()
        .flatten()
        .filter(|p| *p > dernier_close)
        .fold(None::<f64>, |acc, p| Some(acc.map_or(p, |a: f64| a.min(p))));
    ligne.ssl = [liq.pdl, liq.pwl]
        .into_iter()
        .flatten()
        .filter(|p| *p < dernier_close)
        .fold(None::<f64>, |acc, p| Some(acc.map_or(p, |a: f64| a.max(p))));
    ligne.ts_derniere = dernier_ts;
    ligne
}

// ── Cache + endpoint ─────────────────────────────────────────────────────────

fn cache() -> &'static Mutex<HashMap<String, (i64, ReponseStructureMtf)>> {
    static CACHE: OnceLock<Mutex<HashMap<String, (i64, ReponseStructureMtf)>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// GET /api/smc/structure-mtf?asset=XAUUSD — table structure M15→W1 +
/// verdict d'alignement (cache 60 s par asset).
pub async fn get_structure_mtf(state: web::Data<AppState>, query: web::Query<std::collections::HashMap<String, String>>) -> impl Responder {
    let brut = query.get("asset").cloned().unwrap_or_default();
    let Some(asset) = parse_asset(&brut) else {
        return HttpResponse::BadRequest().json(serde_json::json!({ "error": "Asset inconnu" }));
    };
    let cle = asset.as_str().to_string();
    let maintenant = chrono::Utc::now().timestamp();
    if let Some((ts, rep)) = cache().lock().ok().and_then(|c| c.get(&cle).cloned()) {
        if maintenant - ts < 60 {
            return HttpResponse::Ok().json(rep);
        }
    }

    let mut lignes = Vec::with_capacity(TFS.len());
    for (tf_nom, tf) in TFS {
        lignes.push(ligne_structure(&state.db, &asset, tf_nom, tf).await);
    }
    let verdict = verdict(&lignes);
    let rep = ReponseStructureMtf {
        asset: cle.clone(),
        lignes,
        verdict,
        genere_le: maintenant,
    };
    if let Some(mut c) = cache().lock().ok() {
        c.insert(cle, (maintenant, rep.clone()));
    }
    HttpResponse::Ok().json(rep)
}
