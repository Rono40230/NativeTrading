//! Re-jeu paramétrique SMC — l'historique re-dérivé du TP1 réglé.
//!
//! Quand le propriétaire change TP1 (Paramètres › SMC), les métriques qui en
//! découlent (verdicts, R de référence, WR, capital composé) doivent se
//! recalculer automatiquement. Le moteur SMC étant déterministe, on rejoue
//! l'historique des bougies par le chemin du moteur (harnais des études A-B,
//! clôtures seules) avec le TP1 réglé, puis on dérive les métriques du journal
//! d'événements. Résultat mis en cache en mémoire : recalcul uniquement quand
//! le réglage change (ou à froid). Les verdicts RÉELS en base ne sont jamais
//! réécrits — ils restent l'étalon (l'étape 3 servira ce cache aux endpoints).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::Instant;

use actix_web::{web, HttpResponse};
use serde::Serialize;
use tokio::sync::RwLock;

use crate::state::AppState;

/// Rafraîchissement du cache au repos : sans changement de réglage, les
/// nouvelles clôtures live doivent quand même rejoindre les métriques.
const REFRESH_SEC: i64 = 1_800;

/// Chauffe du moteur avant la période mesurée (indicateurs, structure,
/// liquidités) — les clôtures de la chauffe ne comptent pas dans les métriques.
const JOURS_CHAUFFE: i64 = 45;

#[derive(Debug, Clone, Serialize)]
pub struct ClotureRejeu {
    pub asset: String,
    pub tf: String,
    pub ferme_le: i64,
    pub verdict: String,
    /// R réalisé (sortie) du trade re-joué.
    pub r: f64,
    /// R de référence (palier max atteint) — métrique primaire du dashboard.
    pub r_ref: f64,
    /// Capital simulé après cette clôture (composé, risque du registre SMC).
    pub capital_apres: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct RejeuSmc {
    /// TP1 (× R) utilisé pour ce re-jeu.
    pub tp1: f64,
    pub calcule_le: i64,
    pub nb_couples: usize,
    pub nb_bougies: usize,
    pub duree_ms: u128,
    /// Trades remplis clôturés (base des stats), triés chronologiquement.
    pub clotures: Vec<ClotureRejeu>,
    pub total: usize,
    pub gagnants: usize,
    pub taux_reussite: f64,
    /// Σ R de référence.
    pub r_total: f64,
    /// Σ R réalisé.
    pub r_total_realise: f64,
    pub capital_depart: f64,
    pub fraction_risque: f64,
    pub capital_actuel: f64,
}

static CACHE: OnceLock<RwLock<Option<Arc<RejeuSmc>>>> = OnceLock::new();
static EN_COURS: AtomicBool = AtomicBool::new(false);

fn cache() -> &'static RwLock<Option<Arc<RejeuSmc>>> {
    CACHE.get_or_init(|| RwLock::new(None))
}

/// TP1 réglé (config smc_tp1_mult, défaut 0.6, borné comme à l'armement).
async fn lire_tp1(db: &db::Database) -> f64 {
    db.lire_config("smc_tp1_mult")
        .await
        .ok()
        .flatten()
        .and_then(|v| v.trim().parse::<f64>().ok())
        .map(|v| v.clamp(0.2, 1.5))
        .unwrap_or(0.6)
}

/// Lance le re-jeu si nécessaire : pas de cache, ou TP1 réglé ≠ TP1 du cache.
/// Idempotent (flag EN_COURS) — le déclencheur config et le lecteur GET
/// peuvent appeler en concurrence sans doubler le calcul.
pub async fn lancer_si_necessaire(pool: Arc<db::Database>) {
    let tp1 = match lire_tp1(&pool).await {
        t => t,
    };
    if let Some(c) = cache().read().await.clone() {
        let frais = chrono::Utc::now().timestamp() - c.calcule_le < REFRESH_SEC;
        if (c.tp1 - tp1).abs() < 1e-9 && frais {
            return;
        }
    }
    if EN_COURS.swap(true, Ordering::SeqCst) {
        return; // un calcul est déjà en vol
    }
    tokio::spawn(async move {
        let debut = Instant::now();
        match calculer(&pool, tp1).await {
            Ok(rejeu) => {
                tracing::info!(
                    "Rejeu SMC: TP1={:.2}R — {} couple(s), {} clôture(s), WR {:.0} %, R réf {:+.1}, capital {:.2} → {:.2} ({:?})",
                    tp1, rejeu.nb_couples, rejeu.total, rejeu.taux_reussite * 100.0,
                    rejeu.r_total, rejeu.capital_depart, rejeu.capital_actuel, debut.elapsed()
                );
                *cache().write().await = Some(Arc::new(rejeu));
            }
            Err(e) => {
                tracing::warn!("Rejeu SMC: échec — {e}");
            }
        }
        EN_COURS.store(false, Ordering::SeqCst);
    });
}

/// GET /api/smc/rejeu — métriques SMC re-dérivées du TP1 réglé.
/// Déclenche le calcul à la demande s'il n'est pas déjà en cache.
pub async fn get_rejeu(state: web::Data<AppState>) -> impl actix_web::Responder {
    lancer_si_necessaire(state.db.clone()).await;
    if let Some(c) = cache().read().await.clone() {
        return HttpResponse::Ok().json(serde_json::json!({ "en_cours": false, "rejeu": *c }));
    }
    HttpResponse::Ok().json(serde_json::json!({ "en_cours": true, "rejeu": null }))
}

/// Snapshot du cache (None = pas encore calculé).
pub async fn lire_cache() -> Option<Arc<RejeuSmc>> {
    cache().read().await.clone()
}

/// Un recalcul est-il en vol (badge ⏳ côté dashboard).
pub fn recalcul_en_cours() -> bool {
    EN_COURS.load(Ordering::SeqCst)
}

async fn calculer(pool: &Arc<db::Database>, tp1: f64) -> anyhow::Result<RejeuSmc> {
    use engine::TypeEvenementTrade as T;

    let assets = crate::runtime_tick::assets_runtime(pool).await;
    let timeframes = data::worker_config::lire_timeframes(pool).await;

    // Fenêtre = période réelle de l'historique SMC en base (première
    // émission → maintenant), élargie de la chauffe. Les métriques portent
    // uniquement sur les clôtures de la période — comparables au dashboard.
    let maintenant = chrono::Utc::now().timestamp();
    let t0 = pool
        .debut_historique_epoch("SMC")
        .await
        .unwrap_or(maintenant - 7 * 86_400);
    let jours = ((maintenant - t0) / 86_400 + 7 + JOURS_CHAUFFE).max(JOURS_CHAUFFE) as u32;

    let reg = pool.lire_strategie("SMC").await?.unwrap_or_default();
    let fraction = reg.risque_pct / 100.0;
    let mut clotures: Vec<ClotureRejeu> = Vec::new();
    let mut nb_couples = 0usize;
    let mut nb_bougies = 0usize;

    for asset in &assets {
        let amorce = crate::runtime_tick::charger_amorce_mtf_runtime(pool, asset).await;
        for tf in &timeframes {
            let bougies = pool
                .obtenir_bougies_depuis_jours(asset, tf, jours)
                .await
                .unwrap_or_default();
            if bougies.is_empty() {
                continue;
            }
            nb_couples += 1;
            nb_bougies += bougies.len();
            let res = engine_v12::replay::rejouer_bougies_tp1(
                asset.clone(), *tf, &bougies, amorce.clone(), tp1,
            );
            // Signaux confirmés indexés par clé → entry/SL/TPs pour le R de
            // référence (palier), comme la performance servie du dashboard.
            let signaux: std::collections::HashMap<&str, &engine::SignalBrut> = res
                .signaux
                .iter()
                .filter(|s| !s.annonce)
                .map(|s| (s.cle.as_str(), s))
                .collect();
            for e in &res.evenements {
                if !matches!(e.evenement, T::Cloture) {
                    continue;
                }
                // Période mesurée seulement — la chauffe ne compte pas.
                // NB : en replay, `emis_le` est l'horloge murale du calcul —
                // l'axe temporel historique, c'est `debut_barre`.
                if e.debut_barre < t0 {
                    continue;
                }
                let verdict = e.detail.split('|').next().unwrap_or("Expire").to_string();
                let r = e.detail.split('|').nth(1).and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
                let r_ref = signaux
                    .get(e.cle_trade.as_str())
                    .and_then(|s| {
                        db::signaux_palier::r_reference_palier(
                            &verdict, "SMC", s.prix_entree, s.stop_loss, &s.take_profits,
                        )
                    })
                    .unwrap_or(r);
                clotures.push(ClotureRejeu {
                    asset: e.asset.as_str().to_string(),
                    tf: e.tf.as_str().to_string(),
                    ferme_le: e.debut_barre,
                    verdict,
                    r,
                    r_ref,
                    capital_apres: 0.0,
                });
            }
        }
    }

    clotures.sort_by_key(|c| c.ferme_le);
    // Recomposition chronologique du capital (les couples sont calculés dans
    // le désordre — on rejoue la composition sur la série triée).
    let mut capital2 = reg.capital;
    for c in &mut clotures {
        capital2 += c.r * capital2 * fraction;
        c.capital_apres = capital2;
    }

    let total = clotures.len();
    let gagnants = clotures.iter().filter(|c| c.r_ref > 0.0).count();
    let r_total = clotures.iter().map(|c| c.r_ref).sum();
    let r_total_realise = clotures.iter().map(|c| c.r).sum();

    Ok(RejeuSmc {
        tp1,
        calcule_le: chrono::Utc::now().timestamp(),
        nb_couples,
        nb_bougies,
        clotures,
        total,
        gagnants,
        taux_reussite: if total > 0 { gagnants as f64 / total as f64 } else { 0.0 },
        r_total,
        r_total_realise,
        capital_depart: reg.capital,
        fraction_risque: fraction,
        capital_actuel: capital2,
        duree_ms: 0,
    })
}
