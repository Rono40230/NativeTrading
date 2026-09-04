//! Analyse IA du rapport d'activité — POST /api/analyses/{strategie}/ia.
//!
//! À la demande (bouton « Générer » de la page Analyses), jamais en tâche de
//! fond. L'analyste local (Ollama) reçoit le contexte CHIFFRÉ compact de la
//! stratégie ($ réels composés + R de la convention du moteur) et répond en
//! JSON structuré. Cache pour la journée (clé = stratégie + date locale) :
//! regénérable en redémarrant le backend. Constitution respectée : l'IA lit,
//! juge, propose — elle ne touche jamais aux réglages.

use crate::analyses::{analyser, AnalyseStrategie, CategorieAnalyse, PeriodeAnalyse};
use crate::state::AppState;
use actix_web::{web, HttpResponse};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize)]
pub struct AnalyseIa {
    /// Résumé de l'état de la stratégie (2-3 phrases).
    pub etat: String,
    pub points_forts: Vec<String>,
    pub points_faibles: Vec<String>,
    /// Pistes d'étude/correction — le propriétaire décide seul.
    pub corrections: Vec<String>,
    /// Confiance de l'analyste dans son analyse, 0-100.
    pub confiance: u32,
    pub nb_trades: usize,
    pub generee_le: i64,
}

#[derive(Deserialize)]
struct AnalyseIaLlm {
    etat: String,
    #[serde(default)]
    points_forts: Vec<String>,
    #[serde(default)]
    points_faibles: Vec<String>,
    #[serde(default)]
    corrections: Vec<String>,
    /// Le modèle répond indifféremment en entier 0-100 (« 75 ») ou en
    /// décimal 0-1 (« 0.75 ») selon l'humeur — on normalise.
    #[serde(default)]
    confiance: f64,
}

/// Normalise la confiance : décimal ≤ 1 → ×100, bornée 0-100, entière.
fn confiance_normalisee(v: f64) -> u32 {
    let n = if v <= 1.0 { v * 100.0 } else { v };
    n.round().clamp(0.0, 100.0) as u32
}

static CACHE: OnceLock<RwLock<HashMap<String, Arc<AnalyseIa>>>> = OnceLock::new();

fn cache() -> &'static RwLock<HashMap<String, Arc<AnalyseIa>>> {
    CACHE.get_or_init(|| RwLock::new(HashMap::new()))
}

/// Clé de cache du jour : {strategie}-{date locale} — invalidation quotidienne
/// naturelle (les données de la veille alimentent l'analyse du matin).
fn cle_du_jour(id: &str) -> String {
    format!("{}-{}", id, chrono::Local::now().format("%Y-%m-%d"))
}

/// POST /api/analyses/{strategie}/ia — génère (ou sert le cache du jour).
pub async fn post_analyse_ia(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> impl actix_web::Responder {
    let id = path.into_inner();
    if !crate::registre_strategies::MANIFESTES.iter().any(|m| m.id == id) {
        return HttpResponse::NotFound()
            .json(serde_json::json!({ "error": "Stratégie inconnue" }));
    }
    let cle = cle_du_jour(&id);
    if let Some(c) = cache().read().await.get(&cle) {
        return HttpResponse::Ok().json(serde_json::json!({ "en_cache": true, "analyse": *c.clone() }));
    }

    let a = analyser(&state.db, &id).await;
    if a.nb_trades == 0 {
        return HttpResponse::Ok().json(serde_json::json!({
            "en_cache": false,
            "analyse": AnalyseIa {
                etat: "Aucune clôture analysable — l'analyste n'a rien à juger.".into(),
                points_forts: vec![], points_faibles: vec![], corrections: vec![],
                confiance: 0, nb_trades: 0, generee_le: chrono::Utc::now().timestamp(),
            }
        }));
    }

    let prompt = format!("{}\n\n{}", consigne(), contexte(&a));
    match llm::ollama::interroger(&prompt).await {
        Ok(texte) => {
            let analyse = parser(texte, a.nb_trades);
            persister_avis(&state.db, &id, &analyse).await;
            let arc = Arc::new(analyse);
            cache().write().await.insert(cle, arc.clone());
            HttpResponse::Ok().json(serde_json::json!({ "en_cache": false, "analyse": *arc }))
        }
        Err(e) => HttpResponse::ServiceUnavailable()
            .json(serde_json::json!({ "error": format!("Analyste indisponible : {e}") })),
    }
}

/// §14 : rattache l'avis du jour à son snapshot quotidien (l'historique des
/// avis survit aux redémarrages — le cache mémoire, lui, ne survit pas).
async fn persister_avis(db: &std::sync::Arc<db::Database>, id: &str, avis: &AnalyseIa) {
    if let Ok(json) = serde_json::to_string(avis) {
        let jour = db::analyses_snapshots::cle_du_jour(chrono::Utc::now().timestamp());
        let _ = db
            .enregistrer_avis_snapshot(id, &jour, &json, avis.generee_le)
            .await;
    }
}

/// Consigne de l'analyste — prompt ÉDITABLE (Configuration & Métriques IA ›
/// Prompts, id « analyse_rapport ») : llm::prompt_effectif sert la version
/// persistée si elle existe, sinon le défaut ci-dessous (llm/prompts.rs).
/// La partie DYNAMIQUE (effectif vs règle des 30 trades) vit dans le
/// contexte, pas ici — le prompt reste 100 % statique et éditable.
fn consigne() -> String {
    llm::prompt_effectif("analyse_rapport")
}

/// Contexte chiffré compact servi à l'analyste (pas de trades bruts).
fn contexte(a: &AnalyseStrategie) -> String {
    let mut l = Vec::new();
    l.push(format!(
        "STRATÉGIE {} — état {} — source : {}",
        a.strategie,
        a.etat,
        if a.source == "rejeu" { "re-jeu paramétrique" } else { "clôtures vécues" }
    ));
    // Effectif vs règle des 30 trades — la partie dynamique de la consigne
    // vit ici (le prompt éditable reste statique).
    l.push(if a.nb_trades < 30 {
        format!(
            "Effectif : {} clôtures — SOUS la règle des 30 trades, aucune conclusion \
             n'est statistiquement significative : reste descriptif et prudent.",
            a.nb_trades
        )
    } else {
        format!(
            "Effectif : {} clôtures — suffisant (≥ 30), conclusions chiffrées permises.",
            a.nb_trades
        )
    });
    l.push(format!(
        "{} clôtures · risque {:.1} %/trade · capital {:.0} $ → {:.0} $ ({:+.0} $) · ΣR {:+.1} · réussite {:.0} %",
        a.nb_trades,
        a.fraction_risque * 100.0,
        a.capital_depart,
        a.capital_actuel,
        a.capital_actuel - a.capital_depart,
        a.r_total,
        a.taux_reussite * 100.0
    ));
    if let Some(h) = &a.hier {
        l.push(format!(
            "Hier ({}): {:+.1} $ · {:+.1}R · {} clôture(s)",
            h.date, h.dollars, h.r, h.trades
        ));
    } else {
        l.push("Hier : aucune clôture".into());
    }
    l.push(format!("Verdicts : {}", lignes_categories(&a.verdicts)));
    l.push(format!("Assets : {}", lignes_categories(&a.assets)));
    l.push(format!("Timeframes : {}", lignes_categories(&a.tfs)));
    // Journalier : 14 derniers jours seulement (le contexte reste compact).
    l.push(format!("Journalier : {}", lignes_periodes(derniers(&a.journalier, 14))));
    l.push(format!("Hebdomadaire : {}", lignes_periodes(&a.hebdomadaire)));
    l.push(format!("Mensuel : {}", lignes_periodes(&a.mensuel)));
    l.join("\n")
}

fn derniers(v: &[PeriodeAnalyse], n: usize) -> &[PeriodeAnalyse] {
    if v.len() > n { &v[v.len() - n..] } else { v }
}

fn lignes_categories(cats: &[CategorieAnalyse]) -> String {
    cats.iter()
        .map(|c| format!("{} ×{} ({:+.0} $, {:+.1}R, {:.0} % ok)", c.label, c.n, c.dollars, c.r, c.wr * 100.0))
        .collect::<Vec<_>>()
        .join(" ; ")
}

fn lignes_periodes(per: &[PeriodeAnalyse]) -> String {
    per.iter()
        .map(|p| format!("{} {:+.0} $ ({}, {} ok)", p.label, p.dollars, p.trades, p.gagnants))
        .collect::<Vec<_>>()
        .join(" ; ")
}

/// Extraction robuste : premier {...} dernier } (le LLM ajoute parfois du
/// texte autour) ; repli = texte brut dans « etat » (avis non structuré vaut
/// mieux qu'une erreur).
fn parser(texte: String, nb_trades: usize) -> AnalyseIa {
    let nettoyee = texte.trim().to_string();
    let debut = nettoyee.find('{');
    let fin = nettoyee.rfind('}');
    if let (Some(d), Some(f)) = (debut, fin) {
        if f > d {
            if let Ok(vue) = serde_json::from_str::<AnalyseIaLlm>(&nettoyee[d..=f]) {
                return AnalyseIa {
                    etat: vue.etat,
                    points_forts: vue.points_forts,
                    points_faibles: vue.points_faibles,
                    corrections: vue.corrections,
                    confiance: confiance_normalisee(vue.confiance),
                    nb_trades,
                    generee_le: chrono::Utc::now().timestamp(),
                };
            }
        }
    }
    AnalyseIa {
        etat: nettoyee.chars().take(2000).collect(),
        points_forts: vec![],
        points_faibles: vec![],
        corrections: vec![],
        confiance: 0,
        nb_trades,
        generee_le: chrono::Utc::now().timestamp(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_json_environne_de_texte() {
        let a = parser(
            "Voici mon analyse :\n{\"etat\":\"solide\",\"points_forts\":[\"WR élevé\"],\"points_faibles\":[],\"corrections\":[\"étudier TP2\"],\"confiance\":72}\nCordialement".into(),
            42,
        );
        assert_eq!(a.etat, "solide");
        assert_eq!(a.points_forts, vec!["WR élevé"]);
        assert_eq!(a.confiance, 72);
        assert_eq!(a.nb_trades, 42);
    }

    #[test]
    fn repli_texte_brut_quand_json_absent() {
        let a = parser("Le trailing semble trop serré sur BTC.".into(), 5);
        assert!(a.etat.contains("trailing"));
        assert!(a.points_forts.is_empty());
        assert_eq!(a.confiance, 0);
    }

    #[test]
    fn confiance_bornee_a_100() {
        let a = parser("{\"etat\":\"x\",\"confiance\":250}".into(), 1);
        assert_eq!(a.confiance, 100);
    }

    #[test]
    fn confiance_decimale_normalisee() {
        // Retour réel observé (04/09) : « confiance: 0.6 » cassait le parse
        // u32 et déversait le JSON brut dans « etat ». Désormais : 0.6 → 60.
        let a = parser(
            "{\"etat\":\"solide\",\"points_forts\":[\"WR élevé\"],\"points_faibles\":[],\"corrections\":[],\"confiance\":0.6}".into(),
            94,
        );
        assert_eq!(a.etat, "solide", "le JSON doit se parser, pas s'afficher brut");
        assert_eq!(a.confiance, 60);
    }
}
