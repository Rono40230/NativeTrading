//! Centre d'analyse par stratégie — « Rapport d'activité ».
//!
//! GET /api/analyses/{strategie} : agrégats daily/weekly/monthly, verdicts,
//! assets, TF — en $ réels (le même $ qui compose le capital des cartes) et
//! en R (pondéré SMC / net straddle / réalisé base — la convention de chaque
//! moteur, jamais un R de référence). GET /api/analyses : le résumé global.
//!
//! Sources : re-jeu paramétrique SMC/straddle quand le cache est chaud
//! (sinon repli sur la base vécue et lancement du calcul), base pour rockets.
//! Les jours/semaines/mois suivent l'heure LOCALE du serveur (= celle de
//! l'utilisateur) — mêmes frontières que l'histogramme du dashboard.

use crate::state::AppState;
use actix_web::{web, HttpResponse};
use chrono::{Datelike, TimeZone};
use serde::Serialize;
use std::collections::BTreeMap;
use std::sync::Arc;

/// Une clôture normalisée — l'unité d'analyse (un trade qui a engagé du
/// capital, expirés compris : leur R réel a composé le capital).
#[derive(Clone)]
struct ClotureAnalyse {
    ferme_le: i64,
    asset: String,
    tf: String,
    verdict: String,
    /// R qui compose le capital (pondéré SMC, net straddle, réalisé base).
    r: f64,
    /// Profit/perte $ composé (variation du capital simulé à cette clôture).
    dollars: f64,
}

#[derive(Serialize)]
pub struct PeriodeAnalyse {
    /// Clé de tri : 2026-09-03 / 2026-S36 / 2026-09.
    pub cle: String,
    /// Libellé court : 03/09 / S36 / sept. 26.
    pub label: String,
    pub dollars: f64,
    pub r: f64,
    pub trades: usize,
    pub gagnants: usize,
}

#[derive(Serialize)]
pub struct CategorieAnalyse {
    pub label: String,
    pub n: usize,
    pub dollars: f64,
    pub r: f64,
    /// Part des clôtures gagnantes ($ > 0) — 0-1.
    pub wr: f64,
}

#[derive(Serialize)]
pub struct ResumeJour {
    pub date: String,
    pub dollars: f64,
    pub r: f64,
    pub trades: usize,
}

/// Croisé asset × TF (bloc « Timeframes » du rapport) : contribution de
/// chaque TF d'un asset, l'asset totalisé en tête.
#[derive(Serialize)]
pub struct ParAssetTf {
    pub asset: String,
    pub dollars: f64,
    pub n: usize,
    /// Contribution de chaque TF de l'asset ($, R, n, WR) — triée par $.
    pub tfs: Vec<CategorieAnalyse>,
}

#[derive(Serialize)]
pub struct AnalyseStrategie {
    pub strategie: String,
    pub etat: String,
    /// "rejeu" = re-jeu paramétrique, "base" = clôtures vécues.
    pub source: &'static str,
    pub nb_trades: usize,
    pub fenetre_debut: i64,
    pub fenetre_fin: i64,
    pub capital_depart: f64,
    pub capital_actuel: f64,
    pub fraction_risque: f64,
    pub r_total: f64,
    /// Part des clôtures gagnantes ($ > 0) — 0-1.
    pub taux_reussite: f64,
    /// Journée d'hier (données de la veille — vide si aucun trade).
    pub hier: Option<ResumeJour>,
    pub journalier: Vec<PeriodeAnalyse>,
    pub hebdomadaire: Vec<PeriodeAnalyse>,
    pub mensuel: Vec<PeriodeAnalyse>,
    pub verdicts: Vec<CategorieAnalyse>,
    pub assets: Vec<CategorieAnalyse>,
    pub tfs: Vec<CategorieAnalyse>,
    /// Croisé asset × TF, trié par contribution $ décroissante.
    pub par_asset_tf: Vec<ParAssetTf>,
}

/// Récupère les clôtures + métadonnées capital d'une stratégie.
/// SMC/straddle : re-jeu paramétrique (cache chaud), repli base vécue.
async fn collecter(
    db: &Arc<db::Database>,
    id: &str,
) -> (Vec<ClotureAnalyse>, f64, f64, f64, &'static str) {
    if id == "SMC" {
        if let Some(r) = crate::smc_rejeu::lire_cache().await {
            let mut precedent = r.capital_depart;
            let clotures = r
                .clotures
                .iter()
                .map(|c| {
                    let dollars = c.capital_apres - precedent;
                    precedent = c.capital_apres;
                    ClotureAnalyse {
                        ferme_le: c.ferme_le,
                        asset: c.asset.clone(),
                        tf: c.tf.clone(),
                        verdict: normaliser_verdict(&c.verdict),
                        r: c.r_pondere,
                        dollars,
                    }
                })
                .collect();
            return (
                clotures,
                r.capital_depart,
                r.capital_actuel,
                r.fraction_risque,
                "rejeu",
            );
        }
        crate::smc_rejeu::lancer_si_necessaire(db.clone()).await;
    }
    if id == "straddle" {
        if let Some(r) = crate::straddle_rejeu::lire_cache().await {
            let mut precedent = r.capital_depart;
            let fraction = db
                .lire_strategie(id)
                .await
                .ok()
                .flatten()
                .map(|reg| reg.risque_pct / 100.0)
                .unwrap_or(0.01);
            let clotures = r
                .clotures
                .iter()
                .map(|c| {
                    let dollars = c.capital_apres - precedent;
                    precedent = c.capital_apres;
                    ClotureAnalyse {
                        ferme_le: c.ferme_le,
                        asset: c.asset.clone(),
                        tf: "M1".into(),
                        verdict: normaliser_verdict(&c.verdict),
                        r: c.r_net,
                        dollars,
                    }
                })
                .collect();
            return (clotures, r.capital_depart, r.capital_actuel, fraction, "rejeu");
        }
        crate::straddle_rejeu::lancer_si_necessaire(db.clone()).await;
    }
    // Repli/rails général : simulation capital sur la base vécue.
    match crate::capital_simule::simuler(db, id).await {
        Ok(s) => {
            let clotures = s
                .points
                .iter()
                .map(|p| ClotureAnalyse {
                    ferme_le: p.ferme_le,
                    asset: p.asset.clone(),
                    tf: p.tf.clone(),
                    verdict: normaliser_verdict(&p.verdict),
                    r: p.r,
                    dollars: p.profit,
                })
                .collect();
            (
                clotures,
                s.capital_depart,
                s.capital_actuel,
                s.fraction_risque,
                "base",
            )
        }
        Err(_) => (Vec::new(), 0.0, 0.0, 0.0, "base"),
    }
}

/// Verdict canonique d'affichage : TP1+BE / TP2+BE / TP3 / TS / SL / BE /
/// TimeStop → Expire — minuscules base et majuscules rejeu réunifiées.
fn normaliser_verdict(brut: &str) -> String {
    let v = brut.trim();
    if v.eq_ignore_ascii_case("timestop") || v.eq_ignore_ascii_case("expire") {
        "Expire".into()
    } else {
        match v.to_lowercase().as_str() {
            "sl" | "sl+be" => "SL".into(),
            "be" => "BE".into(),
            "tp1" | "tp1+be" => "TP1+BE".into(),
            "tp2" | "tp2+be" => "TP2+BE".into(),
            "tp3" => "TP3".into(),
            "ts" => "TS".into(),
            "" => "—".into(),
            autre => autre.to_uppercase(),
        }
    }
}

/// (clé, libellé) du jour local d'une clôture — 2026-09-03 / « 03/09 ».
fn cle_jour(ts: i64) -> (String, String) {
    let d = chrono::Local
        .timestamp_opt(ts, 0)
        .single()
        .unwrap_or_else(chrono::Local::now);
    (
        format!("{:04}-{:02}-{:02}", d.year(), d.month(), d.day()),
        format!("{:02}/{:02}", d.day(), d.month()),
    )
}

/// (clé, libellé) de la semaine ISO — 2026-S36 / « S36 ».
fn cle_semaine(ts: i64) -> (String, String) {
    let d = chrono::Local
        .timestamp_opt(ts, 0)
        .single()
        .unwrap_or_else(chrono::Local::now);
    let iso = d.iso_week();
    (
        format!("{:04}-S{:02}", iso.year(), iso.week()),
        format!("S{:02}", iso.week()),
    )
}

const MOIS_COURTS: [&str; 12] = [
    "janv.", "févr.", "mars", "avr.", "mai", "juin", "juil.", "août", "sept.", "oct.", "nov.",
    "déc.",
];

/// (clé, libellé) du mois — 2026-09 / « sept. 26 ».
fn cle_mois(ts: i64) -> (String, String) {
    let d = chrono::Local
        .timestamp_opt(ts, 0)
        .single()
        .unwrap_or_else(chrono::Local::now);
    (
        format!("{:04}-{:02}", d.year(), d.month()),
        format!("{} {:02}", MOIS_COURTS[(d.month() - 1) as usize], d.year() % 100),
    )
}

/// Agrège les clôtures par période (jour/semaine/mois), triées par clé.
fn periodes(clotures: &[ClotureAnalyse], cle: fn(i64) -> (String, String)) -> Vec<PeriodeAnalyse> {
    let mut par: BTreeMap<String, (String, f64, f64, usize, usize)> = BTreeMap::new();
    for c in clotures {
        let (k, label) = cle(c.ferme_le);
        let e = par.entry(k).or_insert((label, 0.0, 0.0, 0, 0));
        e.1 += c.dollars;
        e.2 += c.r;
        e.3 += 1;
        if c.dollars > 0.0 {
            e.4 += 1;
        }
    }
    par.into_iter()
        .map(|(cle, (label, dollars, r, trades, gagnants))| PeriodeAnalyse {
            cle,
            label,
            dollars,
            r,
            trades,
            gagnants,
        })
        .collect()
}

/// Agrège par catégorie (verdict/asset/TF) : n, $, R, WR — trié par $ décroissant.
fn categories(clotures: &[ClotureAnalyse], cle: fn(&ClotureAnalyse) -> &str) -> Vec<CategorieAnalyse> {
    let mut par: BTreeMap<String, (usize, f64, f64, usize)> = BTreeMap::new();
    for c in clotures {
        let e = par.entry(cle(c).to_string()).or_insert((0, 0.0, 0.0, 0));
        e.0 += 1;
        e.1 += c.dollars;
        e.2 += c.r;
        if c.dollars > 0.0 {
            e.3 += 1;
        }
    }
    let mut out: Vec<CategorieAnalyse> = par
        .into_iter()
        .map(|(label, (n, dollars, r, gagnants))| CategorieAnalyse {
            label,
            n,
            dollars,
            r,
            wr: if n > 0 { gagnants as f64 / n as f64 } else { 0.0 },
        })
        .collect();
    out.sort_by(|a, b| b.dollars.total_cmp(&a.dollars));
    out
}

/// Construit l'analyse complète d'une stratégie.
pub async fn analyser(db: &Arc<db::Database>, id: &str) -> AnalyseStrategie {
    let (clotures, capital_depart, capital_actuel, fraction, source) = collecter(db, id).await;
    let etat = db
        .lire_strategies()
        .await
        .ok()
        .and_then(|regs| regs.into_iter().find(|r| r.id == id).map(|r| r.etat))
        .unwrap_or_else(|| "Construction".into());

    let nb = clotures.len();
    let r_total = clotures.iter().map(|c| c.r).sum();
    let gagnants = clotures.iter().filter(|c| c.dollars > 0.0).count();

    // Hier (jour local de la veille) — les données de la veille.
    let cle_hier = {
        let hier = chrono::Local::now() - chrono::Duration::hours(24);
        format!("{:04}-{:02}-{:02}", hier.year(), hier.month(), hier.day())
    };
    let hier = periodes(&clotures, cle_jour)
        .into_iter()
        .find(|p| p.cle == cle_hier)
        .map(|p| ResumeJour {
            date: p.label,
            dollars: p.dollars,
            r: p.r,
            trades: p.trades,
        });

    let (debut, fin) = if nb > 0 {
        let mut min = i64::MAX;
        let mut max = i64::MIN;
        for c in &clotures {
            min = min.min(c.ferme_le);
            max = max.max(c.ferme_le);
        }
        (min, max)
    } else {
        (0, 0)
    };

    let a = AnalyseStrategie {
        strategie: id.to_string(),
        etat,
        source,
        nb_trades: nb,
        fenetre_debut: debut,
        fenetre_fin: fin,
        capital_depart,
        capital_actuel,
        fraction_risque: fraction,
        r_total,
        taux_reussite: if nb > 0 { gagnants as f64 / nb as f64 } else { 0.0 },
        hier,
        journalier: periodes(&clotures, cle_jour),
        hebdomadaire: periodes(&clotures, cle_semaine),
        mensuel: periodes(&clotures, cle_mois),
        verdicts: categories(&clotures, |c| c.verdict.as_str()),
        assets: categories(&clotures, |c| c.asset.as_str()),
        tfs: categories(&clotures, |c| c.tf.as_str()),
        par_asset_tf: croise_asset_tf(&clotures),
    };
    // §14 : snapshot quotidien persisté (INSERT OR REPLACE — le jour reflète
    // le dernier calcul ; l'avis IA éventuel est préservé par le UPDATE).
    let maintenant = chrono::Utc::now().timestamp();
    let jour = db::analyses_snapshots::cle_du_jour(maintenant);
    let _ = db
        .enregistrer_analyse_snapshot(
            id,
            &jour,
            a.capital_depart,
            a.capital_actuel,
            a.r_total,
            a.taux_reussite,
            a.nb_trades as i64,
            a.hier.as_ref().map(|h| h.dollars),
            maintenant,
        )
        .await;
    a
}

/// Croisé asset × TF : chaque asset avec la contribution de ses TF,
/// assets triés par $ décroissant.
fn croise_asset_tf(clotures: &[ClotureAnalyse]) -> Vec<ParAssetTf> {
    let mut par_asset: BTreeMap<&str, Vec<&ClotureAnalyse>> = BTreeMap::new();
    for c in clotures {
        par_asset.entry(c.asset.as_str()).or_default().push(c);
    }
    let mut out: Vec<ParAssetTf> = par_asset
        .into_iter()
        .map(|(asset, clot)| {
            let possedes: Vec<ClotureAnalyse> =
                clot.into_iter().map(|c| c.clone()).collect();
            ParAssetTf {
                asset: asset.to_string(),
                dollars: possedes.iter().map(|c| c.dollars).sum(),
                n: possedes.len(),
                tfs: categories(&possedes, |c| c.tf.as_str()),
            }
        })
        .collect();
    out.sort_by(|a, b| b.dollars.total_cmp(&a.dollars));
    out
}

/// GET /api/analyses — vue d'ensemble : une ligne par stratégie active.
pub async fn get_analyses(state: web::Data<AppState>) -> impl actix_web::Responder {
    let ids: Vec<&str> = crate::registre_strategies::MANIFESTES
        .iter()
        .map(|m| m.id)
        .collect();
    let mut liste = Vec::with_capacity(ids.len());
    for id in ids {
        let a = analyser(&state.db, id).await;
        liste.push(serde_json::json!({
            "strategie": a.strategie,
            "etat": a.etat,
            "source": a.source,
            "nb_trades": a.nb_trades,
            "capital_depart": a.capital_depart,
            "capital_actuel": a.capital_actuel,
            "r_total": a.r_total,
            "taux_reussite": a.taux_reussite,
            "hier": a.hier,
        }));
    }
    HttpResponse::Ok().json(serde_json::json!({ "strategies": liste }))
}

/// GET /api/analyses/{strategie}/historique — snapshots quotidiens persistés
/// (§14 : évolution des métriques et des avis IA jour après jour).
pub async fn get_historique_analyses(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> impl actix_web::Responder {
    let id = path.into_inner();
    if !crate::registre_strategies::MANIFESTES.iter().any(|m| m.id == id) {
        return HttpResponse::NotFound()
            .json(serde_json::json!({ "error": "Stratégie inconnue" }));
    }
    match state.db.lister_analyses_snapshots(&id, 60).await {
        Ok(snapshots) => HttpResponse::Ok().json(serde_json::json!({ "snapshots": snapshots })),
        Err(e) => HttpResponse::InternalServerError()
            .json(serde_json::json!({ "error": e.to_string() })),
    }
}

/// GET /api/analyses/{strategie} — analyse complète d'une stratégie.
pub async fn get_analyse(state: web::Data<AppState>, path: web::Path<String>) -> impl actix_web::Responder {
    let id = path.into_inner();
    if !crate::registre_strategies::MANIFESTES.iter().any(|m| m.id == id) {
        return HttpResponse::NotFound()
            .json(serde_json::json!({ "error": "Stratégie inconnue" }));
    }
    HttpResponse::Ok().json(analyser(&state.db, &id).await)
}
