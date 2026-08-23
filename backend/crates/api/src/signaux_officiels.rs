//! Étape 2 — signaux OFFICIELS : writer Telegram aux maquettes propriétaire
//! (message d'IMMINENCE seul — pas de clôture/fill/TP) + table `signaux`.
//!
//! Le writer consulte le REGISTRE : seul une stratégie « Officielle » avec
//! son « son » activé parle sur Telegram (découplé de la vie des signaux).
//! Le lot se calcule par stratégie : (capital × risque) / (stop en pips ×
//! valeur du pip) — conventions de l'onglet gestion du risque.

use std::sync::Arc;

use common::{Direction, Signal};
use db::Database;
use engine::{BusEvenements, BusSignaux};

/// Démarre le writer (spawn).
pub fn demarrer(db: Arc<Database>, bus_signaux: BusSignaux, bus_evenements: BusEvenements) {
    tokio::spawn(ecrire_signaux(db.clone(), bus_signaux));
    tokio::spawn(fermer_signaux(db, bus_evenements));
}

async fn ecrire_signaux(db: Arc<Database>, bus: BusSignaux) {
    let mut rx = bus.abonner();
    tracing::info!("📢 Signaux OFFICIELS actifs (table signaux + Telegram)");
    while let Ok(s) = rx.recv().await {
        // Manifeste connu ? (moteur → stratégie)
        let Some(m) = crate::registre_strategies::MANIFESTES
            .iter()
            .find(|m| m.moteur == s.moteur)
        else {
            continue;
        };
        // Table : seule une stratégie Officielle écrit l'historique officiel.
        let etat = db
            .lire_strategie(m.id)
            .await
            .ok()
            .flatten()
            .map(|r| r.etat)
            .unwrap_or_else(|| "Construction".into());
        if etat != "Officielle" {
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
            m.id,
        );
        if let Err(e) = db.inserer_signal_officiel(&signal, &s.cle).await {
            tracing::warn!("Signaux officiels (insert): {}", e);
        }
        // Telegram : son activé ?
        let reg = db.lire_strategie(m.id).await.ok().flatten();
        if reg.as_ref().is_some_and(|r| r.notifications) {
            if let Some(msg) = formater_message(&db, m.id, &s).await {
                envoyer_telegram(&db, &msg).await;
            }
        }
    }
}

/// Clôtures : mise à jour DB silencieuse (statut Fermé) — pas de message
/// (décision propriétaire : imminence seule sur Telegram).
async fn fermer_signaux(db: Arc<Database>, bus: BusEvenements) {
    let mut rx = bus.abonner();
    while let Ok(e) = rx.recv().await {
        use engine::TypeEvenementTrade as T;
        if !matches!(e.evenement, T::Cloture) {
            continue;
        }
        if let Err(err) = db.fermer_signal_par_cle(&e.cle_trade, e.emis_le.timestamp()).await {
            tracing::warn!("Signaux officiels (clôture): {}", err);
        }
    }
}

/// Message d'imminence (maquette propriétaire) + lot par stratégie.
async fn formater_message(
    db: &Database,
    id_strategie: &str,
    s: &engine::types::SignalBrut,
) -> Option<String> {
    let reg = db.lire_strategie(id_strategie).await.ok()??;
    let dir = match s.direction {
        Direction::Long => "🟢 ACHAT",
        Direction::Short => "🔴 VENTE",
        _ => "⚪",
    };
    let asset = s.asset.as_str().to_string();
    let tf = s.tf.as_str().to_string();
    let entree = s.prix_entree;
    let sl = s.stop_loss;
    let tps: Vec<f64> = s.take_profits.clone();

    // Conventions de pips de l'actif (onglet gestion du risque).
    let (taille_pip, valeur_pip) = db::asset_params::lire_un(db.pool(), &asset)
        .await
        .ok()
        .flatten()
        .map(|p| (p.taille_pip, p.valeur_pips))
        .unwrap_or((1.0, 1.0));
    let pips = |a: f64, b: f64| ((a - b).abs() / taille_pip).round() as i64;

    // Lot = (capital × risque) / (stop en pips × valeur du pip).
    let stop_pips = pips(entree, sl);
    let lot = if stop_pips > 0 && valeur_pip > 0.0 {
        (reg.capital * reg.risque_pct / 100.0) / (stop_pips as f64 * valeur_pip)
    } else {
        0.0
    };

    let mut msg = format!(
        "{icone} {nom}\nSetup {dir} en formation sur {asset} en {tf}\nForce {force}/10\nLot = {lot:.4} pour {risque:.0}% de risque\n\nEntrée : {entree:.2}$\nStop Loss : {sl:.2}$ (soit -{stop_pips} pips)",
        icone = crate::registre_strategies::MANIFESTES
            .iter()
            .find(|m| m.id == id_strategie)
            .map(|m| m.icone)
            .unwrap_or("▪️"),
        nom = id_strategie,
        dir = dir,
        asset = asset,
        tf = tf,
        force = s.score.clamp(1, 10),
        lot = lot,
        risque = reg.risque_pct,
        entree = entree,
        sl = sl,
        stop_pips = stop_pips,
    );
    for (i, tp) in tps.iter().take(3).enumerate() {
        msg.push_str(&format!(
            "\nTP{} : {:.2}$ (soit +{} pips)",
            i + 1,
            tp,
            pips(*tp, entree)
        ));
    }
    Some(msg)
}

/// Envoi Telegram direct — erreur = log simple, jamais bloquant.
async fn envoyer_telegram(db: &Database, texte: &str) {
    let (token, chat) = notifications::telegram::lire_tokens_pool(db.pool()).await;
    if token.is_empty() || chat.is_empty() {
        return;
    }
    if let Err(e) = notifications::telegram::post_message(&token, &chat, texte).await {
        tracing::warn!("Telegram: {}", e);
    }
}
