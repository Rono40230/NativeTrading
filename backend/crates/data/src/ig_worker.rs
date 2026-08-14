//! Worker d'ingestion IG REST — forex + indices, 24/7, cycle 30 s.
//!
//! Boucle persistante qui interroge `GET /prices/{epic}` pour les actifs non-
//! crypto lus en DB (`assets.epic_ig`, source='ig') × les timeframes de la
//! configuration (`worker_timeframes`) et écrit les bougies en DB via
//! `inserer_bougies_avec_source` (INSERT OR IGNORE → idempotent).
//!
//! La liste des actifs et des timeframes est relue à CHAQUE cycle : activer ou
//! désactiver un asset, changer les timeframes ou l'historique depuis l'UI est
//! pris en compte en ≤ 30 s. Le flag `worker_actif_ig=0` met le worker en
//! sommeil sans le tuer.
//!
//! Backfill intelligent : au premier cycle, chaque combinaison stale (> 1
//! jour) ou absente est rechargée (dimensionnée sur `worker_historique_mois`
//! et l'écart réel, plafonné à 1000). Les cycles suivants fetch les 2 dernières
//! bougies fermées.
//!
//! Protection du quota IG (REST historique ≈ 10 000 data points/semaine) :
//! pacing d'une combinaison à la cadence de clôture de son timeframe (M5 →
//! 5 min, M15 → 15 min, H1 → 1 h, D1 → 6 h) et budget glissant 7 jours —
//! voir `budget.rs`. Seules les bougies fermées sont stockées — voir
//! `reponse.rs`.
//!
//! Résilience : garde anti-double-start, session partagée `Arc<Mutex<
//! IgSession>>` avec relogin auto (TTL 5h, circuit breaker), 401 → reset +
//! relogin au cycle suivant, échec par actif → WARN et on continue,
//! requêtes sérialisées espacées de 200 ms.

mod budget;
mod reponse;

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::Utc;
use common::{Asset, Timeframe};
use db::Database;
use tokio::sync::Mutex;
use tokio::time::sleep;

use crate::ig_session::IgSession;
use crate::worker_config::{lire_actif, lire_historique_mois, lire_timeframes, CLE_ACTIF_IG};
use crate::worker_status::STATUT_IG;
use budget::BudgetQuota;
use reponse::IgReponse;

/// Période du cycle d'ingestion (30 secondes).
const CYCLE: Duration = Duration::from_secs(30);
/// Espacement minimal entre deux requêtes REST IG (rate limit ~5 req/s).
const ESPACEMENT_REQUETE: Duration = Duration::from_millis(200);
/// Seuil de staleness déclenchant un backfill : données plus vieilles qu'1 jour.
const SEUIL_STALE_SEC: i64 = 86_400;
/// Plafond de sécurité d'un backfill, quelle que soit la profondeur demandée
/// (protège le quota IG — le budget glissant borne de toute façon).
const MAX_BACKFILL_SECURITE: usize = 1000;
/// Nombre de bougies fetch en update périodique (2 = la fermée + la marge).
const MAX_UPDATE: usize = 2;
/// Petite pause anti-boucle-chaude quand un cycle (backfill) dépasse 30 s.
const PAUSE_SECURITE: Duration = Duration::from_secs(1);
/// Marge après la clôture théorique avant de refetch un timeframe.
const MARGE_FERMETURE: Duration = Duration::from_secs(15);
/// Ré-examen d'une combinaison après un échec de fetch (réseau, 403…).
const RETRY_ECHEC: Duration = Duration::from_secs(60);

/// Source enregistrée en DB pour les bougies issues de ce worker.
const SOURCE: &str = "ig_worker";

/// Garde anti-double-start. Le worker doit n'être spawné qu'une fois.
/// Pattern identique à `BYBIT_WS_DEMARRE` dans `data::bybit_ws`.
static IG_WORKER_DEMARRE: AtomicBool = AtomicBool::new(false);

/// Marque le worker comme démarré. `true` au premier appel seulement.
fn marquer_demarre() -> bool {
    !IG_WORKER_DEMARRE.swap(true, Ordering::SeqCst)
}

/// Filtre les assets DB pour ce worker : `source='ig' AND actif AND epic_ig IS
/// NOT NULL` → couples `(asset_id, epic)`. Fonction pure → testable.
fn filtrer_assets_ig(assets: Vec<db::assets::AssetWorker>) -> Vec<(String, String)> {
    assets
        .into_iter()
        .filter(|a| a.actif && a.source == "ig")
        .filter_map(|a| a.epic_ig.map(|epic| (a.id, epic)))
        .collect()
}

/// Lit depuis la DB les actifs à ingérer via IG, résolus en `Asset` du crate
/// common. Un identifiant inconnu est loggué ERROR puis ignoré — jamais de
/// panic. Toute erreur DB retourne une liste vide (retry au cycle suivant).
async fn assets_ig_depuis_db(db: &Arc<Database>) -> Vec<(Asset, String)> {
    match db.lister_assets_worker().await {
        Ok(assets) => filtrer_assets_ig(assets)
            .into_iter()
            .filter_map(|(id, epic)| match Asset::try_from(id.as_str()) {
                Ok(asset) => Some((asset, epic)),
                Err(e) => {
                    tracing::error!("IG worker: asset inconnu {} — ignoré ({})", id, e);
                    None
                }
            })
            .collect(),
        Err(e) => {
            tracing::warn!("IG worker: lecture DB des actifs impossible ({}) — retry plus tard", e);
            Vec::new()
        }
    }
}

/// Nombre approximatif de bougies par mois pour un timeframe (base 24/7 ;
/// le forex/indices ont des sessions plus courtes, mais ce calcul ne sert que
/// de plafond de dimensionnement de backfill).
fn bougies_par_mois(tf: Timeframe) -> u64 {
    30 * 24 * 60 / tf.minutes().max(1)
}

/// Dimensionne le backfill complet selon la profondeur d'historique configurée
/// (`worker_historique_mois`), plafonnée à `MAX_BACKFILL_SECURITE`.
fn cible_backfill(tf: Timeframe, mois: i64) -> usize {
    let mois = mois.clamp(1, 24) as u64;
    (bougies_par_mois(tf).saturating_mul(mois) as usize).min(MAX_BACKFILL_SECURITE).max(MAX_UPDATE)
}

/// Durée entre deux fetchs d'une même combinaison : la cadence de clôture du
/// timeframe (une bougie M5 ne ferme qu'une fois toutes les 5 minutes —
/// fetch plus souvent renvoie chaque fois les mêmes données et brûle le
/// quota pour rien). D1 est revu toutes les 6 h.
fn periode_rafraichissement(tf: &Timeframe) -> Duration {
    match tf {
        Timeframe::D1 => Duration::from_secs(6 * 3600),
        autre => Duration::from_secs(autre.minutes() * 60),
    }
}

/// Dimensionne le fetch selon l'état de la DB pour une combinaison asset/tf :
///
/// - aucune bougie → backfill complet (`cible`) ;
/// - backfill autorisé (premier cycle) et écart > 1 jour → juste assez de
///   bougies pour couvrir l'écart (+ marge), plafonné à `cible` ;
/// - sinon → update normal (2 bougies).
///
/// En cycle normal (`backfill_autorise = false`) on ne backfill JAMAIS : un
/// marché fermé un week-end ne doit pas déclencher des dizaines de gros fetchs
/// en boucle.
fn calculer_max(derniere_ts: Option<i64>, tf: Timeframe, backfill_autorise: bool, cible: usize) -> usize {
    let Some(ts) = derniere_ts else {
        return cible;
    };
    if !backfill_autorise {
        return MAX_UPDATE;
    }
    let ecart_sec = Utc::now().timestamp() - ts;
    if ecart_sec <= SEUIL_STALE_SEC {
        return MAX_UPDATE; // données fraîches → update normal
    }
    // Stale : couvrir l'écart réel (ex. D1 à J+2 → ~4 bougies, pas la cible).
    let duree_tf_sec = tf.minutes() as i64 * 60;
    ((ecart_sec / duree_tf_sec + 3) as usize).clamp(MAX_UPDATE, cible)
}

// ─── Fetch REST ───────────────────────────────────────────────────────────────

/// Résultat d'un fetch unitaire — gouverne la suite du cycle.
enum ResultatFetch {
    /// Bougies récupérées (possiblement vides : marché fermé).
    Ok(Vec<common::Candle>),
    /// 401 : la session est morte → avorter le cycle, relogin au suivant.
    SessionInvalide,
    /// Erreur locale à cet actif → logguer WARN et continuer les autres.
    Echec(String),
}

/// Appelle `GET {base}/prices/{epic}?resolution={res}&max={n}&pageSize=0`
/// avec les headers de session fournis (récupérés une fois par cycle).
async fn fetch_bougies(
    client: &reqwest::Client,
    url_base: &str,
    headers: &reqwest::header::HeaderMap,
    epic: &str,
    tf: &Timeframe,
    max: usize,
) -> ResultatFetch {
    let resolution = crate::providers::ig::resolution_pour_tf(tf);
    let url = format!(
        "{}/prices/{}?resolution={}&max={}&pageSize=0",
        url_base, epic, resolution, max
    );

    let reponse = match client
        .get(&url)
        .headers(headers.clone())
        .header("Version", "3")
        .header("Accept", "application/json; charset=UTF-8")
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => return ResultatFetch::Echec(format!("réseau: {}", e)),
    };

    let statut = reponse.status();
    if !statut.is_success() {
        return if statut.as_u16() == 401 {
            ResultatFetch::SessionInvalide
        } else {
            // 403 = clé sans permission pour cet epic (fréquent en demo) →
            // erreur locale à l'actif, on continue avec les autres.
            ResultatFetch::Echec(format!("HTTP {}", statut))
        };
    }

    match reponse.json::<IgReponse>().await {
        Ok(data) => ResultatFetch::Ok(reponse::convertir_bougies(&data, tf)),
        Err(e) => ResultatFetch::Echec(format!("parse JSON: {}", e)),
    }
}

// ─── Boucle principale ────────────────────────────────────────────────────────

/// Démarre le worker en arrière-plan — non bloquant. Idempotent : un second
/// appel est un no-op + avertissement.
pub fn demarrer_worker_ig(db: Arc<Database>, ig_session: Arc<Mutex<IgSession>>) {
    if !marquer_demarre() {
        tracing::warn!("⚠️  Worker IG REST déjà démarré — second spawn ignoré");
        return;
    }
    tokio::spawn(async move {
        boucle_worker(db, ig_session).await;
        // Ne devrait jamais arriver : la boucle est infinie. Si elle sort, on
        // libère le garde pour permettre un redémarrage manuel ultérieur.
        IG_WORKER_DEMARRE.store(false, Ordering::SeqCst);
        tracing::error!("IG worker: boucle principale terminée — ingestion arrêtée");
    });
}

/// État mutable du worker, porté d'un cycle à l'autre.
struct EtatWorker {
    /// true tant que le backfill des données stale n'a pas abouti.
    backfill_autorise: bool,
    /// Prochaine échéance de fetch par combinaison `(asset_id, timeframe)`.
    /// HashMap : les combos sont dynamiques (relus en DB à chaque cycle).
    prochaines: HashMap<(String, String), Instant>,
    /// Budget de data points sur fenêtre glissante.
    budget: BudgetQuota,
}

/// Boucle infinie : un cycle toutes les 30 s. À chaque cycle on relit la DB
/// (actifs, timeframes, historique, interrupteur) — la configuration UI
/// s'applique sans redémarrage. Le premier cycle autorise le backfill
/// (données stale), les suivants ne font que des updates à la cadence de
/// clôture de chaque timeframe. Le temps passé à fetcher est déduit de
/// l'attente (cycle >= 30 s).
async fn boucle_worker(db: Arc<Database>, ig_session: Arc<Mutex<IgSession>>) {
    tracing::info!(
        "📊 IG worker: démarrage ingestion REST (actifs/timeframes pilotés en DB, cycle {:?}, source '{}')",
        CYCLE,
        SOURCE
    );

    let mut etat = EtatWorker {
        backfill_autorise: true,
        prochaines: HashMap::new(),
        budget: BudgetQuota::new(),
    };

    loop {
        let debut = Instant::now();

        // Interrupteur UI : worker désactivé → sommeil, pas de fetch.
        if !lire_actif(&db, CLE_ACTIF_IG).await {
            tracing::debug!("IG worker: désactivé (worker_actif_ig=0) — cycle sauté");
            STATUT_IG.marque_deconnecte();
            sleep(CYCLE).await;
            continue;
        }

        // Listes dynamiques relues à chaque cycle.
        let actifs = assets_ig_depuis_db(&db).await;
        let timeframes = lire_timeframes(&db).await;
        let mois = lire_historique_mois(&db).await;
        if actifs.is_empty() || timeframes.is_empty() {
            tracing::warn!(
                "IG worker: aucun actif (source='ig', epic_ig) ou timeframe à suivre — cycle sauté"
            );
            sleep(CYCLE).await;
            continue;
        }

        cycle(&db, &ig_session, &actifs, &timeframes, mois, &mut etat).await;

        // Attente du reste du cycle ; si le cycle a dépassé 30 s (backfill
        // initial), petite pause de sécurité avant de repartir.
        if let Some(restant) = CYCLE.checked_sub(debut.elapsed()) {
            sleep(restant).await;
        } else {
            sleep(PAUSE_SECURITE).await;
        }
    }
}

/// Un cycle complet : headers de session une seule fois, puis pour chaque
/// combinaison due (pacing par timeframe + budget disponible), appel REST
/// espacé de 200 ms et écriture en DB. Résilient : aucune erreur ne fait
/// sortir la boucle.
async fn cycle(
    db: &Arc<Database>,
    ig_session: &Arc<Mutex<IgSession>>,
    actifs: &[(Asset, String)],
    timeframes: &[Timeframe],
    historique_mois: i64,
    etat: &mut EtatWorker,
) {
    let debut_cycle = Instant::now();

    // Session : relogin automatique si expirée (circuit breaker intégré).
    let (client, url_base, headers) = {
        let mut sess = ig_session.lock().await;
        match sess.headers(db).await {
            Ok(h) => (sess.client().clone(), sess.url().to_string(), h),
            Err(e) => {
                tracing::error!(
                    "IG worker: session indisponible — cycle avorté, retry dans {:?} ({})",
                    CYCLE,
                    e
                );
                STATUT_IG.marque_deconnecte();
                return;
            }
        }
    };
    STATUT_IG.marque_connecte(actifs.len() as u64);

    let maintenant = Instant::now();
    let (mut requetes, mut inserees, mut echecs) = (0usize, 0u64, 0usize);
    let mut session_invalide = false;

    // Pacing : élaguer les échéances des combos retirés de la config (asset
    // désactivé, timeframe retiré) pour que la map ne grossisse pas indéfiniment.
    let combos_valides: HashSet<(String, String)> = actifs
        .iter()
        .flat_map(|(asset, _)| {
            timeframes
                .iter()
                .map(move |tf| (asset.as_str().to_string(), tf.as_str().to_string()))
        })
        .collect();
    etat.prochaines.retain(|cle, _| combos_valides.contains(cle));

    'actifs: for (asset, epic) in actifs.iter() {
        for tf in timeframes.iter() {
            let cle = (asset.as_str().to_string(), tf.as_str().to_string());

            // Pacing : rien de neuf attendu avant la prochaine clôture.
            if let Some(echeance) = etat.prochaines.get(&cle) {
                if &maintenant < echeance {
                    continue;
                }
            }

            let derniere = match db.timestamp_derniere_bougie_chart(asset, tf).await {
                Ok(t) => t,
                Err(e) => {
                    tracing::warn!(
                        "IG worker: lecture DB {} {}: {} — update par défaut",
                        asset.as_str(),
                        tf.as_str(),
                        e
                    );
                    None
                }
            };
            let cible = cible_backfill(*tf, historique_mois);
            let max = calculer_max(derniere, *tf, etat.backfill_autorise, cible);
            // Budget IG : une requête ne part que si son coût max tient.
            if !etat.budget.autorise(max) {
                break 'actifs; // fenêtre pleine : inutile de parcourir le reste
            }

            let resultat = fetch_bougies(&client, &url_base, &headers, epic, tf, max).await;
            requetes += 1;

            // Prochaine échéance selon l'issue (retry rapide sur échec).
            let rafraichissement = match &resultat {
                ResultatFetch::Ok(_) => periode_rafraichissement(tf) + MARGE_FERMETURE,
                ResultatFetch::Echec(_) | ResultatFetch::SessionInvalide => RETRY_ECHEC,
            };
            etat.prochaines.insert(cle, Instant::now() + rafraichissement);

            match resultat {
                ResultatFetch::Ok(bougies) => {
                    etat.budget.consigner(bougies.len());
                    if !bougies.is_empty() {
                        // Timestamp de la plus récente bougie du lot (pour le statut).
                        let ts_derniere = bougies.last().map(|b| b.timestamp.timestamp());
                        match db
                            .inserer_bougies_avec_source(asset, tf, &bougies, SOURCE)
                            .await
                        {
                            Ok(n) => {
                                inserees += n;
                                if let Some(ts) = ts_derniere {
                                    STATUT_IG.consigne_bougies(ts, n);
                                }
                            }
                            Err(e) => tracing::warn!(
                                "IG worker: écriture DB {} {}: {}",
                                asset.as_str(),
                                tf.as_str(),
                                e
                            ),
                        }
                    }
                }
                ResultatFetch::Echec(raison) => {
                    echecs += 1;
                    tracing::warn!(
                        "IG worker: fetch échoué {} {} ({}) — on continue",
                        asset.as_str(),
                        tf.as_str(),
                        raison
                    );
                }
                ResultatFetch::SessionInvalide => {
                    // Session morte en plein cycle : les requêtes restantes
                    // échoueraient aussi → on coupe et force un relogin.
                    tracing::warn!(
                        "IG worker: session expirée (401) sur {} {} — relogin au prochain cycle",
                        asset.as_str(),
                        tf.as_str()
                    );
                    session_invalide = true;
                    break 'actifs;
                }
            }

            // Rate limit IG : jamais deux requêtes collées.
            sleep(ESPACEMENT_REQUETE).await;
        }
    }

    if session_invalide {
        STATUT_IG.marque_deconnecte();
        ig_session.lock().await.reset();
        return;
    }

    // Le backfill d'amorçage reste autorisé tant que le budget l'a empêché
    // de terminer (reprise au fil de la fenêtre glissante, sans restart).
    if etat.budget.autorise(MAX_UPDATE) {
        etat.backfill_autorise = false;
    }

    if inserees > 0 {
        tracing::info!(
            "IG worker: cycle terminé en {:?} — {} bougie(s) insérée(s), {} requête(s), {} échec(s)",
            debut_cycle.elapsed(),
            inserees,
            requetes,
            echecs
        );
    } else {
        tracing::debug!(
            "IG worker: cycle terminé ({} requête(s), {} échec(s), rien de nouveau)",
            requetes,
            echecs
        );
    }
}

// ─── Tests unitaires (pas de réseau, pas de DB) ───────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn asset_worker(id: &str, source: &str, epic: Option<&str>, actif: bool) -> db::assets::AssetWorker {
        db::assets::AssetWorker {
            id: id.to_string(),
            source: source.to_string(),
            symbol_bybit: None,
            epic_ig: epic.map(|e| e.to_string()),
            actif,
        }
    }

    #[test]
    fn filtrer_assets_ig_selectionne_source_ig_actifs_avec_epic() {
        let assets = vec![
            asset_worker("EURUSD", "ig", Some("CS.D.EURUSD.CFD.IP"), true),
            asset_worker("DAX", "ig", Some("IX.D.DAX.IFD.IP"), true),
            // Crypto routée vers Bybit → exclue.
            asset_worker("BTC", "binance", Some("FAKE"), true),
            // Inactif → exclue.
            asset_worker("GBPUSD", "ig", Some("CS.D.GBPUSD.CFD.IP"), false),
            // Actif IG sans epic → exclue.
            asset_worker("XPTUSD", "ig", None, true),
        ];
        let retenus = filtrer_assets_ig(assets);
        assert_eq!(retenus.len(), 2);
        assert_eq!(retenus[0], ("EURUSD".to_string(), "CS.D.EURUSD.CFD.IP".to_string()));
        assert_eq!(retenus[1], ("DAX".to_string(), "IX.D.DAX.IFD.IP".to_string()));
    }

    #[test]
    fn cible_backfill_dimensionnee_sur_lhistorique_configure() {
        // D1 sur 6 mois → 180 bougies.
        assert_eq!(cible_backfill(Timeframe::D1, 6), 180);
        // H1 sur 3 mois → 2160 → plafonné au maximum de sécurité.
        assert_eq!(cible_backfill(Timeframe::H1, 3), MAX_BACKFILL_SECURITE);
        // M5 sur 1 mois → déjà plafonné ; 0 mois est borné à 1.
        assert_eq!(cible_backfill(Timeframe::M5, 0), MAX_BACKFILL_SECURITE);
        // Jamais sous MAX_UPDATE.
        assert!(cible_backfill(Timeframe::W1, 1) >= MAX_UPDATE);
    }

    #[test]
    fn periodes_de_rafraichissement() {
        for (tf, attendu) in [
            (Timeframe::M5, 300u64),
            (Timeframe::M15, 900),
            (Timeframe::H1, 3600),
            (Timeframe::D1, 6 * 3600), // D1 revu toutes les 6 h, pas 24 h
        ] {
            assert_eq!(
                periode_rafraichissement(&tf),
                Duration::from_secs(attendu)
            );
        }
    }

    #[test]
    fn calculer_max_selon_letat_db() {
        let maintenant = Utc::now().timestamp();
        let cible = cible_backfill(Timeframe::M5, 6); // = MAX_BACKFILL_SECURITE
        // Aucune donnée → backfill complet, même en cycle normal.
        assert_eq!(calculer_max(None, Timeframe::M5, true, cible), cible);
        assert_eq!(calculer_max(None, Timeframe::M5, false, cible), cible);
        // Données fraîches → update.
        assert_eq!(
            calculer_max(Some(maintenant - 60), Timeframe::M5, true, cible),
            MAX_UPDATE
        );
        // D1 stale de ~30 h → 1 bougie entière + marge 3 = 4, pas la cible.
        let cible_d1 = cible_backfill(Timeframe::D1, 6);
        assert_eq!(
            calculer_max(Some(maintenant - 30 * 3600), Timeframe::D1, true, cible_d1),
            4
        );
        // M5 avec 3 jours de trou → ~867 bougies pour couvrir l'écart réel,
        // sans jamais dépasser la cible (une seconde d'horlope peut s'ajouter).
        let pour_trou = calculer_max(Some(maintenant - 3 * 86_400), Timeframe::M5, true, cible);
        assert!((867..=cible).contains(&pour_trou), "attendu ~867..=cible, obtenu {}", pour_trou);
        // Week-end (écart énorme) mais cycle normal → jamais de backfill.
        assert_eq!(
            calculer_max(Some(maintenant - 7 * 86_400), Timeframe::M5, false, cible),
            MAX_UPDATE
        );
    }

    #[test]
    fn garde_anti_double_start() {
        // On manipule directement la statique pour ce test ; on la remet dans
        // son état initial ensuite afin de ne pas polluer les autres tests.
        let avant = IG_WORKER_DEMARRE
            .compare_exchange(false, false, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok();
        let premier = marquer_demarre();
        let second = marquer_demarre();
        IG_WORKER_DEMARRE.store(false, Ordering::SeqCst);
        assert!(avant, "la garde devait être à false au départ du test");
        assert!(premier, "le premier marquage doit renvoyer true");
        assert!(!second, "le second marquage doit renvoyer false");
    }
}
