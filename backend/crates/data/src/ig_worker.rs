//! Worker d'ingestion IG REST — forex + indices, 24/7, cycle 30 s.
//!
//! Boucle persistante qui interroge `GET /prices/{epic}` pour les 19 actifs
//! non-crypto (12 forex + 7 indices) × 4 timeframes (M5, M15, H1, D1) et
//! écrit les bougies en DB via `inserer_bougies_avec_source` (INSERT OR
//! IGNORE → idempotent).
//!
//! Backfill intelligent : au premier cycle, chaque combinaison stale (> 1
//! jour) ou absente est rechargée (200 bougies max, dimensionné sur l'écart
//! réel). Les cycles suivants fetch les 2 dernières bougies fermées.
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

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::Utc;
use common::{Asset, Timeframe};
use db::Database;
use tokio::sync::Mutex;
use tokio::time::sleep;

use crate::ig_session::IgSession;
use budget::BudgetQuota;
use reponse::IgReponse;

/// Période du cycle d'ingestion (30 secondes).
const CYCLE: Duration = Duration::from_secs(30);
/// Espacement minimal entre deux requêtes REST IG (rate limit ~5 req/s).
const ESPACEMENT_REQUETE: Duration = Duration::from_millis(200);
/// Seuil de staleness déclenchant un backfill : données plus vieilles qu'1 jour.
const SEUIL_STALE_SEC: i64 = 86_400;
/// Nombre de bougies maximum par backfill.
const MAX_BACKFILL: usize = 200;
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

/// Actifs couverts : 12 paires forex + 7 indices (asset DB, epic IG).
const ASSETS_IG: &[(&str, &str)] = &[
    ("EURUSD", "CS.D.EURUSD.CFD.IP"),
    ("GBPJPY", "CS.D.GBPJPY.CFD.IP"),
    ("USDJPY", "CS.D.USDJPY.CFD.IP"),
    ("GBPUSD", "CS.D.GBPUSD.CFD.IP"),
    ("USDCHF", "CS.D.USDCHF.CFD.IP"),
    ("AUDUSD", "CS.D.AUDUSD.CFD.IP"),
    ("USDCAD", "CS.D.USDCAD.CFD.IP"),
    ("NZDUSD", "CS.D.NZDUSD.CFD.IP"),
    ("CADJPY", "CS.D.CADJPY.CFD.IP"),
    ("NZDJPY", "CS.D.NZDJPY.CFD.IP"),
    ("EURJPY", "CS.D.EURJPY.CFD.IP"),
    ("EURGBP", "CS.D.EURGBP.CFD.IP"),
    ("DAX", "IX.D.DAX.IFD.IP"),
    ("NAS100", "IX.D.NASDAQ.IFD.IP"),
    ("SP500", "IX.D.SPTRD.IFD.IP"),
    ("US30", "IX.D.DOW.IFD.IP"),
    ("FTSE100", "IX.D.FTSE.IFD.IP"),
    ("CAC40", "IX.D.CAC.IFD.IP"),
    ("JP225", "IX.D.NIKKEI.IFD.IP"),
];

/// Timeframes couverts (les plus importants pour SMC + sentiment).
const TIMEFRAMES_IG: &[Timeframe] = &[Timeframe::M5, Timeframe::M15, Timeframe::H1, Timeframe::D1];

/// Garde anti-double-start. Le worker doit n'être spawné qu'une fois.
/// Pattern identique à `BYBIT_WS_DEMARRE` dans `data::bybit_ws`.
static IG_WORKER_DEMARRE: AtomicBool = AtomicBool::new(false);

/// Marque le worker comme démarré. `true` au premier appel seulement.
fn marquer_demarre() -> bool {
    !IG_WORKER_DEMARRE.swap(true, Ordering::SeqCst)
}

/// Résout la liste des actifs suivis une fois pour toutes au démarrage.
/// Un identifiant inconnu est loggué ERROR puis ignoré — jamais de panic.
fn actifs_ig() -> Vec<(Asset, &'static str)> {
    ASSETS_IG
        .iter()
        .filter_map(|(nom, epic)| match Asset::try_from(*nom) {
            Ok(asset) => Some((asset, *epic)),
            Err(e) => {
                tracing::error!("IG worker: asset inconnu {} — ignoré ({})", nom, e);
                None
            }
        })
        .collect()
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
/// - aucune bougie → backfill complet (200) ;
/// - backfill autorisé (premier cycle) et écart > 1 jour → juste assez de
///   bougies pour couvrir l'écart (+ marge), plafonné à 200 ;
/// - sinon → update normal (2 bougies).
///
/// En cycle normal (`backfill_autorise = false`) on ne backfill JAMAIS : un
/// marché fermé un week-end ne doit pas déclencher 76 gros fetchs en boucle.
fn calculer_max(derniere_ts: Option<i64>, tf: Timeframe, backfill_autorise: bool) -> usize {
    let Some(ts) = derniere_ts else {
        return MAX_BACKFILL;
    };
    if !backfill_autorise {
        return MAX_UPDATE;
    }
    let ecart_sec = Utc::now().timestamp() - ts;
    if ecart_sec <= SEUIL_STALE_SEC {
        return MAX_UPDATE; // données fraîches → update normal
    }
    // Stale : couvrir l'écart réel (ex. D1 à J+2 → ~4 bougies, pas 200).
    let duree_tf_sec = tf.minutes() as i64 * 60;
    ((ecart_sec / duree_tf_sec + 3) as usize).clamp(MAX_UPDATE, MAX_BACKFILL)
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
        // libère la garde pour permettre un redémarrage manuel ultérieur.
        IG_WORKER_DEMARRE.store(false, Ordering::SeqCst);
        tracing::error!("IG worker: boucle principale terminée — ingestion arrêtée");
    });
}

/// État mutable du worker, porté d'un cycle à l'autre.
struct EtatWorker {
    /// true tant que le backfill des données stale n'a pas abouti.
    backfill_autorise: bool,
    /// Prochaine échéance de fetch par combinaison actif×timeframe.
    prochaines: Vec<Option<Instant>>,
    /// Budget de data points sur fenêtre glissante.
    budget: BudgetQuota,
}

/// Boucle infinie : un cycle toutes les 30 s. Le premier cycle autorise le
/// backfill (données stale), les suivants ne font que des updates à la
/// cadence de clôture de chaque timeframe. Le temps passé à fetcher est
/// déduit de l'attente (cycle >= 30 s).
async fn boucle_worker(db: Arc<Database>, ig_session: Arc<Mutex<IgSession>>) {
    let actifs = actifs_ig();
    tracing::info!(
        "📊 IG worker: démarrage ingestion REST ({} actifs × {} timeframes, cycle {:?}, source '{}')",
        actifs.len(),
        TIMEFRAMES_IG.len(),
        CYCLE,
        SOURCE
    );

    let mut etat = EtatWorker {
        backfill_autorise: true,
        prochaines: vec![None; actifs.len() * TIMEFRAMES_IG.len()],
        budget: BudgetQuota::new(),
    };

    loop {
        let debut = Instant::now();
        cycle(&db, &ig_session, &actifs, &mut etat).await;
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
    actifs: &[(Asset, &'static str)],
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
                return;
            }
        }
    };

    let maintenant = Instant::now();
    let (mut requetes, mut inserees, mut echecs) = (0usize, 0u64, 0usize);
    let mut session_invalide = false;

    'actifs: for (i, (asset, epic)) in actifs.iter().enumerate() {
        for (j, tf) in TIMEFRAMES_IG.iter().enumerate() {
            let idx = i * TIMEFRAMES_IG.len() + j;

            // Pacing : rien de neuf attendu avant la prochaine clôture.
            if let Some(echeance) = etat.prochaines[idx] {
                if maintenant < echeance {
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
            let max = calculer_max(derniere, *tf, etat.backfill_autorise);
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
            etat.prochaines[idx] = Some(Instant::now() + rafraichissement);

            match resultat {
                ResultatFetch::Ok(bougies) => {
                    etat.budget.consigner(bougies.len());
                    if !bougies.is_empty() {
                        match db
                            .inserer_bougies_avec_source(asset, tf, &bougies, SOURCE)
                            .await
                        {
                            Ok(n) => inserees += n,
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

    #[test]
    fn assets_ig_coherents_avec_providers() {
        // Chaque (asset, epic) du worker doit correspondre au mapping
        // officiel de providers::ig — protège contre toute divergence.
        for (nom, epic) in ASSETS_IG {
            let asset = Asset::try_from(*nom).expect("asset connu du crate common");
            assert_eq!(
                crate::providers::ig::epic_pour_asset(&asset),
                Some(*epic),
                "{}",
                nom
            );
        }
        // 12 forex + 7 indices, tous résolus, 4 timeframes.
        assert_eq!(ASSETS_IG.len(), 19);
        assert_eq!(actifs_ig().len(), 19);
        assert_eq!(
            TIMEFRAMES_IG,
            &[Timeframe::M5, Timeframe::M15, Timeframe::H1, Timeframe::D1]
        );
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
        // Aucune donnée → backfill complet, même en cycle normal.
        assert_eq!(calculer_max(None, Timeframe::M5, true), MAX_BACKFILL);
        assert_eq!(calculer_max(None, Timeframe::M5, false), MAX_BACKFILL);
        // Données fraîches → update.
        assert_eq!(
            calculer_max(Some(maintenant - 60), Timeframe::M5, true),
            MAX_UPDATE
        );
        // D1 stale de ~30 h → 1 bougie entière + marge 3 = 4, pas 200.
        assert_eq!(
            calculer_max(Some(maintenant - 30 * 3600), Timeframe::D1, true),
            4
        );
        // M5 avec 3 jours de trou → plafonné à 200.
        assert_eq!(
            calculer_max(Some(maintenant - 3 * 86_400), Timeframe::M5, true),
            MAX_BACKFILL
        );
        // Week-end (écart énorme) mais cycle normal → jamais de backfill.
        assert_eq!(
            calculer_max(Some(maintenant - 7 * 86_400), Timeframe::M5, false),
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
