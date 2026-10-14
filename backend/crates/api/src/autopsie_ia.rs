//! §Autopsie de la semaine (13/10, owner — inspiration KTA #10/#26) : les
//! clôtures des 7 derniers jours, toutes stratégies, décrites factuellement
//! — ce qui a distingué les gagnants des perdants. DESCRIPTIF, jamais
//! prédictif (leçon conviction). Cache du jour, prompt `autopsie_semaine`.

use std::sync::{Mutex, OnceLock};

use actix_web::{web, HttpResponse};
use serde::Serialize;
use sqlx::Row;

use crate::state::AppState;

#[derive(Debug, Clone, Serialize, Default)]
pub struct Autopsie {
    pub etat: String,
    pub gagnants: Vec<String>,
    pub perdants: Vec<String>,
    pub remarques: Vec<String>,
    pub confiance: u32,
    pub generee_le: i64,
    /// Compteurs factuels de la semaine, servis par l'app (hors LLM).
    pub compteurs: String,
}

fn cache() -> &'static Mutex<Option<Autopsie>> {
    static C: OnceLock<Mutex<Option<Autopsie>>> = OnceLock::new();
    C.get_or_init(|| Mutex::new(None))
}

fn jour_courant(ts: i64) -> String {
    chrono::DateTime::from_timestamp(ts, 0)
        .map(|t| t.with_timezone(&chrono::Local).format("%Y-%m-%d").to_string())
        .unwrap_or_default()
}

/// POST /api/analyses/autopsie — cache du jour ou régénération.
pub async fn post_autopsie(state: web::Data<AppState>) -> HttpResponse {
    {
        let g = cache().lock().unwrap_or_else(|p| p.into_inner());
        if let Some(a) = g.as_ref() {
            if jour_courant(a.generee_le) == jour_courant(chrono::Utc::now().timestamp()) {
                return HttpResponse::Ok().json(serde_json::json!({ "en_cache": true, "autopsie": a }));
            }
        }
    }

    let compteurs = collecter(&state).await;
    let mut a = Autopsie { compteurs: compteurs.clone(), generee_le: chrono::Utc::now().timestamp(), ..Default::default() };
    if compteurs.lines().any(|l| l.contains('·')) {
        let prompt = format!("{}\n\n{}", llm::prompt_effectif("autopsie_semaine"), compteurs);
        if let Ok(texte) = llm::ollama::interroger(&prompt).await {
            a = fusionner(texte, compteurs);
        } else {
            a.etat = "Analyste indisponible — compteurs factuels ci-dessous.".into();
        }
    } else {
        a.etat = "Aucune clôture ces 7 derniers jours.".into();
    }
    a.generee_le = chrono::Utc::now().timestamp();
    *cache().lock().unwrap_or_else(|p| p.into_inner()) = Some(a.clone());
    HttpResponse::Ok().json(serde_json::json!({ "en_cache": false, "autopsie": a }))
}

/// Compteurs factuels des 7 derniers jours : par stratégie, verdicts,
/// ventilations TF/asset gagnants vs perdants — tout lu dans le vécu.
async fn collecter(state: &web::Data<AppState>) -> String {
    let db = &state.db;
    let debut = chrono::Utc::now().timestamp() - 7 * 86_400;
    let Ok(rows) = sqlx::query(
        "SELECT strategie, asset, timeframe, direction,
                CASE WHEN r_realise > 0 THEN 1 ELSE 0 END AS gagnant,
                r_realise
         FROM signaux
         WHERE statut != 'Actif' AND ferme_le IS NOT NULL AND ferme_le >= ?
           AND r_realise IS NOT NULL",
    )
    .bind(debut)
    .fetch_all(db.pool())
    .await
    else {
        return String::new();
    };
    if rows.is_empty() {
        return String::new();
    }

    use std::collections::BTreeMap;
    let mut par_strategie: BTreeMap<String, (i64, i64, f64)> = BTreeMap::new(); // (nb, gagnants, r_total)
    let mut par_tf: BTreeMap<String, (i64, i64)> = BTreeMap::new();
    let mut par_asset: BTreeMap<String, (i64, i64)> = BTreeMap::new();
    for r in &rows {
        let s: String = r.get("strategie");
        let g: i64 = r.get("gagnant");
        let rr: f64 = r.get("r_realise");
        let e = par_strategie.entry(s.clone()).or_insert((0, 0, 0.0));
        e.0 += 1;
        e.1 += g;
        e.2 += rr;
        let tf: String = r.get("timeframe");
        let e = par_tf.entry(format!("{} {}", s, tf)).or_insert((0, 0));
        e.0 += 1;
        e.1 += g;
        let a: String = r.get("asset");
        let e = par_asset.entry(a).or_insert((0, 0));
        e.0 += 1;
        e.1 += g;
    }

    let mut l = vec![format!(
        "CLÔTURES DES 7 DERNIERS JOURS : {} au total",
        rows.len()
    )];
    for (s, (n, g, r)) in &par_strategie {
        l.push(format!("STRATÉGIE {s} : {n} clôtures · {g} gagnantes · Σ R {r:+.1}"));
    }
    l.push("VENTILATION TF (stratégie TF : nb/gagnants) :".into());
    for (k, (n, g)) in &par_tf {
        l.push(format!("  {k} : {g}/{n}"));
    }
    l.push("VENTILATION ASSET (asset : nb/gagnants) :".into());
    for (k, (n, g)) in &par_asset {
        l.push(format!("  {k} : {g}/{n}"));
    }
    l.push("Règle des 30 : une semaine = faible effectif — reste descriptif.".into());
    l.join("\n")
}

/// Extraction robuste { … } ; les compteurs factuels restent servis par l'app.
fn fusionner(texte: String, compteurs: String) -> Autopsie {
    let n = texte.trim().to_string();
    let (d, f) = (n.find('{'), n.rfind('}'));
    if let (Some(d), Some(f)) = (d, f) {
        if f > d {
            #[derive(serde::Deserialize)]
            struct Rep {
                #[serde(default)] etat: String,
                #[serde(default)] gagnants: Vec<String>,
                #[serde(default)] perdants: Vec<String>,
                #[serde(default)] remarques: Vec<String>,
                #[serde(default)] confiance: f64,
            }
            if let Ok(v) = serde_json::from_str::<Rep>(&n[d..=f]) {
                return Autopsie {
                    etat: v.etat,
                    gagnants: v.gagnants,
                    perdants: v.perdants,
                    remarques: v.remarques,
                    confiance: crate::analyses_ia::confiance_normalisee(v.confiance),
                    generee_le: chrono::Utc::now().timestamp(),
                    compteurs,
                };
            }
        }
    }
    Autopsie { etat: n.chars().take(800).collect(), compteurs, ..Default::default() }
}
