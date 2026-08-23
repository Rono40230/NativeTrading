//! Phase 2.8 — BASCULE OFFICIELLE : les émissions live du runtime v12
//! deviennent les signaux officiels de l'app.
//!
//! Branché aux bus du runtime (comme `journal_emissions`, qui reste actif
//! comme piste d'audit) :
//! - **Signal** → INSERT table `signaux` (strategie `SmcDirectional`,
//!   `cle_moteur` pour le matching) + notification Telegram ;
//! - **Événement lifecycle** (Fill/BE/TP/Clôture) → notification Telegram,
//!   et Clôture → `statut='Fermé'` sur la ligne correspondante.
//!
//! Telegram est envoyé EN DIRECT depuis ce writer (post_message, timeout
//! 10 s) — pas de file d'attente : l'objectif roadmap est « sur le bus,
//! < 1 s ». Une erreur d'envoi ne bloque jamais le flux (log + continue).

use std::sync::Arc;

use common::{Direction, Signal};
use db::Database;
use engine::{BusEvenements, BusSignaux};

/// Démarre le writer (spawn). `db` sert aux tokens Telegram.
pub fn demarrer(db: Arc<Database>, bus_signaux: BusSignaux, bus_evenements: BusEvenements) {
    tokio::spawn(ecrire_signaux(db.clone(), bus_signaux));
    tokio::spawn(traiter_evenements(db, bus_evenements));
}

async fn ecrire_signaux(db: Arc<Database>, bus: BusSignaux) {
    let mut rx = bus.abonner();
    tracing::info!("📢 Signaux OFFICIELS v12 actifs (table signaux + Telegram)");
    while let Ok(s) = rx.recv().await {
        if s.moteur != "smc_v12" {
            continue;
        }
        let signal = Signal::nouveau(
            s.asset.clone(),
            s.tf,
            s.direction,
            s.score as f64,
            s.prix_entree,
            s.stop_loss,
            s.take_profits.clone(),
            "SmcDirectional",
        );
        if let Err(e) = db.inserer_signal_officiel(&signal, &s.cle).await {
            tracing::warn!("Signaux officiels (insert): {}", e);
            continue;
        }
        let dir_txt = match s.direction {
            Direction::Long => "🟢 BUY",
            Direction::Short => "🔴 SELL",
            _ => "⚪",
        };
        envoyer_telegram(
            &db,
            &format!(
                "{} SMC v12 · {} {} · entrée {:.2} · SL {:.2} · TP1 {:.2} · score {}/10\n{}",
                dir_txt,
                s.asset.as_str(),
                s.tf.as_str(),
                s.prix_entree,
                s.stop_loss,
                s.take_profits.first().copied().unwrap_or(0.0),
                s.score.min(10).max(1),
                s.raison,
            ),
        )
        .await;
    }
}

async fn traiter_evenements(db: Arc<Database>, bus: BusEvenements) {
    let mut rx = bus.abonner();
    while let Ok(e) = rx.recv().await {
        if e.moteur != "smc_v12" {
            continue;
        }
        // Option A (décision propriétaire) : Telegram MINIMAL — signal +
        // clôture uniquement. Fill/BE/TP restent journalisés (audit +
        // lifecycle DB) mais ne notifient pas.
        use engine::TypeEvenementTrade as T;
        if !matches!(e.evenement, T::Cloture) {
            continue;
        }
        if let Err(err) = db.fermer_signal_par_cle(&e.cle_trade, e.emis_le.timestamp()).await {
            tracing::warn!("Signaux officiels (clôture): {}", err);
        }
        envoyer_telegram(
            &db,
            &format!(
                "🔒 CLÔTURE ({}) · {} {} @ {:.2}",
                e.detail,
                e.asset.as_str(),
                e.tf.as_str(),
                e.prix
            ),
        )
        .await;
    }
}

/// Envoi Telegram direct (tokens DB) — erreur = log simple, jamais bloquant.
async fn envoyer_telegram(db: &Database, texte: &str) {
    let (token, chat) = notifications::telegram::lire_tokens_pool(db.pool()).await;
    if token.is_empty() || chat.is_empty() {
        return;
    }
    if let Err(e) = notifications::telegram::post_message(&token, &chat, texte).await {
        tracing::warn!("Telegram: {}", e);
    }
}
