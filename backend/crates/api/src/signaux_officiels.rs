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
            let sf = crate::setups_formation::depuis_annonce(m.id, &s);
            crate::setups_formation::enregistrer_annonce(sf.clone());
            // Scanner SMC (11/09) — trace durable : le vivier mémoire est
            // purgé à 2 h ; le journal alimente les dissipés + l'étude §3.2.
            let _ = db::smc_setups_journal::upsert_annonce(
                db.pool(),
                &db::smc_setups_journal::SetupJournal {
                    cle: sf.cle.clone(),
                    strategie: sf.strategie.clone(),
                    asset: sf.asset.clone(),
                    tf: sf.tf.clone(),
                    direction: sf.direction.clone(),
                    force_max: sf.force as i64,
                    entree: sf.entree,
                    sl: sf.sl,
                    tps: serde_json::to_string(&sf.tps).unwrap_or_default(),
                    debut: sf.debut_barre,
                    annonce_le: sf.ts_annonce,
                    fin: None,
                    issue: None,
                    signal_id: None,
                },
            )
            .await;
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
        // Straddle : le moteur ouvre les DEUX jambes au même prix E (timer
        // T-10 s). Le signal porte la jambe LONG ; la jambe SHORT est
        // symétrique autour de E — dérivée en miroir pour l'insertion
        // complète (sl_short + TPs + heure d'entrée = ouverture).
        if s.direction == Direction::Both {
            let e = s.prix_entree;
            let sl_short = 2.0 * e - s.stop_loss;
            let tps_short: Vec<f64> = s.take_profits.iter().map(|tp| 2.0 * e - tp).collect();
            if let Err(err) = db
                .inserer_signal_straddle_complet(&signal, sl_short, &tps_short, Some(s.debut_barre), &s.cle)
                .await
            {
                tracing::warn!("Signaux officiels (insert straddle Both): {}", err);
                continue;
            }
        } else if let Err(e) = db.inserer_signal_officiel(&signal, &s.cle).await {
            tracing::warn!("Signaux officiels (insert): {}", e);
            continue;
        }
        // §8 (07/09) : journaliser l'instantané de qualification SMC —
        // lecture seule, idempotent, jamais dans le chemin de la décision.
        if let Some(detail) = &s.detail {
            let _ = db::smc_features::inserer_detail_qualification(
                db.pool(),
                &signal.id.to_string(),
                detail,
            )
            .await;
        }
        crate::setups_formation::marquer_confirme(m.id, &s.cle);
        // Scanner SMC (11/09) — clôture du cycle de vie : devenu signal.
        let _ = db::smc_setups_journal::clôturer(
            db.pool(),
            &s.cle,
            "signal",
            chrono::Utc::now().timestamp(),
            Some(&signal.id.to_string()),
        )
        .await;
        // §8-2 (07/09) : conviction IA à l'émission — ARRIÈRE-PLAN, jamais
        // dans le chemin du signal. L'analyste note (0-100 + raison), la
        // colonne « IA » des tableaux se remplit. Observation d'abord :
        // rien n'est filtré, la corrélation conviction × verdict ne viendra
        // qu'après ≥ 30 trades notés.
        if llm_conviction_deja_present(&db, &signal.id.to_string()).await {
            // déjà noté (re-émission du même signal) — on ne double pas
        } else {
            let db_c = db.clone();
            let signal_id_c = signal.id.to_string();
            let conviction_ctx = contexte_conviction(&s, m.id);
            tokio::spawn(async move {
                noter_conviction(db_c, signal_id_c, conviction_ctx).await;
            });
        }

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
            let _ = db.marquer_remplie_par_cle(&e.cle_trade, e.asset.as_str(), e.debut_barre).await;
            continue;
        }
        if !matches!(e.evenement, T::Cloture) {
            continue;
        }
        let verdict = e.detail.split('|').next().unwrap_or("Expire");
        let r = e.detail.split('|').nth(1).and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
        if let Err(err) = db
            .fermer_signal_par_cle(&e.cle_trade, e.asset.as_str(), verdict, e.prix, r, e.emis_le.timestamp())
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
    // Capital SIMULÉ courant de la stratégie (composé à chaque clôture) —
    // le lot mise sur le capital mis à jour par les trades précédents.
    let capital_lot = crate::capital_simule::capital_actuel(db, id_strategie)
        .await
        .unwrap_or(reg.capital);
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
    let risque_euros = capital_lot * reg.risque_pct / 100.0;
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
        let risque_euros = capital_lot * params.profil.fraction();
        let dist = (entree - sl).abs();
        let mut qty = if dist > 0.0 { risque_euros / dist } else { 0.0 };
        let plafond = capital_lot * params.plafond_position_pct / 100.0;
        if entree > 0.0 {
            qty = qty.min(plafond / entree);
        }
        let alpha = s.score >= 9;
        let msg = format!(
            "{icone} {nom}\n{symbole} — classement {points}/10{alpha}\nLot = {qty:.2} ({risque_euros:.0}$ risqués — {profil})\n\nOrdre stop-limit : achat au-delà de {entree:.4}$ (plafond {limite:.4}$)\nInvalidation : {sl:.4}$ (−{pct_stop:.1} %)\nAu R1 ({r1:.4}$) : vendre 50 % + trailing {trail:.0} %",
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
    // RÈGLE 26/08 : le TIMER ouvre les 2 jambes au prix E à T-10 s — le
    // message décrit l'ouverture straddle complète (niveaux miroir ±1R/±2R).
    if id_strategie == "straddle" {
        let annonce = titre_annonce_straddle(db, &s.cle).await;
        let heure = chrono::DateTime::from_timestamp(s.debut_barre, 0)
            .map(|d| d.format("%H:%M:%S").to_string())
            .unwrap_or_default();
        let trailing = db::strategies_params::lire_straddle_params(db.pool())
            .await
            .trailing_r;
        if s.direction == Direction::Both {
            let e = s.prix_entree;
            let r = (e - sl).abs();
            let msg = format!(
                "{icone} {nom}\nPasse sur {asset} — {annonce}\n2 jambes ouvertes à {e:.2}$ à {heure} (timer T-10 s)\nLot = {lot:.2} par jambe ({risque_euros:.0}$ risqués)\n\nLONG  : SL {sl_long:.2}$ | TP1 {tp1l:.2} → SL E−0,5R | TP2 {tp2l:.2} → trailing {trailing:.1}R\nSHORT : SL {sl_short:.2} | TP1 {tp1s:.2} → SL E+0,5R | TP2 {tp2s:.2} → trailing {trailing:.1}R\nTime-stop : 60 min — R net = somme des 2 jambes",
                icone = crate::registre_strategies::MANIFESTES
                    .iter()
                    .find(|m| m.id == id_strategie)
                    .map(|m| m.icone)
                    .unwrap_or("▪️"),
                nom = id_strategie,
                asset = asset,
                annonce = annonce,
                e = e,
                heure = heure,
                lot = lot,
                risque_euros = risque_euros,
                sl_long = e - r,
                tp1l = e + r,
                tp2l = e + 2.0 * r,
                sl_short = e + r,
                tp1s = e - r,
                tp2s = e - 2.0 * r,
                trailing = trailing,
            );
            return Some(msg);
        }
        let msg = format!(
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
    // Formats : « straddle-{asset}-{ts}-B » (27/08+) ou « straddle-{ts}-L/S » (ancien).
    let Some(ts) = cle
        .split('-')
        .filter_map(|m| m.parse::<i64>().ok())
        .next()
    else {
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

// ── §8-2 : conviction IA à l'émission (07/09) ────────────────────────────────
// L'analyste note chaque signal officiel en arrière-plan (0-100 + raison,
// JSON) — la colonne « IA » des tableaux. OBSERVATION D'ABORD : jamais de
// filtrage, la corrélation conviction × verdict attendra ≥ 30 trades notés
// (§8, décision sur preuve).

/// Le signal est-il déjà noté ? (re-émissions : on ne double pas la note)
async fn llm_conviction_deja_present(db: &Arc<Database>, signal_id: &str) -> bool {
    sqlx::query_scalar::<_, Option<i64>>(
        "SELECT llm_conviction FROM signaux WHERE id = ?",
    )
    .bind(signal_id)
    .fetch_one(db.pool())
    .await
    .ok()
    .flatten()
    .is_some()
}

/// Contexte compact de conviction : identité + niveaux + le DÉTAIL de
/// qualification quand il existe (les deux chantiers §8 se nourrissent).
fn contexte_conviction(s: &engine::SignalBrut, strategie: &str) -> String {
    let mut l = vec![
        format!("Stratégie : {} (moteur {})", strategie, s.moteur),
        format!("Signal : {} {} {}", s.asset.as_str(), s.tf.as_str(),
                if matches!(s.direction, common::Direction::Long) { "LONG" } else { "SHORT" }),
        format!("Entrée {:.4} · SL {:.4} · TP1 {:.4} · score {}/10", s.prix_entree, s.stop_loss,
                s.take_profits.first().copied().unwrap_or(s.prix_entree), s.score.clamp(1, 10)),
        format!("Raison moteur : {}", s.raison),
    ];
    if let Some(d) = &s.detail {
        l.push(format!("Qualification : {}", d));
    }
    l.join("\n")
}

/// Interroge l'analyste puis écrit la note. Silencieux sur échec (l'IA
/// absente ne paralyse rien — l'émission est déjà faite).
async fn noter_conviction(db: Arc<Database>, signal_id: String, contexte: String) {
    let prompt = format!("{}\n\n{contexte}", llm::prompt_effectif("conviction_signal"));
    let Ok(texte) = llm::ollama::interroger(&prompt).await else {
        tracing::info!("🧠 Conviction IA : analyste indisponible — signal non noté");
        return;
    };
    // Parse JSON {conviction, raison} (repli : premier entier du texte).
    let (conviction, raison) = parser_conviction(&texte);
    if conviction < 0 {
        return; // pas parsable — on ne note pas n'importe quoi
    }
    let _ = sqlx::query(
        "UPDATE signaux SET llm_conviction = ?, llm_raison = ? WHERE id = ? AND llm_conviction IS NULL",
    )
    .bind(conviction)
    .bind(&raison)
    .bind(&signal_id)
    .execute(db.pool())
    .await;
    tracing::info!("🧠 Conviction IA notée : signal {} → {}/100", signal_id, conviction);
}

/// Extrait (conviction, raison) d'une réponse LLM.
fn parser_conviction(texte: &str) -> (i32, String) {
    let debut = texte.find('{').unwrap_or(0);
    let fin = texte.rfind('}').map(|i| i + 1).unwrap_or(texte.len());
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&texte[debut..fin]) {
        let c = v.get("conviction").and_then(|x| x.as_i64()).unwrap_or(-1) as i32;
        let r = v.get("raison").and_then(|x| x.as_str()).unwrap_or("").to_string();
        if (0..=100).contains(&c) {
            return (c, r);
        }
    }
    (-1, String::new())
}
