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
        // Table : seule une stratégie Officielle ou Observation écrit
        // l'historique (Observation = journalisé SANS Telegram — étape 4) ;
        // Construction n'écrit rien.
        let etat = db
            .lire_strategie(m.id)
            .await
            .ok()
            .flatten()
            .map(|r| r.etat)
            .unwrap_or_else(|| "Construction".into());
        if etat == "Construction" {
            continue;
        }
        let silencieuse = etat != "Officielle";
        // ANNONCE intrabar (setup qualifié, trade pas encore confirmé) :
        // enregistré pour le panneau « Setups en formation » de l'app,
        // message d'imminence sur Telegram (selon l'état) — pas de ligne en
        // base (elle viendra à la clôture si le trade confirme).
        if s.annonce {
            crate::setups_formation::enregistrer_annonce(
                crate::setups_formation::depuis_annonce(m.id, &s),
            );
            if !silencieuse {
                let reg = db.lire_strategie(m.id).await.ok().flatten();
                if reg.as_ref().is_some_and(|r| r.notifications) {
                    if let Some(msg) = formater_message(&db, m.id, &s).await {
                        envoyer_telegram(&db, &msg).await;
                    }
                }
            }
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
            continue;
        }
        crate::setups_formation::marquer_confirme(m.id, &s.cle);
        // Telegram : son activé ? L'annonce intrabar est déjà partie →
        // on marque la ligne sans re-messager. Observation = silencieux.
        let reg = db.lire_strategie(m.id).await.ok().flatten();
        let notifie = if !silencieuse && reg.as_ref().is_some_and(|r| r.notifications) {
            if s.deja_annonce {
                false
            } else if let Some(msg) = formater_message(&db, m.id, &s).await {
                envoyer_telegram(&db, &msg).await
            } else {
                false
            }
        } else {
            false
        };
        if notifie || s.deja_annonce {
            if let Err(e) = db.marquer_telegram_envoye(&signal.id.to_string()).await {
                tracing::warn!("Signaux officiels (drapeau Telegram): {}", e);
            }
        }
    }
}

/// Clôtures : mise à jour DB silencieuse (statut Fermé + verdict) — pas de
/// message (décision propriétaire : imminence seule sur Telegram).
/// Le détail moteur porte « verdict|R » (ex. « TP2|2.0000 »).
/// Les FILL marquent le remplissage : le trade existe au marché (les stats
/// ne comptent que les remplis).
async fn fermer_signaux(db: Arc<Database>, bus: BusEvenements) {
    let mut rx = bus.abonner();
    while let Ok(e) = rx.recv().await {
        use engine::TypeEvenementTrade as T;
        if matches!(e.evenement, T::Fill) {
            let _ = db.marquer_remplie_par_cle(&e.cle_trade, e.debut_barre).await;
            continue;
        }
        if !matches!(e.evenement, T::Cloture) {
            continue;
        }
        let verdict = e.detail.split('|').next().unwrap_or("Expire");
        let r = e.detail.split('|').nth(1).and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
        if let Err(err) = db
            .fermer_signal_par_cle(&e.cle_trade, verdict, e.prix, r, e.emis_le.timestamp())
            .await
        {
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
    // Distance EXACTE pour le lot (jamais d'arrondi intermédiaire — un stop
    // de 0,04 $ sur XAG ne doit pas s'arrondir à 0 pip → lot = 0) ; l'entier
    // `stop_pips` ne sert qu'à l'affichage.
    let stop_pips_exact = (entree - sl).abs() / taille_pip;
    let stop_pips = stop_pips_exact.round() as i64;
    let risque_euros = reg.capital * reg.risque_pct / 100.0;
    let lot = if stop_pips_exact > 0.0 && valeur_pip > 0.0 && taille_pip > 0.0 {
        risque_euros / (stop_pips_exact * valeur_pip)
    } else {
        0.0
    };

    // Template ROCKETS (maquette actée étape 2 : « stop-limit + invalidation »
    // chiffrés — enrichi du lot et du plan R1/trailing). Lot = capital de la
    // stratégie × profil de risque, plafonné à 5 % du capital en montant.
    if id_strategie == "rockets" {
        let params = crate::rockets_verticale::lire_params(db).await;
        let risque_euros = reg.capital * params.profil.fraction();
        let dist = (entree - sl).abs();
        let mut qty = if dist > 0.0 { risque_euros / dist } else { 0.0 };
        let plafond = reg.capital * params.plafond_position_pct / 100.0;
        if entree > 0.0 {
            qty = qty.min(plafond / entree);
        }
        let alpha = s.score >= 9;
        let msg = format!(
            "{icone} {nom}\n{symbole} — classement {points}/10{alpha}\nLot = {qty:.4} ({risque_euros:.0}$ risqués — {profil})\n\nOrdre stop-limit : achat au-delà de {entree:.4}$ (plafond {limite:.4}$)\nInvalidation : {sl:.4}$ (−{pct_stop:.1} %)\nAu R1 ({r1:.4}$) : vendre 50 % + trailing {trail:.0} %",
            icone = crate::registre_strategies::MANIFESTES.iter().find(|m| m.id == id_strategie).map(|m| m.icone).unwrap_or("▪️"),
            nom = id_strategie,
            symbole = asset,
            points = s.score.clamp(1, 10),
            alpha = if alpha { " — ROCKET ALPHA" } else { "" },
            qty = qty,
            risque_euros = risque_euros,
            profil = params.profil.libelle(),
            entree = entree,
            limite = entree * (1.0 + params.cassure_min_pct / 100.0),
            sl = sl,
            pct_stop = if entree > 0.0 { (entree - sl).abs() / entree * 100.0 } else { 0.0 },
            r1 = tps.first().copied().unwrap_or(entree),
            trail = params.trailing_pct,
        );
        return Some(msg);
    }

    // Template STRADDLE (maquette provisoire actée étape 2 : annonce +
    // setup + entrée horodatée + SL + trailing — enrichie du lot, 3 couches).
    // La jambe survivante porte le signal : E, SL = E∓1R, TP1/TP2 canoniques.
    if id_strategie == "straddle" {
        let annonce = titre_annonce_straddle(db, &s.cle).await;
        let heure = chrono::DateTime::from_timestamp(s.debut_barre, 0)
            .map(|d| d.format("%H:%M:%S").to_string())
            .unwrap_or_default();
        let trailing = db::strategies_params::lire_straddle_params(db.pool())
            .await
            .trailing_r;
        let mut msg = format!(
            "{icone} {nom}\nPasse sur {asset} — {annonce}\nJambe {dir} remplie à {entree:.2}$ à {heure}\nLot = {lot:.2} ({risque_euros:.0}$ risqués)\n\nStop Loss : {sl:.2}$ (soit -{stop_pips} pips)\nTP1 : {tp1:.2}$ → BE à l'entrée\nTP2 : {tp2:.2}$ → BE à TP1 + trailing {trailing:.1}R\nTime-stop : 60 min",
            icone = crate::registre_strategies::MANIFESTES
                .iter()
                .find(|m| m.id == id_strategie)
                .map(|m| m.icone)
                .unwrap_or("▪️"),
            nom = id_strategie,
            asset = asset,
            annonce = annonce,
            dir = dir,
            entree = entree,
            heure = heure,
            lot = lot,
            risque_euros = risque_euros,
            sl = sl,
            stop_pips = stop_pips,
            tp1 = tps.first().copied().unwrap_or(entree),
            tp2 = tps.get(1).copied().unwrap_or(entree),
            trailing = trailing,
        );
        return Some(msg);
    }

    let mut msg = format!(
        "{icone} {nom}\nSetup {dir} en formation sur {asset} en {tf}\nForce {force}/10\nLot = {lot:.2} ({risque_euros:.0}$ risqués)\n\nEntrée : {entree:.2}$\nStop Loss : {sl:.2}$ (soit -{stop_pips} pips)",
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
        risque_euros = risque_euros,
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
/// Retourne true si le message est parti (pour le drapeau en base).
/// Retrouve le libellé de l'annonce d'une passe straddle depuis sa clé
/// (« straddle-{ts}-L/S ») : correspondance dans le cache calendrier (High,
/// ±2 min de tolérance). Repli : « annonce US ».
async fn titre_annonce_straddle(db: &Database, cle: &str) -> String {
    let Some(ts) = cle.split('-').nth(1).and_then(|t| t.parse::<i64>().ok()) else {
        return "annonce US".into();
    };
    let Ok(rows) = db.lire_calendrier_cache(7 * 24 * 3600).await else {
        return "annonce US".into();
    };
    for r in &rows {
        if r.get("impact").and_then(|v| v.as_str()) != Some("High") {
            continue;
        }
        let Some(dh) = r.get("date_heure").and_then(|v| v.as_str()) else {
            continue;
        };
        if let Ok(t) = chrono::DateTime::parse_from_rfc3339(dh) {
            if (t.timestamp() - ts).abs() <= 120 {
                return r
                    .get("titre")
                    .and_then(|v| v.as_str())
                    .unwrap_or("annonce US")
                    .to_string();
            }
        }
    }
    "annonce US".into()
}

async fn envoyer_telegram(db: &Database, texte: &str) -> bool {
    let (token, chat) = notifications::telegram::lire_tokens_pool(db.pool()).await;
    if token.is_empty() || chat.is_empty() {
        return false;
    }
    match notifications::telegram::post_message(&token, &chat, texte).await {
        Ok(_) => true,
        Err(e) => {
            tracing::warn!("Telegram: {}", e);
            false
        }
    }
}
