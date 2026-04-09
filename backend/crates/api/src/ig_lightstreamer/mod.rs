//! Client Lightstreamer TLCP pour IG Markets.
//!
//! Flux : REST login → create_session LS → subscribe CHART:{epic}:{resolution}:1
//! → bind_session (stream HTTP infini ligne par ligne) → parse TLCP → Candle → DB + broadcast WS.
//!
//! Résolutions : 1MINUTE 5MINUTE 15MINUTE 30MINUTE HOUR 4HOUR DAY WEEK

use anyhow::{anyhow, Result};
use common::{Asset, Candle, Timeframe};
use data::providers::ig as ig_helpers;
use db::Database;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};

use crate::ig_session::IgSession;

mod connection;
mod protocol;

// ─── Événement de bougie diffusé vers les WebSocket clients ──────────────────

#[derive(Debug, Clone)]
pub struct LsCandle {
    pub asset:     Asset,
    pub timeframe: Timeframe,
    pub candle:    Candle,
    /// true = bougie clôturée (persister), false = bougie en cours (mise à jour live)
    pub closed:    bool,
}

// ─── Clé de souscription ──────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SubKey {
    pub epic:       String,
    pub resolution: String,
    pub asset:      Asset,
    pub timeframe:  Timeframe,
}

// ─── Client principal ─────────────────────────────────────────────────────────

pub struct IgLightstreamer {
    ig_session:  Arc<Mutex<IgSession>>,
    db:          Arc<Database>,
    tx:          broadcast::Sender<LsCandle>,
    subs:        Arc<Mutex<HashMap<SubKey, usize>>>,
    next_sub_id: Arc<Mutex<usize>>,
    ls_session:  Arc<Mutex<Option<connection::LsSession>>>,
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
            subs:        Arc::new(Mutex::new(HashMap::new())),
            next_sub_id: Arc::new(Mutex::new(1)),
            ls_session:  Arc::new(Mutex::new(None)),
        };
        (client, rx)
    }

    /// Démarre la boucle principale de connexion/reconnexion.
    /// À appeler une seule fois dans AppState::new() via tokio::spawn.
    pub async fn run(self: Arc<Self>) {
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
            tracing::info!("IG LS: abonné CHART:{}:{}:1 (sub_id={})", epic, resolution, sub_id);
        }

        Ok(())
    }

    /// Désabonne l'asset+timeframe.
    pub async fn unsubscribe(&self, asset: &Asset, timeframe: Timeframe) -> Result<()> {
        let epic = match ig_helpers::epic_pour_asset(asset) {
            Some(e) => e.to_string(),
            None    => return Ok(()),
        };
        let resolution = protocol::resolution_ls(&timeframe).to_string();
        let key = SubKey { epic, resolution, asset: asset.clone(), timeframe };

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
}
