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
use chrono::{Datelike, TimeZone, Timelike};
use serde::Serialize;
use std::collections::BTreeMap;
use std::sync::Arc;

pub use crate::analyses_smc::{BlocIa, TrancheScore};
use crate::analyses_smc::enrichissements_smc;

/// Une clôture normalisée — l'unité d'analyse (un trade qui a engagé du
/// capital, expirés compris : leur R réel a composé le capital).
#[derive(Clone)]
pub(crate) struct ClotureAnalyse {
    /// Id du signal — rapprochement score/avis LLM (enrichissements SMC).
    pub(crate) id: String,
    pub(crate) ferme_le: i64,
    pub(crate) asset: String,
    pub(crate) tf: String,
    pub(crate) verdict: String,
    /// R qui compose le capital (pondéré SMC, net straddle, réalisé base).
    pub(crate) r: f64,
    /// Profit/perte $ composé (variation du capital simulé à cette clôture).
    pub(crate) dollars: f64,
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

/// Case de la heatmap heure × jour (contribution $ des clôtures par
/// créneau horaire — parcours naturel du straddle, §14).
#[derive(Serialize)]
pub struct CaseHeatmap {
    /// 0=lundi … 6=dimanche (chrono).
    pub jour: u8,
    /// Heure locale 0-23.
    pub heure: u8,
    pub dollars: f64,
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
    /// R-distance moyen par clôture (r_total / nb_trades).
    pub r_moyen: f64,
    /// Part des clôtures perdantes ($ < 0) — 0-1 (complément inexact du WR :
    /// les sorties ~0 $ ne sont ni gagnantes ni perdantes).
    pub taux_perte: f64,
    /// Vrai quand SMC/straddle servent le REPLI base vécue parce que le
    /// re-jeu paramétrique n'est pas encore prêt (boot : ~35 s de calcul,
    /// ou relance après un changement de réglage). Sans cette marque, la
    /// carte affichait le R vécu puis SAUTAIT vers le R du re-jeu une
    /// minute plus tard, sans explication (bug rapporté 05/09).
    pub recalcul: bool,
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
    /// Heatmap heure × jour (cases non vides, tri jour puis heure).
    pub heatmap: Vec<CaseHeatmap>,
    /// Tranches de score SMC (bandes métier) — vide pour les autres.
    pub par_score: Vec<TrancheScore>,
    /// Bloc IA (rail conviction LLM + directions) — vide pour les autres.
    pub ia: BlocIa,
}

/// Récupère les clôtures + métadonnées capital d'une stratégie.
/// SMC/straddle : re-jeu paramétrique (cache chaud), repli base vécue.
async fn collecter(
    db: &Arc<db::Database>,
    id: &str,
) -> (Vec<ClotureAnalyse>, f64, f64, f64, &'static str) {
    // Source officielle (décision 15/09 soir) : la BASE VÉCUE pour toutes
    // les stratégies — les rejeus paramétriques ne sont plus servis ici
    // (ils restent calculés pour l'étude live↔replay, tâche 2.1).
    match crate::capital_simule::simuler(db, id).await {
        Ok(s) => {
            let clotures = s
                .points
                .iter()
                .map(|p| ClotureAnalyse {
                    id: p.id.clone(),
                    ferme_le: p.ferme_le,
                    asset: p.asset.clone(),
                    tf: p.tf.clone(),
                    verdict: normaliser_verdict(&p.verdict),
                    r: p.r_distance,
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
    let perdants = clotures.iter().filter(|c| c.dollars < 0.0).count();
    let r_moyen = if nb > 0 { r_total / nb as f64 } else { 0.0 };

    // Enrichissements SMC (tranches de score + bloc IA) — mêmes lignes que
    // les clôtures, jamais une fenêtre front.
    let (par_score, ia) = if id == "SMC" {
        enrichissements_smc(db, &clotures).await
    } else {
        (Vec::new(), BlocIa::vide())
    };

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
        r_moyen,
        taux_perte: if nb > 0 { perdants as f64 / nb as f64 } else { 0.0 },
        recalcul: false,
        taux_reussite: if nb > 0 { gagnants as f64 / nb as f64 } else { 0.0 },
        hier,
        journalier: periodes(&clotures, cle_jour),
        hebdomadaire: periodes(&clotures, cle_semaine),
        mensuel: periodes(&clotures, cle_mois),
        verdicts: categories(&clotures, |c| c.verdict.as_str()),
        assets: categories(&clotures, |c| c.asset.as_str()),
        tfs: categories(&clotures, |c| c.tf.as_str()),
        par_asset_tf: croise_asset_tf(&clotures),
        heatmap: heatmap_hj(&clotures),
        par_score,
        ia,
    };
    // §14 : snapshot quotidien persisté (INSERT OR REPLACE — le jour reflète
    // le dernier calcul ; l'avis IA éventuel est préservé par le UPDATE).
    let maintenant = chrono::Utc::now().timestamp();
    let jour = db::analyses_snapshots::cle_du_jour(maintenant);
    {
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
    }
    a
}

/// Agrégat heure × jour des clôtures ($, effectif) — heure LOCALE.
fn heatmap_hj(clotures: &[ClotureAnalyse]) -> Vec<CaseHeatmap> {
    let mut cases: BTreeMap<(u8, u8), (f64, usize)> = BTreeMap::new();
    for c in clotures {
        let Some(d) = chrono::Local.timestamp_opt(c.ferme_le, 0).single() else {
            continue;
        };
        let k = (d.weekday().num_days_from_monday() as u8, d.hour() as u8);
        let e = cases.entry(k).or_insert((0.0, 0));
        e.0 += c.dollars;
        e.1 += 1;
    }
    cases
        .into_iter()
        .map(|((jour, heure), (dollars, trades))| CaseHeatmap { jour, heure, dollars, trades })
        .collect()
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
            "recalcul": a.recalcul,
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Base mémoire + migrations + une stratégie SMC dotée d'un capital.
    async fn base_test() -> Arc<db::Database> {
        let db = db::Database::new(":memory:").await.expect("DB mémoire");
        db.run_migrations().await.expect("migrations OK");
        sqlx::query("UPDATE strategies SET etat = 'Officielle', capital = 1000.0, risque_pct = 1.0 WHERE id = 'SMC'")
            .execute(db.pool())
            .await
            .expect("stratégie SMC");
        Arc::new(db)
    }

    /// Insère une clôture SMC remplie (entrée 2000, SL 1990 → risque 10).
    async fn cloture(db: &Arc<db::Database>, id: &str, verdict: &str, score: f64, direction: &str, r_realise: f64) {
        sqlx::query(
            "INSERT INTO signaux (id, asset, timeframe, direction, score, prix_entree, stop_loss,
                                  take_profit, strategie, statut, verdict, r_realise, cree_le,
                                  heure_entree, ferme_le)
             VALUES (?, 'BTC', 'M5', ?, ?, 2000.0, 1990.0, '[2006, 2020, 2030]',
                     'SMC', 'Fermé', ?, ?, 1700000000, 1700000600, 1700001200)",
        )
        .bind(id)
        .bind(direction)
        .bind(score)
        .bind(verdict)
        .bind(r_realise)
        .execute(db.pool())
        .await
        .expect("insertion clôture");
    }

    /// HARMONISATION (15/09 soir) : l'analyse, la simulation capital et les
    /// catégories doivent compter EXACTEMENT les mêmes clôtures avec
    /// EXACTEMENT les mêmes R — c'est le verrou qui interdit à un écran de
    /// diverger d'un autre (bug « 257 vs 348 vs camemberts »).
    #[actix_web::test]
    async fn analyser_coherent_avec_simuler_et_categories() {
        let db = base_test().await;
        cloture(&db, "t1", "TP1+BE", 7.0, "Long", 0.3).await;   // r_distance 0.6
        cloture(&db, "t2", "TP2+BE", 10.0, "Short", 1.2).await; // r_distance 2.0
        cloture(&db, "t3", "SL", 15.0, "Long", -1.0).await;     // r_distance −1.0
        cloture(&db, "t4", "Expire", 8.0, "Short", 0.0).await;  // repli r_realise 0.0

        let a = analyser(&db, "SMC").await;
        let sim = crate::capital_simule::simuler(&db, "SMC").await.expect("simulation");

        // Même effectif partout : points capital, catégories, périodes, heatmap.
        assert_eq!(a.nb_trades, 4);
        assert_eq!(sim.points.len(), 4, "points capital ≠ nb_trades");
        assert_eq!(a.tfs.iter().map(|c| c.n).sum::<usize>(), 4, "Σ tfs.n");
        assert_eq!(a.assets.iter().map(|c| c.n).sum::<usize>(), 4, "Σ assets.n");
        assert_eq!(a.verdicts.iter().map(|c| c.n).sum::<usize>(), 4, "Σ verdicts.n");
        assert_eq!(a.journalier.iter().map(|p| p.trades).sum::<usize>(), 4, "Σ journalier");
        assert_eq!(a.heatmap.iter().map(|c| c.trades).sum::<usize>(), 4, "Σ heatmap");

        // Même R partout : r_total == Σ r_distance des points == Σ par verdict.
        let somme_points: f64 = sim.points.iter().map(|p| p.r_distance).sum();
        assert!((a.r_total - somme_points).abs() < 1e-9, "r_total {} ≠ Σ points {}", a.r_total, somme_points);
        assert!((a.r_total - 1.6).abs() < 1e-9, "ΣR distance attendu 1.6, obtenu {}", a.r_total);
        let somme_verdicts: f64 = a.verdicts.iter().map(|c| c.r).sum();
        assert!((a.r_total - somme_verdicts).abs() < 1e-9, "Σ verdicts.r ≠ r_total");
        assert!((a.r_moyen - 0.4).abs() < 1e-9, "r_moyen {}", a.r_moyen);

        // WR ($ > 0) + taux de perte ($ < 0) : complémentaires au pire des ~0 $.
        assert!(a.taux_reussite + a.taux_perte <= 1.0 + 1e-9);
        assert!(a.taux_reussite >= 0.0 && a.taux_reussite <= 1.0);

        // Tranches de score : mêmes lignes, aucun trade perdu.
        assert_eq!(a.par_score.iter().map(|t| t.n).sum::<usize>(), 4, "Σ par_score.n");
        assert_eq!(a.par_score[0].n, 2, "tranche 6–8 : scores 7 et 8");

        // Capital : départ persisté, actuel = départ + Σ profits.
        assert!((a.capital_depart - 1000.0).abs() < 1e-9);
        let somme_profits: f64 = sim.points.iter().map(|p| p.profit).sum();
        assert!((a.capital_actuel - (a.capital_depart + somme_profits)).abs() < 1e-6);
    }

    /// Le r_distance servi par /api/signaux est le même que celui des points
    /// capital (miroir signaux_lecture ↔ capital_simule).
    #[actix_web::test]
    async fn r_distance_signaux_egale_points_capital() {
        let db = base_test().await;
        cloture(&db, "u1", "TP2+BE", 10.0, "Long", 1.2).await;
        cloture(&db, "u2", "SL", 10.0, "Short", -1.0).await;

        let sim = crate::capital_simule::simuler(&db, "SMC").await.expect("simulation");
        let signaux = db.obtenir_signaux(100).await.expect("signaux");
        let par_id: std::collections::HashMap<String, f64> = signaux
            .iter()
            .filter_map(|s| {
                let r = s.get("r_distance")?.as_f64()?;
                let id = s.get("id")?.as_str()?.to_string();
                Some((id, r))
            })
            .collect();
        assert_eq!(par_id.len(), 2, "r_distance servi sur chaque clôture remplie");
        for p in &sim.points {
            let r_signal = par_id.get(&p.id).unwrap_or_else(|| panic!("signal {} absent", p.id));
            assert!((r_signal - p.r_distance).abs() < 1e-9, "signal {} : {} ≠ {}", p.id, r_signal, p.r_distance);
        }
    }
}
