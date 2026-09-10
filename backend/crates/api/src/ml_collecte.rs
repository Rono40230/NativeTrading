//! §11 étape 1 (06/09) — boucle ML v2 : rattrapage de l'existant.
//!
//! La collecte continue est branchée à `fermer_signal_par_cle` (toutes les
//! futures clôtures SMC/straddle/rockets alimentent `ml_training_samples`).
//! Ce module rejoue l'HISTORIQUE : les signaux fermés remplis de la base
//! deviennent des samples — idempotent par `signal_id` (index unique),
//! donc relançable à chaque boot sans doublon.
//!
//! Convention : la base VÉCUE est la vérité terrain (les verdicts tels que
//! gérés) ; le re-jeu paramétrique reste un outil d'étude de réglages et
//! n'entre pas dans le corpus d'apprentissage. Les expirés sont inclus
//! (outcome 'expire'/'invalide' : un setup qui ne vit pas est une classe
//! prédictible utile).

use std::sync::Arc;

use db::Database;

/// Rejoue tout l'historique fermé en samples. Retourne le nombre de
/// nouveaux samples insérés (les doublons sont ignorés).
pub async fn rattraper(db: &Arc<Database>) -> usize {
    let rows = match sqlx::query(
        "SELECT id, strategie, asset, timeframe, direction, prix_entree,
                COALESCE(prix_verdict, prix_entree) AS sortie, stop_loss,
                verdict, r_realise
         FROM signaux
         WHERE statut = 'Fermé' AND verdict IS NOT NULL
           AND heure_entree IS NOT NULL AND prix_verdict IS NOT NULL",
    )
    .fetch_all(db.pool())
    .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("🧠 ML rattrapage : lecture impossible — {e}");
            return 0;
        }
    };

    use sqlx::Row as _;
    let mut insérés = 0usize;
    for r in &rows {
        let sample = db::ml_samples::MlSample {
            strategie: r.get::<String, _>("strategie"),
            asset: r.get::<String, _>("asset"),
            timeframe: r.get::<String, _>("timeframe"),
            direction: r.get::<String, _>("direction"),
            prix_entree: r.get::<f64, _>("prix_entree"),
            prix_sortie: r.get::<f64, _>("sortie"),
            stop_loss: r.get::<f64, _>("stop_loss"),
            outcome: r.get::<String, _>("verdict"),
            rr_realise: r.get::<Option<f64>, _>("r_realise"),
            signal_id: Some(r.get::<String, _>("id")),
        };
        match db::ml_samples::sauvegarder_sample(db.pool(), &sample).await {
            Ok(_) => insérés += 1,
            Err(e) => tracing::warn!("🧠 ML rattrapage : sample échoué — {e}"),
        }
    }

    let total = rows.len();
    if insérés > 0 {
        tracing::info!(
            "🧠 ML v2 : rattrapage — {insérés} nouveaux samples ({total} clôtures candidates, doublons ignorés)"
        );
    }
    insérés
}

/// Boucle de fond : un rattrapage au boot (les clôtures arrivées pendant
/// l'arrêt de l'app sont rattrapées), puis la collecte continue vit dans
/// `fermer_signal_par_cle` — rien à faire de plus.
pub async fn boucle_rattrapage(db: Arc<Database>) {
    // Laisse les moteurs démarrer (les clôtures boot entrent par le
    // collecteur continu ; le rattrapage ne regarde que le passé).
    tokio::time::sleep(std::time::Duration::from_secs(30)).await;
    rattraper(&db).await;
    rattraper_features(&db).await;
}

/// §11 étape 2 — backfill des FEATURES : pour chaque sample déjà collecté,
/// reconstitue le snapshot de features depuis les bougies historiques
/// autours de l'émission (52 OHLCV via `ml::extraire_features`). Les 7
/// contextuelles SMC ne sont pas reconstituables rétroactivement (journali-
/// sation du scoring = §8) — elles sont posées à 0 et DOCUMENTÉES dans la
/// doc du module. Idempotent : ne sélectionne QUE les signaux sans
/// snapshot (crash du 10/09 : le re-traitement intégral à chaque boot
/// broyait ~576k lignes × 315 samples pendant 10 min → corruption du tas).
pub async fn rattraper_features(db: &Arc<Database>) -> usize {
    use sqlx::Row as _;

    let rows = match sqlx::query(
        "SELECT s.id, s.strategie, s.asset, s.timeframe, s.cree_le
         FROM signaux s
         JOIN ml_training_samples m ON m.signal_id = s.id
         WHERE s.statut = 'Fermé' AND s.heure_entree IS NOT NULL
           AND (
             (LOWER(s.strategie) LIKE '%smc%'
              AND NOT EXISTS (SELECT 1 FROM smc_features_snapshot f WHERE f.signal_id = s.id))
             OR (LOWER(s.strategie) LIKE '%straddle%'
              AND NOT EXISTS (SELECT 1 FROM straddle_features_snapshot f WHERE f.signal_id = s.id))
             OR (LOWER(s.strategie) NOT LIKE '%smc%' AND LOWER(s.strategie) NOT LIKE '%straddle%'
              AND NOT EXISTS (SELECT 1 FROM rockets_features_snapshot f WHERE f.signal_id = s.id))
           )",
    )
    .fetch_all(db.pool())
    .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("🧠 ML features : lecture impossible — {e}");
            return 0;
        }
    };

    let maintenant = chrono::Utc::now().timestamp();
    let mut écrits = 0usize;
    for r in &rows {
        let id: String = r.get("id");
        let strategie: String = r.get::<String, _>("strategie").to_lowercase();
        let asset: String = r.get("asset");
        let tf: String = r.get("timeframe");
        let cree_le: i64 = r.get("cree_le");

        // Bougies du TF autours de l'émission (extraire_features en veut ≥60).
        // Borné à l'âge du signal : un signal de 3 jours n'a pas besoin de
        // 400 jours d'historique (~576k lignes en M1 pour rien).
        let jours = (((maintenant - cree_le) / 86_400) + 3).clamp(3, 400) as u32;
        let asset_parsé = common::Asset::from(asset.as_str());
        let Ok(tf_parsé) = common::Timeframe::try_from(tf.as_str()) else { continue };
        let Ok(bougies) = db
            .obtenir_bougies_depuis_jours(&asset_parsé, &tf_parsé, jours)
            .await
        else { continue };
        // Bornes : bougies closes AVANT l'émission (le contexte du setup).
        let contexte: Vec<common::Candle> = bougies
            .iter()
            .filter(|b| b.timestamp.timestamp() < cree_le + 60)
            .cloned()
            .collect();
        let Some(features_ohlcv) = ml::extraire_features(&contexte) else { continue };

        let nom = strategie.as_str();
        let features: Vec<f64> = if nom.contains("smc") {
            // 7 contextuelles à 0 (non reconstituables — cf. doc) : le
            // vecteur reste dimensionnellement identique (59).
            db::smc_features::construire_features_59(
                &features_ohlcv,
                &db::smc_features::ContexteSmc {
                    tendance: 0.0, order_block: 0.0, ifvg: 0.0, fibonacci: 0.0,
                    imbalance: 0.0, kill_zone_active: false, sweep_detecte: false,
                },
            )
        } else if nom.contains("straddle") {
            db::straddle_features::construire_features_56(
                &features_ohlcv, 0.0, "inconnue", "inconnue", 0.0,
            )
        } else {
            features_ohlcv.clone()
        };

        let ok = if nom.contains("smc") {
            db::smc_features::inserer_snapshot(db.pool(), &id, &asset, &features).await.is_ok()
        } else if nom.contains("straddle") {
            db::straddle_features::inserer_snapshot(db.pool(), &id, &asset, &features)
                .await
                .is_ok()
        } else {
            db::rockets_features::inserer_snapshot(db.pool(), 0, &asset, &features)
                .await
                .is_ok()
        };
        if ok {
            écrits += 1;
        }
    }
    if écrits > 0 {
        tracing::info!("🧠 ML v2 : features backfill — {écrits} snapshots reconstitués (OHLCV historiques)");
    }
    écrits
}
