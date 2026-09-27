//! Whale watching (2.2) : détection de volume anormal par asset × TF.
//!
//! Principe : les gros acteurs institutionnels laissent des traces dans le
//! volume. Un z-score > 2σ signifie que la bougie courante a un volume
//! significativement au-dessus de la normale → probable activité
//! institutionnelle. Le moteur SMC s'en sert pour ENRICHIR le score
//! (+1 si >2σ, +2 si >3σ) — jamais pour filtrer.
//!
//! Sources : Bybit WS (volume réel crypto) + MT5 (tick volume forex/
//! métaux/indices — proxy du volume interbancaire, corrélation > 0,9).

use std::collections::HashMap;
use std::sync::Arc;

use common::{Asset, Candle};
use db::Database;

/// Cache des statistiques volume par asset × TF (moyenne + σ sur 60 bougies).
/// Recalculé à chaque appel — les 60 bougies M5 représentent 5 h de données,
/// la fenêtre est courte par design (le whale watching doit réagir vite).
pub struct WhaleWatching {
    /// (asset, tf) → (moyenne_volume, écart_type)
    stats: HashMap<(String, String), (f64, f64)>,
}

impl WhaleWatching {
    pub fn new() -> Self {
        Self { stats: HashMap::new() }
    }

    /// Recalcule les stats volume pour un couple asset × TF.
    /// À appeler périodiquement (runtime tick, 60 s) — pas par signal.
    pub async fn rafraichir(&mut self, db: &Arc<Database>, asset: &Asset, tf: &common::Timeframe) {
        let Ok(bougies) = db.obtenir_bougies(asset, tf, 60).await else {
            return;
        };
        let volumes: Vec<f64> = bougies.iter().map(|b| b.volume).filter(|v| *v > 0.0).collect();
        if volumes.len() < 20 {
            return; // pas assez de données pour une stat fiable
        }
        let n = volumes.len() as f64;
        let moy = volumes.iter().sum::<f64>() / n;
        let var = volumes.iter().map(|v| (v - moy) * (v - moy)).sum::<f64>() / n;
        let sigma = var.sqrt();
        if sigma > 0.0 {
            self.stats.insert((asset.as_str().to_string(), tf.as_str().to_string()), (moy, sigma));
        }
    }

    /// Z-score du volume de la dernière bougie d'un couple asset × TF.
    /// Retourne None si les stats ne sont pas encore calculées.
    pub fn zscore_derniere_bougie(&self, db_bougies: &[Candle], asset: &str, tf: &str) -> Option<f64> {
        let derniere = db_bougies.last()?;
        let (moy, sigma) = self.stats.get(&(asset.to_string(), tf.to_string()))?;
        if *sigma <= 0.0 {
            return None;
        }
        Some((derniere.volume - moy) / sigma)
    }

    /// Bonus de score whale : +1 si z > 2σ, +2 si z > 3σ, 0 sinon.
    /// Ne FILTRE jamais — enrichit seulement.
    pub fn bonus_score(&self, zscore: Option<f64>) -> u32 {
        match zscore {
            Some(z) if z > 3.0 => 2,
            Some(z) if z > 2.0 => 1,
            _ => 0,
        }
    }

    /// Test : les stats sont-elles prêtes pour ce couple ?
    pub fn est_pret(&self, asset: &str, tf: &str) -> bool {
        self.stats.contains_key(&(asset.to_string(), tf.to_string()))
    }
}

impl Default for WhaleWatching {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bonus_score_nul_sans_donnees() {
        let ww = WhaleWatching::new();
        assert_eq!(ww.bonus_score(None), 0);
    }

    #[test]
    fn bonus_1_sigma_2() {
        let ww = WhaleWatching::new();
        assert_eq!(ww.bonus_score(Some(2.5)), 1);
    }

    #[test]
    fn bonus_2_sigma_3() {
        let ww = WhaleWatching::new();
        assert_eq!(ww.bonus_score(Some(3.2)), 2);
    }

    #[test]
    fn bonus_0_sous_2_sigma() {
        let ww = WhaleWatching::new();
        assert_eq!(ww.bonus_score(Some(1.8)), 0);
        assert_eq!(ww.bonus_score(Some(0.0)), 0);
    }

    #[test]
    fn pas_de_stats_au_demarrage() {
        let ww = WhaleWatching::new();
        assert!(!ww.est_pret("BTC", "M5"));
    }
}

/// GET /api/whale — z-scores volume actuels par asset × TF (pour le
/// Scanner : badge 🐋 sur les setups avec volume anormal).
pub async fn get_whale_scores(
    state: actix_web::web::Data<crate::state::AppState>,
) -> impl actix_web::Responder {
    let ww = state.whale.read().await;
    let mut scores: Vec<serde_json::Value> = Vec::new();
    for ((asset, tf), (moy, sigma)) in &ww.stats {
        // Récupérer la dernière bougie pour calculer le z-score actuel
        let asset_enum = common::Asset::nouveau(asset);
        let tf_enum = match common::Timeframe::try_from(tf.as_str()) {
            Ok(t) => t,
            Err(_) => continue,
        };
        let Ok(bougies) = state.db.obtenir_bougies(&asset_enum, &tf_enum, 1).await else {
            continue;
        };
        let Some(derniere) = bougies.last() else { continue; };
        let z = if *sigma > 0.0 { (derniere.volume - moy) / sigma } else { 0.0 };
        scores.push(serde_json::json!({
            "asset": asset,
            "tf": tf,
            "zscore": (z * 10.0).round() / 10.0,
            "bonus": ww.bonus_score(Some(z)),
        }));
    }
    actix_web::HttpResponse::Ok().json(scores)
}

/// Boucle de fond : rafraîchit les stats volume toutes les 60 s pour les
/// couples armés au runtime (asset × TF qui ont des moteurs SMC).
pub fn demarrer_boucle_whale(db: std::sync::Arc<db::Database>, whale: std::sync::Arc<tokio::sync::RwLock<WhaleWatching>>, assets: Vec<(Asset, common::Timeframe)>) {
    tokio::spawn(async move {
        // Laisser le boot se poser avant d'occuper la DB.
        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
        loop {
            let mut ww = whale.write().await;
            for (asset, tf) in &assets {
                ww.rafraichir(&db, asset, tf).await;
            }
            drop(ww);
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
        }
    });
}
