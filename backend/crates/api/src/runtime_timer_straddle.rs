//! §Timer straddle runtime (A, owner 14/10) : 1 Hz indépendant des prix.
//!
//! Quand une annonce straddle est imminente (T-30 s..T), le runtime
//! injecte chaque seconde un Tick au DERNIER prix connu dans le moteur —
//! le timer tire à T-3 s même si l'EA n'a poussé aucune bougie pendant
//! la plate-forme pré-annonce. Les annonces sont rechargées toutes les
//! 60 s depuis la matrice (`fenetres_straddle`).

use std::sync::Arc;

use db::Database;
use engine::{EvenementPrix, PrixEvent, Runtime};

pub struct TimerStraddle {
    tick: tokio::time::Interval,
    annonces: Vec<(String, i64)>,
    dernier_chargement: i64,
}

/// Le tokio::select! a besoin d'un Future — on expose un sleep d'une
/// seconde plutôt que l'Interval (qui n'est pas un Future directement).
pub struct TickStraddle;

impl std::future::Future for TickStraddle {
    type Output = ();
    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let fut = Box::pin(tokio::time::sleep(std::time::Duration::from_secs(1)));
        let mut fut = fut;
        std::future::Future::poll(std::pin::Pin::new(&mut fut), cx)
    }
}

impl TimerStraddle {
    pub fn new() -> Self {
        let mut tick = tokio::time::interval(std::time::Duration::from_secs(1));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        Self { tick, annonces: Vec::new(), dernier_chargement: 0 }
    }

    /// À appeler à chaque tick (1 s) — injecte les prix si annonce imminente.
    pub async fn pulse(&mut self, db: &Arc<Database>, runtime: &mut Runtime) {
        let maintenant = chrono::Utc::now().timestamp();
        if maintenant - self.dernier_chargement > 60 {
            let perimetre = crate::runtime_perimetre::lire_perimetre_straddle(db).await;
            self.annonces = crate::runtime_params::fenetres_straddle(db, &perimetre)
                .await
                .into_iter()
                .flat_map(|(asset, ts_liste)| {
                    ts_liste.into_iter().map(move |ts| (asset.clone(), ts))
                })
                .collect();
            self.dernier_chargement = maintenant;
        }
        for (asset, annonce_ts) in &self.annonces {
            let reste = annonce_ts - maintenant;
            if reste <= 30 && reste >= -1 {
                if let Some(prix) = crate::mt5_collecteur::prix_live(asset) {
                    let ev = EvenementPrix {
                        asset: common::Asset::from(asset.as_str()),
                        tf: common::Timeframe::M1,
                        debut_bougie: maintenant - (maintenant % 60),
                        event: PrixEvent::Tick { prix, volume: None },
                        recu_le: chrono::Utc::now(),
                    };
                    runtime.traiter_evenement(ev);
                }
            }
        }
    }
}
