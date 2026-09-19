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

use std::sync::{Arc, OnceLock};
use serde::Serialize;

/// Chauffe du moteur avant la période mesurée (indicateurs, structure,
/// liquidités) — les clôtures de la chauffe ne comptent pas dans les métriques.
const JOURS_CHAUFFE: i64 = 45;

#[derive(Debug, Clone, Serialize)]
pub struct ClotureRejeu {
    pub asset: String,
    pub tf: String,
    pub ferme_le: i64,
    pub verdict: String,
    /// R réalisé (sortie) du trade re-joué — moteur unitaire.
    pub r: f64,
    /// R pondéré après ventes partielles (f1 à TP1, f2 à TP2, solde) —
    /// c'est LUI qui compose le capital.
    pub r_pondere: f64,
    /// R de référence (palier max atteint) — métrique primaire du dashboard.
    pub r_ref: f64,
    /// Capital simulé après cette clôture (composé, risque du registre SMC).
    pub capital_apres: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct RejeuSmc {
    /// TP1 (× R) utilisé pour ce re-jeu.
    pub tp1: f64,
    /// TP2 (× R) utilisé pour ce re-jeu.
    pub tp2: f64,
    /// TP3 : mode (true = liquidité lointaine, false = R fixe) et cible/repli.
    pub tp3_lointaine: bool,
    pub tp3_rfixe: f64,
    /// Trailing stop après TP2 : None = inactif ; Some(k) = k×R de l'extrême.
    pub trailing_r: Option<f64>,
    /// Fractions des ventes partielles (défaut 50/30/20).
    pub fractions: crate::smc_pondere::Fractions,
    /// Empreinte de l'armement SMC par couple (outil Paramètres › SMC) —
    /// le périmètre des métriques suit les couples armés.
    pub empreinte_couples: String,
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
    /// Σ R réalisé (moteur unitaire).
    pub r_total_realise: f64,
    /// Σ R pondéré (ventes partielles) — ce que le capital a réellement composé.
    pub r_total_pondere: f64,
    pub capital_depart: f64,
    pub fraction_risque: f64,
    pub capital_actuel: f64,
}

/// GET /api/smc/rejeu — métriques SMC re-dérivées du TP1 réglé.
/// Déclenche le calcul à la demande s'il n'est pas déjà en cache.

/// Snapshot du cache (None = pas encore calculé).

/// Un recalcul est-il en vol (badge ⏳ côté dashboard).

/// Cache d'études du laboratoire (16/09) : re-jeux paramétrés, clé =
/// empreinte complète des paramètres, fraîcheur 30 min, capacité 8 (LRU).
/// Évite de recalculer un re-jeu déjà demandé (simulation puis balayage).
static CACHE_ETUDES: OnceLock<std::sync::Mutex<std::collections::HashMap<String, (i64, Arc<RejeuSmc>)>>> =
    OnceLock::new();

fn cache_etudes()
    -> &'static std::sync::Mutex<std::collections::HashMap<String, (i64, Arc<RejeuSmc>)>> {
    CACHE_ETUDES.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
}

/// Re-jeu paramétré avec cache — laboratoire (extension SMC 17/09).
/// Même moteur exact que calculer() ; le trailing (Option<k×R>) est le
/// levier d'étude du jour.
const REFRESH_SEC: i64 = 1_800;

pub async fn calculer_etude(
    pool: &Arc<db::Database>,
    tp1: f64,
    tp2: f64,
    tp3_lointaine: bool,
    tp3_rfixe: f64,
    trailing: Option<f64>,
    fractions: crate::smc_pondere::Fractions,
    filtre_assets: &[String],
    filtre_tfs: &[String],
) -> anyhow::Result<Arc<RejeuSmc>> {
    let empreinte_couples = crate::reglages_smc::empreinte_couples(
        &crate::reglages_smc::lire_couples_armes(pool).await,
    );
    let cle = format!(
        "{tp1}|{tp2}|{tp3_lointaine}|{tp3_rfixe}|{trailing:?}|{:?}|{empreinte_couples}|{:?}|{:?}",
        fractions, filtre_assets, filtre_tfs
    );
    let maintenant = chrono::Utc::now().timestamp();
    if let Ok(garde) = cache_etudes().lock() {
        if let Some((calcule_le, rejeu)) = garde.get(&cle) {
            if maintenant - calcule_le < REFRESH_SEC {
                return Ok(rejeu.clone());
            }
        }
    }
    let rejeu = Arc::new(
        calculer_avec_filtres(
            pool, tp1, tp2, tp3_lointaine, tp3_rfixe, trailing, fractions, &empreinte_couples,
            filtre_assets, filtre_tfs,
        )
        .await?,
    );
    if let Ok(mut garde) = cache_etudes().lock() {
        if garde.len() >= 8 {
            // LRU : évicter l'entrée la plus ancienne.
            if let Some(plus_vieille) = garde
                .iter()
                .max_by_key(|(_, (calcule_le, _))| -*calcule_le)
                .map(|(k, _)| k.clone())
            {
                garde.remove(&plus_vieille);
            }
        }
        garde.insert(cle, (maintenant, rejeu.clone()));
    }
    Ok(rejeu)
}

/// Re-jeu complet avec périmètre d'ÉTUDE optionnel (17/09) : filtres assets/TF
/// en plus de l'armement — le laboratoire simule un sous-ensemble sans jamais
/// toucher à l'armement réel.
#[allow(clippy::too_many_arguments)]
/// Mode TP3 (config smc_tp3_mode : "lointaine" | "rfixe" ; défaut lointaine).
pub async fn lire_tp3_lointaine(db: &db::Database) -> bool {
    db.lire_config("smc_tp3_mode")
        .await
        .ok()
        .flatten()
        .map(|v| !v.trim().eq_ignore_ascii_case("rfixe"))
        .unwrap_or(true)
}

/// R fixe TP3 (config smc_tp3_rfixe, défaut 3.0, borné 3.0-10.0).
pub async fn lire_tp3_rfixe(db: &db::Database) -> f64 {
    db.lire_config("smc_tp3_rfixe")
        .await
        .ok()
        .flatten()
        .and_then(|v| v.trim().parse::<f64>().ok())
        .map(|v| v.clamp(3.0, 10.0))
        .unwrap_or(3.0)
}

pub(crate) async fn calculer_avec_filtres(
    pool: &Arc<db::Database>,
    tp1: f64,
    tp2: f64,
    tp3_lointaine: bool,
    tp3_rfixe: f64,
    trailing: Option<f64>,
    fractions: crate::smc_pondere::Fractions,
    empreinte_couples: &str,
    filtre_assets: &[String],
    filtre_tfs: &[String],
) -> anyhow::Result<RejeuSmc> {
    use engine::TypeEvenementTrade as T;

    let assets = crate::runtime_tick::assets_runtime(pool).await;
    let timeframes = data::worker_config::lire_timeframes(pool).await;
    // Périmètre = couples ARMÉS (outil Paramètres › SMC — H1 jamais générateur).
    let armes = crate::reglages_smc::lire_couples_armes(pool).await;

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
            if !crate::reglages_smc::est_arme(&armes, asset.as_str(), tf.as_str()) {
                continue; // couple désarmé (ou H1) — hors métriques
            }
            if !filtre_assets.is_empty() && !filtre_assets.iter().any(|a| a == asset.as_str()) {
                continue; // périmètre d'étude (laboratoire)
            }
            if !filtre_tfs.is_empty() && !filtre_tfs.iter().any(|f| f == tf.as_str()) {
                continue; // périmètre d'étude (laboratoire)
            }
            let bougies = pool
                .obtenir_bougies_depuis_jours(asset, tf, jours)
                .await
                .unwrap_or_default();
            if bougies.is_empty() {
                continue;
            }
            nb_couples += 1;
            nb_bougies += bougies.len();
            let res = engine_v12::replay::rejouer_bougies_niveaux(
                asset.clone(), *tf, &bougies, amorce.clone(), tp1, tp2,
                smc::v12::signals::Tp3Reglage { lointaine: tp3_lointaine, rfixe: tp3_rfixe },
                trailing,
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
                // Distances réelles des paliers en R (TP1/TP2 réglables) —
                // base du R pondéré (ventes partielles).
                let (r_tp1, r_tp2) = signaux
                    .get(e.cle_trade.as_str())
                    .map(|s| {
                        let risque = (s.prix_entree - s.stop_loss).abs();
                        if risque > 0.0 {
                            (
                                (s.take_profits.first().copied().unwrap_or(s.prix_entree) - s.prix_entree).abs() / risque,
                                (s.take_profits.get(1).copied().unwrap_or(s.prix_entree) - s.prix_entree).abs() / risque,
                            )
                        } else {
                            (0.0, 0.0)
                        }
                    })
                    .unwrap_or((0.0, 0.0));
                // Rejeu paramétrique : pas de prix de sortie du solde en
                // base → repli mécanique (stop suiveur post-TP2 = TP1).
                let pondere =
                    crate::smc_pondere::r_pondere(&verdict, r, r_tp1, r_tp2, fractions, None);
                clotures.push(ClotureRejeu {
                    asset: e.asset.as_str().to_string(),
                    tf: e.tf.as_str().to_string(),
                    ferme_le: e.debut_barre,
                    verdict,
                    r,
                    r_pondere: pondere,
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
        capital2 += c.r_pondere * capital2 * fraction;
        c.capital_apres = capital2;
    }

    let total = clotures.len();
    let gagnants = clotures.iter().filter(|c| c.r_ref > 0.0).count();
    // WR : les Expire ne comptent pas au dénominateur (décision 05/09) —
    // même logique que les camemberts : un ordre jamais rempli ou une
    // attente morte n'est pas un trade pris. Ils restent dans `clotures`
    // (capital inchangé : R pondéré d'un Expire = 0).
    let total_wr = clotures.iter().filter(|c| c.verdict != "Expire").count();
    let r_total = clotures.iter().map(|c| c.r_ref).sum();
    let r_total_realise = clotures.iter().map(|c| c.r).sum();
    let r_total_pondere = clotures.iter().map(|c| c.r_pondere).sum();

    Ok(RejeuSmc {
        tp1,
        tp2,
        tp3_lointaine,
        tp3_rfixe,
        trailing_r: trailing,
        calcule_le: chrono::Utc::now().timestamp(),
        nb_couples,
        nb_bougies,
        clotures,
        total,
        gagnants,
        taux_reussite: if total_wr > 0 { gagnants as f64 / total_wr as f64 } else { 0.0 },
        r_total,
        r_total_realise,
        r_total_pondere,
        fractions,
        empreinte_couples: empreinte_couples.to_string(),
        capital_depart: reg.capital,
        fraction_risque: fraction,
        capital_actuel: capital2,
        duree_ms: 0,
    })
}

