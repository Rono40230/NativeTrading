//! Client Lightstreamer TLCP pour IG Markets.
#![allow(dead_code)]
//!
//! Flux : REST login → create_session LS → subscribe CHART:{epic}:{resolution}:1
//! → bind_session (stream HTTP infini ligne par ligne) → parse TLCP → Candle → DB + broadcast WS.
//!
//! Résolutions : 1MINUTE 5MINUTE 15MINUTE 30MINUTE HOUR 4HOUR DAY WEEK

use anyhow::{anyhow, Result};
use common::{Asset, Candle, Timeframe};
use data::providers::ig as ig_helpers;
use db::Database;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex, RwLock};

use crate::ig_session::IgSession;

mod connection;
mod protocol;
mod rest_ig;

// ─── Événement de bougie diffusé vers les WebSocket clients ──────────────────

#[derive(Debug, Clone)]
pub struct LsCandle {
    pub asset: Asset,
    pub timeframe: Timeframe,
    pub candle: Candle,
    /// true = bougie clôturée (persister), false = bougie en cours (mise à jour live)
    pub closed: bool,
}

// ─── Clé de souscription ──────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SubKey {
    pub epic: String,
    pub resolution: String,
    pub asset: Asset,
    pub timeframe: Timeframe,
}

// ─── Client principal ─────────────────────────────────────────────────────────

pub struct IgLightstreamer {
    ig_session: Arc<Mutex<IgSession>>,
    db: Arc<Database>,
    tx: broadcast::Sender<LsCandle>,
    subs: Arc<Mutex<HashMap<SubKey, usize>>>,
    next_sub_id: Arc<Mutex<usize>>,
    ls_session: Arc<Mutex<Option<connection::LsSession>>>,
    /// Assets dont /prices/{epic} a retourné 403 — plus jamais tentés jusqu'au redémarrage
    rest_403: Arc<RwLock<HashSet<String>>>,
}

impl IgLightstreamer {
    pub fn new(
        ig_session: Arc<Mutex<IgSession>>,
        db: Arc<Database>,
    ) -> (Self, broadcast::Receiver<LsCandle>) {
        let (tx, rx) = broadcast::channel(512);
        let client = Self {
            ig_session,
            db,
            tx,
            subs: Arc::new(Mutex::new(HashMap::new())),
            next_sub_id: Arc::new(Mutex::new(1)),
            ls_session: Arc::new(Mutex::new(None)),
            rest_403: Arc::new(RwLock::new(HashSet::new())),
        };
        (client, rx)
    }

    /// Démarre la boucle principale de connexion/reconnexion.
    /// À appeler une seule fois dans AppState::new() via tokio::spawn.
    pub async fn run(self: Arc<Self>) {
        /*
        loop {
            match connection::connect_and_bind(
                &self.ig_session,
                &self.db,
                &self.subs,
                &self.ls_session,
                &self.tx,
            )
            .await
            {
                Ok(()) => {
                    tracing::warn!("IG Lightstreamer: session terminée, reconnexion dans 5s");
                }
                Err(e) => {
                    tracing::warn!("IG Lightstreamer: erreur — {} — reconnexion dans 10s", e);
                    tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                }
            }
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        }
        */
        tracing::info!("Boucle LS run() désactivée de force (Error 71 bypass)");
    }

    /// Abonne l'asset+timeframe au stream Lightstreamer. Idempotent.
    pub async fn subscribe(&self, asset: &Asset, timeframe: Timeframe) -> Result<()> {
        let epic = ig_helpers::epic_pour_asset(asset)
            .ok_or_else(|| anyhow!("Pas d'epic IG pour {}", asset.as_str()))?
            .to_string();
        let resolution = protocol::resolution_ls(&timeframe).to_string();

        let key = SubKey {
            epic: epic.clone(),
            resolution: resolution.clone(),
            asset: asset.clone(),
            timeframe,
        };

        {
            let subs = self.subs.lock().await;
            if subs.contains_key(&key) {
                return Ok(());
            }
        }

        let sub_id = {
            let mut next = self.next_sub_id.lock().await;
            let id = *next;
            *next += 1;
            id
        };

        {
            let mut subs = self.subs.lock().await;
            subs.insert(key, sub_id);
        }

        let ls = self.ls_session.lock().await;
        if let Some(sess) = ls.as_ref() {
            protocol::send_subscribe(&sess.endpoint, &sess.session_id, &epic, &resolution, sub_id)
                .await?;
            tracing::info!(
                "IG LS: abonné CHART:{}:{}:1 (sub_id={})",
                epic,
                resolution,
                sub_id
            );
        }

        Ok(())
    }

    /// Désabonne l'asset+timeframe.
    pub async fn unsubscribe(&self, asset: &Asset, timeframe: Timeframe) -> Result<()> {
        let epic = match ig_helpers::epic_pour_asset(asset) {
            Some(e) => e.to_string(),
            None => return Ok(()),
        };
        let resolution = protocol::resolution_ls(&timeframe).to_string();
        let key = SubKey {
            epic,
            resolution,
            asset: asset.clone(),
            timeframe,
        };

        let sub_id = {
            let mut subs = self.subs.lock().await;
            subs.remove(&key)
        };

        if let Some(id) = sub_id {
            let ls = self.ls_session.lock().await;
            if let Some(sess) = ls.as_ref() {
                protocol::send_unsubscribe(&sess.endpoint, &sess.session_id, id)
                    .await
                    .unwrap_or_else(|e| tracing::warn!("LS unsubscribe: {}", e));
            }
        }

        Ok(())
    }

    /// Retourne un nouveau receiver du canal broadcast.
    pub fn subscribe_broadcast(&self) -> broadcast::Receiver<LsCandle> {
        self.tx.subscribe()
    }

    /// Charge l'historique IG via REST (200 bougies max) et le stocke en DB source='rest_ig'.
    /// Utilise le cache DB si des données récentes existent (< 2 périodes du TF).
    /// Retourne les bougies chart (jamais des données MT5).
    pub async fn fetch_historique(
        &self,
        asset: &Asset,
        timeframe: Timeframe,
        limite: i64,
    ) -> Vec<Candle> {
        // 1. Retourner le cache DB dès qu'on a des données (même anciennes)
        //    Évite tout appel REST inutile si la DB est déjà peuplée (MT5, binance, etc.)
        let nb_cache = self
            .db
            .obtenir_bougies(asset, &timeframe, limite)
            .await
            .map(|b| b.len())
            .unwrap_or(0);

        if nb_cache > 0 {
            return self
                .db
                .obtenir_bougies(asset, &timeframe, limite)
                .await
                .unwrap_or_default();
        }

        // 2. Cache vide — vérifier si /prices/{epic} a déjà retourné 403 pour cet asset
        let rest_key = format!("{:?}_{:?}", asset, timeframe);
        {
            let blocked = self.rest_403.read().await;
            if blocked.contains(&rest_key) {
                return vec![];
            }
        }

        // 3. Appel REST IG (une seule tentative — 403 bloque pour toute la session)
        let bougies_rest = self.fetch_rest_ig(asset, &timeframe, limite as usize).await;

        if !bougies_rest.is_empty() {
            let _ = self
                .db
                .inserer_bougies_avec_source(asset, &timeframe, &bougies_rest, "rest_ig")
                .await;
        }

        // 4. Retourner depuis DB
        self.db
            .obtenir_bougies(asset, &timeframe, limite)
            .await
            .unwrap_or_default()
    }

    /// Appel REST IG GET /prices/{epic}?resolution={res}&max=200
    pub async fn fetch_rest_ig(&self, asset: &Asset, timeframe: &Timeframe, max: usize) -> Vec<Candle> {
        rest_ig::fetch_rest_ig(&self.ig_session, &self.db, &self.rest_403, asset, timeframe, max).await
    }
}

// ─── Helpers locaux ────────────────────────────────────────────────────────────

fn timeframe_duree_secs(tf: &Timeframe) -> i64 {
    match tf {
        Timeframe::M1 => 60,
        Timeframe::M5 => 300,
        Timeframe::M15 => 900,
        Timeframe::M30 => 1800,
        Timeframe::H1 => 3600,
        Timeframe::H4 => 14400,
        Timeframe::D1 => 86400,
        Timeframe::W1 => 604800,
    }
}
