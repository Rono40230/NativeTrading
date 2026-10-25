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
/// Partagée avec l'analyste PAR ASSET (phase 3).
pub(crate) fn confiance_normalisee(v: f64) -> u32 {
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

    let prompt = match definition_strategie(&id) {
        Some(def) => format!("{}\n\n{}\n\n{}", def, consigne(), contexte(&state.db, &a).await),
        None => format!("{}\n\n{}", consigne(), contexte(&state.db, &a).await),
    };
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

/// Paramètres moteur SMC EN VIGUEUR, lus en direct de la config et des
/// surcharges — injectés dans le contexte de CHAQUE analyse SMC (revue
/// prompts 13/10 : la définition reste une doctrine pure, jamais chiffrée ;
/// tout changement de réglage — y compris hot-reload — se reflète ici).
pub(crate) async fn parametres_moteur_smc(db: &db::Database) -> String {
    let (r_actif, r_k, r_f1, r_f2, r_f3, r_tp1, r_tp3) = tokio::join!(
        db.lire_config("smc_tp3_trailing"),
        db.lire_config("smc_tp3_trailing_r"),
        db.lire_config("smc_frac_tp1"),
        db.lire_config("smc_frac_tp2"),
        db.lire_config("smc_frac_tp3"),
        db.lire_config("smc_tp1_mult"),
        db.lire_config("smc_tp3_mode"),
    );
    let v = |r: common::Result<Option<String>>| r.ok().flatten().unwrap_or_default();
    let trailing_actif = v(r_actif) == "1";
    let k = v(r_k);
    let f1 = v(r_f1);
    let f2 = v(r_f2);
    let f3 = v(r_f3);
    let tp1 = v(r_tp1);
    let tp3_mode = v(r_tp3);
    let nb_surcharges: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM smc_reglages_asset")
        .fetch_one(db.pool())
        .await
        .unwrap_or(0);
    format!(
        "PARAMÈTRES MOTEUR ACTUELS (lus en direct) : trailing après TP2 {} (k = {}) · sorties : ventes partielles {}/{}/{} · TP1 ×{}R · TP3 {} · surcharges par asset actives : {}.",
        if trailing_actif { "ACTIF" } else { "inactif" },
        if k.is_empty() { "?" } else { &k },
        pct(&f1), pct(&f2), pct(&f3),
        if tp1.is_empty() { "?" } else { &tp1 },
        if tp3_mode.is_empty() { "?" } else { &tp3_mode },
        nb_surcharges
    )
}

fn pct(fract: &str) -> String {
    match fract.parse::<f64>() {
        Ok(v) => format!("{} %", (v * 100.0).round() as i64),
        Err(_) => fract.to_string(),
    }
}


/// Définition (doctrine) d'une stratégie — injectée EN TÊTE de ses analyses
/// (revue prompts 13/10, écart 29 : la promesse de la page Prompts devient
/// réelle). KDJ doté de sa définition le 22/10 (owner — doctrine moteur
/// kdj_halftrend : KDJ+HalfTrend+SMA100/EMA200, exécution à l'open).
pub(crate) fn definition_strategie(id: &str) -> Option<String> {
    let cle = match id {
        "SMC" => "smc_definition",
        "straddle" => "straddle_definition",
        "rockets" => "rockets_definition",
        "kdj_halftrend" => "kdj_definition",
        _ => return None,
    };
    Some(llm::prompt_effectif(cle))
}

/// Paramètres moteur STRADDLE en vigueur, lus en direct (même discipline que
/// [`parametres_moteur_smc`]) — injectés avec les analyse straddle.
pub(crate) async fn parametres_moteur_straddle(db: &db::Database) -> String {
    let surcharges: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM straddle_reglages_asset")
        .fetch_one(db.pool())
        .await
        .unwrap_or(0);
    let perimetre = db.lire_config("perimetre_straddle").await.ok().flatten().unwrap_or_default();
    let nb_perim = serde_json::from_str::<Vec<String>>(&perimetre).map(|v| v.len()).unwrap_or(0);
    let armees: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM creneaux_evenements WHERE arme = 1")
        .fetch_one(db.pool())
        .await
        .unwrap_or(0);
    format!(
        "PARAMÈTRES MOTEUR ACTUELS (lis en direct) : placement à T−10 s (réglable par asset) · SL = 0,5×ATR H1 · trailing 1,0R après TP2 · time-stop 60 min · périmètre {nb_perim} assets · {armees} cases armées · {surcharges} surcharge(s) par asset."
    )
}

async fn contexte(db: &db::Database, a: &AnalyseStrategie) -> String {
    let mut l = Vec::new();
    l.push(parametres_moteur_smc(db).await);
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
    // Deux voix distinctes (décision 23/09) : R DISTANCE (la stratégie —
    // entrées et TP) et $ (le résultat réel, réglages compris). L'écart
    // entre les deux = la conversion (mécanique de sortie). Le R encaissé
    // brut n'est plus servi à l'analyste — il racontait la moitié de
    // l'histoire en la faisant passer pour la whole.
    let perf_pct = if a.capital_depart > 0.0 {
        (a.capital_actuel / a.capital_depart - 1.0) * 100.0
    } else {
        0.0
    };
    l.push(format!(
        "{} clôtures · risque {:.1} %/trade · Σ R DISTANCE {:+.1} (le niveau le plus lointain atteint — juge entrées et TP) · capital {:.0} $ → {:.0} $ ({:+.0} $, {:+.1} %) · réussite {:.0} %",
        a.nb_trades,
        a.fraction_risque * 100.0,
        a.r_distance_total,
        a.capital_depart,
        a.capital_actuel,
        a.capital_actuel - a.capital_depart,
        perf_pct,
        a.taux_reussite * 100.0
    ));
    l.push(format!(
        "CONVERSION : {:+.1} R de distance parcourus → {:+.1} % de capital. L'écart vient de la mécanique de sortie (ventes partielles, break-even, trailing) — ce n'est PAS un défaut de sélection des trades.",
        a.r_distance_total, perf_pct
    ));
    if let Some(h) = &a.hier {
        l.push(format!(
            "Hier ({}): {:+.1} $ · Σ R distance {:+.1} · {} clôture(s)",
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
    // §11-3 (06/09) : la boucle ML nourrit l'analyste — le TOP des features
    // par permutation OOS (ce qui distingue les trades gagnants), lu à chaud
    // depuis ml_feature_importance. Absent si jamais entraîné.
    if let Ok(top) = db::ml_feature_importance::lire_top_importances(db.pool(), &cle_strategie(a), 8).await {
        if !top.is_empty() && top.iter().any(|f| f.importance > 0.0) {
            let parts: Vec<String> = top
                .iter()
                .map(|f| format!("{} ({:.1} %)", f.feature_nom, f.importance * 100.0))
                .collect();
            l.push(format!(
                "ML — features qui distinguent les gagnants (importance par permutation) : {}",
                parts.join(", ")
            ));
        }
    }
    l.join("\n")
}

/// Clé ML de la stratégie analysée (« SMC » → « smc », etc.).
fn cle_strategie(a: &AnalyseStrategie) -> String {
    a.strategie.to_lowercase()
}

fn derniers(v: &[PeriodeAnalyse], n: usize) -> &[PeriodeAnalyse] {
    if v.len() > n { &v[v.len() - n..] } else { v }
}

fn lignes_categories(cats: &[CategorieAnalyse]) -> String {
    cats.iter()
        .map(|c| format!("{} ×{} ({:+.0} $, Σ R distance {:+.1}, {:.0} % ok)", c.label, c.n, c.dollars, c.r, c.wr * 100.0))
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
