//! Analyse IA de la matrice des créneaux straddle (owner 10/10).
//!
//! Sous le tableau (asset × événement), l'analyste local (Ollama) lit les
//! compteurs RÉELS des cases et formule, en quelques phrases : quels
//! événements travailler, quels assets, et quels CROISEMENTS (asset ×
//! événement) portent leΣR — règle des 30 PAR CASE, verdicts de la boucle
//! à l'appui. L'IA propose, ne décide jamais (constitution 24/08) :
//! l'armement reste au clic propriétaire. Cache du jour.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use actix_web::{web, HttpResponse, Responder};
use sqlx::Row;

use crate::state::AppState;

#[derive(Debug, Clone, serde::Serialize)]
pub struct AnalyseCreneaux {
    pub etat: String,
    /// Les événements à travailler en priorité (avec leur justification).
    pub meilleurs_evenements: Vec<String>,
    /// Les croisements asset × événement les plus porteurs.
    pub meilleurs_croisements: Vec<String>,
    pub confiance: u32,
    pub generee_le: i64,
}

#[derive(serde::Deserialize)]
struct ReponseLlm {
    etat: String,
    #[serde(default)]
    meilleurs_evenements: Vec<String>,
    #[serde(default)]
    meilleurs_croisements: Vec<String>,
    #[serde(default)]
    confiance: f64,
}

fn cache() -> &'static Mutex<Option<AnalyseCreneaux>> {
    static C: OnceLock<Mutex<Option<AnalyseCreneaux>>> = OnceLock::new();
    C.get_or_init(|| Mutex::new(None))
}

/// POST /api/evenements/ia — génère (ou sert le cache du jour).
pub async fn post_analyse(state: web::Data<AppState>) -> impl Responder {
    {
        let g = cache().lock().unwrap_or_else(|p| p.into_inner());
        if let Some(a) = g.as_ref() {
            let jour = chrono::Local::now().format("%Y-%m-%d").to_string();
            if chrono::DateTime::from_timestamp(a.generee_le, 0)
                .map(|t| t.format("%Y-%m-%d").to_string())
                .as_deref()
                == Some(jour.as_str())
            {
                return HttpResponse::Ok().json(serde_json::json!({ "en_cache": true, "analyse": a }));
            }
        }
    }

    let contexte = contexte(&state).await;
    if contexte.lines().count() < 3 {
        return HttpResponse::Ok().json(serde_json::json!({
            "en_cache": false,
            "analyse": AnalyseCreneaux {
                etat: "Aucune case comptée pour l'instant — l'analyste n'a rien à juger.".into(),
                meilleurs_evenements: vec![], meilleurs_croisements: vec![],
                confiance: 0, generee_le: chrono::Utc::now().timestamp(),
            }
        }));
    }

    let prompt = format!("{}\n\n{}", llm::prompt_effectif("analyse_creneaux"), contexte);
    match llm::ollama::interroger(&prompt).await {
        Ok(texte) => {
            let analyse = parser(texte);
            *cache().lock().unwrap_or_else(|p| p.into_inner()) = Some(analyse.clone());
            HttpResponse::Ok().json(serde_json::json!({ "en_cache": false, "analyse": analyse }))
        }
        Err(e) => HttpResponse::ServiceUnavailable()
            .json(serde_json::json!({ "error": format!("Analyste indisponible : {e}") })),
    }
}

/// Contexte chiffré compact : par événement, par asset, puis les cellules
/// non nulles (le croisement) + seuils de la boucle.
async fn contexte(state: &AppState) -> String {
    let rows = sqlx::query(
        "SELECT asset, evenement, occurrences, somme_r, verdict_test
         FROM creneaux_evenements WHERE occurrences > 0",
    )
    .fetch_all(state.db.pool())
    .await
    .unwrap_or_default();

    let mut par_event: HashMap<String, (i64, f64)> = HashMap::new();
    let mut par_asset: HashMap<String, (i64, f64)> = HashMap::new();
    let mut cellules: Vec<String> = Vec::new();
    for r in &rows {
        let asset: String = r.get("asset");
        let event: String = r.get("evenement");
        let n: i64 = r.get("occurrences");
        let s: f64 = r.get("somme_r");
        let verdict: Option<String> = r.try_get("verdict_test").ok().flatten();
        *par_event.entry(event.clone()).or_insert((0, 0.0)) = {
            let e = par_event.get(&event).copied().unwrap_or((0, 0.0));
            (e.0 + n, e.1 + s)
        };
        *par_asset.entry(asset.clone()).or_insert((0, 0.0)) = {
            let a = par_asset.get(&asset).copied().unwrap_or((0, 0.0));
            (a.0 + n, a.1 + s)
        };
        cellules.push(format!(
            "{} × {} : {} tirage(s), Σ {:+.1}R{}",
            asset,
            event,
            n,
            s,
            verdict.map(|v| format!(" — verdict {v}")).unwrap_or_default()
        ));
    }
    fn nom(id: &str) -> &str {
        crate::evenements::catalogue()
            .iter()
            .find(|e| e.ident == id)
            .map(|e| e.nom)
            .unwrap_or(id)
    }
    let mut l = Vec::new();
    let mut evs: Vec<_> = par_event.iter().collect();
    evs.sort_by(|a, b| b.1 .0.cmp(&a.1 .0));
    for (id, (n, s)) in &evs {
        l.push(format!("ÉVÉNEMENT {} : {} tirage(s), Σ {:+.1}R", nom(id), n, s));
    }
    let mut assets: Vec<_> = par_asset.iter().collect();
    assets.sort_by(|a, b| b.1 .0.cmp(&a.1 .0));
    for (id, (n, s)) in &assets {
        l.push(format!("ASSET {} : {} tirage(s), Σ {:+.1}R", id, n, s));
    }
    l.push(format!("CROISEMENTS (cases non nulles) : {}", cellules.join(" ; ")));
    l.push("Règle des 30 : une case sous 30 tirages reste non jugeable — signale-le.".into());
    l.join("\n")
}

/// Extraction robuste (premier { dernier }) — même discipline que les
/// autres analystes ; repli texte brut dans « etat ».
fn parser(texte: String) -> AnalyseCreneaux {
    let n = texte.trim().to_string();
    let (d, f) = (n.find('{'), n.rfind('}'));
    if let (Some(d), Some(f)) = (d, f) {
        if f > d {
            if let Ok(v) = serde_json::from_str::<ReponseLlm>(&n[d..=f]) {
                return AnalyseCreneaux {
                    etat: v.etat,
                    meilleurs_evenements: v.meilleurs_evenements,
                    meilleurs_croisements: v.meilleurs_croisements,
                    confiance: crate::analyses_ia::confiance_normalisee(v.confiance),
                    generee_le: chrono::Utc::now().timestamp(),
                };
            }
        }
    }
    AnalyseCreneaux {
        etat: n.chars().take(1500).collect(),
        meilleurs_evenements: vec![],
        meilleurs_croisements: vec![],
        confiance: 0,
        generee_le: chrono::Utc::now().timestamp(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_json_environne() {
        let a = parser(
            "Voici :\n{\"etat\":\"solide\",\"meilleurs_evenements\":[\"Ouverture Francfort\"],\"meilleurs_croisements\":[\"XAUUSD × LBMA\"],\"confiance\":72}".into(),
        );
        assert_eq!(a.etat, "solide");
        assert_eq!(a.meilleurs_evenements, vec!["Ouverture Francfort"]);
        assert_eq!(a.confiance, 72);
    }

    #[test]
    fn repli_texte_brut() {
        let a = parser("La matrice manque de tirages.".into());
        assert!(a.etat.contains("tirages"));
        assert!(a.meilleurs_croisements.is_empty());
    }
}
