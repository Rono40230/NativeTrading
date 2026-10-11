//! Synthèse IA du PORTFOLLE (owner 11/10) — POST /api/analyses/portefeuille/ia.
//!
//! La vue d'ensemble du rapport d'activité gagne une lecture d'analyste :
//! les métriques consolidées des 4 stratégies (clôtures, R distance, $,
//! WR, hiér) servies à Ollama qui dit QUI PORTE, QUI COINCE, OÙ
//! CONCENTRER L'ATTENTION. Même constitution que les autres analystes :
//! à la demande, cache du jour, l'IA propose jamais ne décide.

use std::sync::OnceLock;

use actix_web::{web, HttpResponse, Responder};

use crate::state::AppState;

#[derive(Debug, Clone, serde::Serialize)]
pub struct SynthesePortefeuille {
    pub etat: String,
    pub points_forts: Vec<String>,
    pub points_faibles: Vec<String>,
    pub recommandations: Vec<String>,
    pub confiance: u32,
    pub generee_le: i64,
}

#[derive(serde::Deserialize)]
struct ReponseLlm {
    etat: String,
    #[serde(default)]
    points_forts: Vec<String>,
    #[serde(default)]
    points_faibles: Vec<String>,
    #[serde(default)]
    recommandations: Vec<String>,
    #[serde(default)]
    confiance: f64,
}

static CACHE: OnceLock<tokio::sync::RwLock<Option<SynthesePortefeuille>>> = OnceLock::new();

fn cache() -> &'static tokio::sync::RwLock<Option<SynthesePortefeuille>> {
    CACHE.get_or_init(|| tokio::sync::RwLock::new(None))
}

/// POST /api/analyses/portefeuille/ia — génère (ou sert le cache du jour).
pub async fn post_synthese(state: web::Data<AppState>) -> impl Responder {
    {
        let g = cache().read().await;
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
    if contexte.len() < 50 {
        return HttpResponse::Ok().json(serde_json::json!({
            "en_cache": false,
            "analyse": SynthesePortefeuille {
                etat: "Aucune clôture analysable — l'analyste n'a rien à juger.".into(),
                points_forts: vec![], points_faibles: vec![], recommandations: vec![],
                confiance: 0, generee_le: chrono::Utc::now().timestamp(),
            }
        }));
    }

    let prompt = format!("{}\n\n{}", llm::prompt_effectif("analyse_portefeuille"), contexte);
    match llm::ollama::interroger(&prompt).await {
        Ok(texte) => {
            let synthese = parser(texte);
            *cache().write().await = Some(synthese.clone());
            HttpResponse::Ok().json(serde_json::json!({ "en_cache": false, "analyse": synthese }))
        }
        Err(e) => HttpResponse::ServiceUnavailable()
            .json(serde_json::json!({ "error": format!("Analyste indisponible : {e}") })),
    }
}

/// Contexte consolidé : une ligne par stratégie + totaux.
async fn contexte(state: &AppState) -> String {
    let mut l: Vec<String> = Vec::new();
    let mut total_clotures = 0usize;
    let mut total_r = 0f64;
    let mut total_dollars = 0f64;
    let mut total_hier = 0f64;
    for m in crate::registre_strategies::MANIFESTES {
        let a = crate::analyses::analyser(&state.db, m.id).await;
        if a.nb_trades == 0 {
            continue;
        }
        let dollars = a.capital_actuel - a.capital_depart;
        let hier = a.hier.as_ref().map(|h| h.dollars).unwrap_or(0.0);
        l.push(format!(
            "STRATÉGIE {} : {} clôtures · Σ R DISTANCE {:+.1} · {} → {:.0} $ ({:+.0} $) · WR {:.0} % · hier {:+.0} $",
            m.nom, a.nb_trades, a.r_distance_total, a.capital_depart, a.capital_actuel, dollars, a.taux_reussite * 100.0, hier
        ));
        total_clotures += a.nb_trades;
        total_r += a.r_distance_total;
        total_dollars += dollars;
        total_hier += hier;
    }
    if l.is_empty() {
        return String::new();
    }
    l.push(format!(
        "TOTALES : {} clôtures · Σ R DISTANCE {:+.1} · {:+.0} $ cumulés · hier {:+.0} $",
        total_clotures, total_r, total_dollars, total_hier
    ));
    l.push("Règle des 30 : une stratégie sous 30 clôtures reste non jugeable — dis-le si c'est le cas.".into());
    l.push("R DISTANCE juge entrées et TP ; $ est le résultat réel composé — deux voix distinctes.".into());
    l.join("\n")
}

/// Extraction robuste : premier { dernier } — repli texte brut dans etat.
fn parser(texte: String) -> SynthesePortefeuille {
    let n = texte.trim().to_string();
    let (d, f) = (n.find('{'), n.rfind('}'));
    if let (Some(d), Some(f)) = (d, f) {
        if f > d {
            if let Ok(v) = serde_json::from_str::<ReponseLlm>(&n[d..=f]) {
                return SynthesePortefeuille {
                    etat: v.etat,
                    points_forts: v.points_forts,
                    points_faibles: v.points_faibles,
                    recommandations: v.recommandations,
                    confiance: crate::analyses_ia::confiance_normalisee(v.confiance),
                    generee_le: chrono::Utc::now().timestamp(),
                };
            }
        }
    }
    SynthesePortefeuille {
        etat: n.chars().take(1500).collect(),
        points_forts: vec![], points_faibles: vec![], recommandations: vec![],
        confiance: 0, generee_le: chrono::Utc::now().timestamp(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_json_environne() {
        let a = parser(
            "Voici :\n{\"etat\":\"solide\",\"points_forts\":[\"SMC +89,8 R\"],\"points_faibles\":[\"KDJ 3 clôtures\"],\"recommandations\":[\"surveiller KDJ\"],\"confiance\":72}".into(),
        );
        assert_eq!(a.etat, "solide");
        assert_eq!(a.points_forts, vec!["SMC +89,8 R"]);
        assert_eq!(a.confiance, 72);
    }

    #[test]
    fn repli_texte_brut() {
        let a = parser("Le portefeuille manque de recul.".into());
        assert!(a.etat.contains("recul"));
        assert!(a.points_forts.is_empty());
    }
}
