//! Replay post-armement + réconciliation silencieuse.
//!
//! À l'armement d'un couple (asset × TF), le runtime rejoue ~7 jours de
//! bougies stockées : les moteurs reconstruisent leur état ET rejouent le
//! cycle de vie des trades ouverts avant un éventuel arrêt de l'app. Les
//! événements Fill/Cloture du replay sont répercutés en base — sans
//! Telegram, sans nouveaux signaux (la clé stable open_ts fait la
//! correspondance).
//!
//! Historique : ce rattrapage n'existait que pour les couples Bybit — les
//! actifs MT5 (XAU/XAG v12, indices straddle) sautaient le bloc replay et
//! gardaient des lignes « Actif » zombies (SL/TP touchés pendant l'arrêt,
//! jamais rattrapés).

use std::sync::Arc;

use common::{Asset, Timeframe};
use db::Database;
use engine::Runtime;

/// Profondeur de replay adaptée au moteur v12 : ~7 jours d'historique par
/// TF (ATR14 stabilisé, pivots, PDH/PDL/PWH/PWL), plafonné.
pub(crate) fn barres_replay_v12(tf: Timeframe) -> i64 {
    (7 * 1440 / tf.minutes() as i64).clamp(60, 10_080)
}

/// Résultat d'un replay + réconciliation.
pub(crate) struct ResultatReplay {
    pub bougies: usize,
    pub signaux: usize,
    /// Clôtures appliquées en base (les remplissages ne sont pas comptés).
    pub ecritures_db: u64,
}

/// Rejoue la fenêtre d'historique en base sur les moteurs fraîchement
/// enregistrés, puis répercute silencieusement remplissages et clôtures
/// survenus pendant l'arrêt. None = lecture des bougies impossible.
pub(crate) async fn rejouer_et_reconcilier(
    db: &Arc<Database>,
    runtime: &mut Runtime,
    asset: &Asset,
    tf: Timeframe,
) -> Option<ResultatReplay> {
    let profondeur = barres_replay_v12(tf);
    let bougies = db.obtenir_bougies(asset, &tf, profondeur).await.ok()?;
    let historique = runtime.rejouer(asset.clone(), tf, &bougies);
    let mut ecritures_db = reconcilier_evenements(db, &historique.evenements).await;
    ecritures_db += reconcilier_proximite(db, asset.as_str(), tf, &historique.evenements).await;
    ecritures_db += reconstruire_cycles_vie(db, asset.as_str(), tf).await;
    Some(ResultatReplay {
        bougies: bougies.len(),
        signaux: historique.signaux.len(),
        ecritures_db,
    })
}

/// Réconciliation du replay : applique en base les remplissages (Fill) et
/// clôtures (Cloture) survenus pendant la fenêtre rejouée (trades ouverts
/// avant un arrêt). Silencieux — pas de Telegram, pas de nouveaux signaux.
/// Retourne le nombre de clôtures appliquées.
async fn reconcilier_evenements(
    db: &Arc<Database>,
    evenements: &[engine::EvenementTrade],
) -> u64 {
    use engine::TypeEvenementTrade as T;
    let mut total = 0u64;
    for e in evenements {
        match e.evenement {
            // Remplissage rattrapé : l'ordre en attente a été touché pendant
            // l'arrêt — marque la ligne pour que les stats « remplis »
            // restent exactes (idempotent : WHERE heure_entree IS NULL).
            T::Fill => {
                if let Err(err) = db
                    .marquer_remplie_par_cle(&e.cle_trade, e.asset.as_str(), e.debut_barre)
                    .await
                {
                    tracing::warn!("Réconciliation replay ({}): {}", e.cle_trade, err);
                }
            }
            T::Cloture => {
                let verdict = e.detail.split('|').next().unwrap_or("Expire");
                let r = e
                    .detail
                    .split('|')
                    .nth(1)
                    .and_then(|s| s.parse::<f64>().ok())
                    .unwrap_or(0.0);
                match db
                    .fermer_signal_par_cle(
                        &e.cle_trade,
                        e.asset.as_str(),
                        verdict,
                        e.prix,
                        r,
                        e.emis_le.timestamp(),
                    )
                    .await
                {
                    Ok(n) => total += n,
                    Err(err) => tracing::warn!("Réconciliation replay ({}): {}", e.cle_trade, err),
                }
            }
            _ => {}
        }
    }
    total
}

/// Décode une clé v12 « open_ts:side:source:entry_bits » (side: 0=Buy,
/// 1=Sell ; source: 0=Ob, 1=BsZones). None pour les autres formats.
fn decoder_cle_v12(cle: &str) -> Option<(i64, bool, u8, f64)> {
    let mut m = cle.split(':');
    let open_ts = m.next()?.parse::<i64>().ok()?;
    let cote = match m.next()?.parse::<u8>().ok()? {
        0 => false,
        1 => true,
        _ => return None,
    };
    let source = m.next()?.parse::<u8>().ok()?;
    let bits = m.next()?.parse::<u64>().ok()?;
    if m.next().is_some() {
        return None;
    }
    Some((open_ts, cote, source, f64::from_bits(bits)))
}

/// Écart max entre la naissance du trade rejoué et la ligne Active
/// rapprochée : la genèse d'un replay peut glisser de quelques barres
/// (warm-up), jamais de plusieurs heures.
const DERIVE_NAISSANCE_MAX_SEC: i64 = 6 * 3600;

/// Rattrapage de proximité : la clé v12 encode side + entry (bits), mais
/// l'open_ts peut DÉRIVER entre le live et un replay fraîchement réchauffé
/// (genèse de zone différente → même trade né quelques barres plus tôt,
/// clé différente → clôture perdue par le passage par clé exacte). On
/// rapproche ici les événements Fill/Cloture des lignes Actives SMC du
/// couple par (direction, entrée identique, naissance à ±6 h) — verdict et
/// R restent ceux de l'événement moteur (sémantique du writer officiel).
async fn reconcilier_proximite(
    db: &Arc<Database>,
    asset: &str,
    tf: Timeframe,
    evenements: &[engine::EvenementTrade],
) -> u64 {
    use engine::TypeEvenementTrade as T;
    let actifs: Vec<db::signaux::SignalActifCle> =
        match db::signaux::lister_actifs_avec_cle(db.pool()).await {
            Ok(a) => a
                .into_iter()
                .filter(|s| {
                    s.asset == asset && s.timeframe == tf.as_str() && s.strategie == "SMC"
                })
                .collect(),
            Err(err) => {
                tracing::warn!("Réconciliation proximité (lister actifs): {}", err);
                return 0;
            }
        };
    if actifs.is_empty() {
        return 0;
    }
    let mut total = 0u64;
    for e in evenements {
        if !matches!(e.evenement, T::Fill | T::Cloture) {
            continue;
        }
        let Some((open_ts, cote, _, entree)) = decoder_cle_v12(&e.cle_trade) else {
            continue;
        };
        let direction = if cote { "Short" } else { "Long" };
        for a in &actifs {
            if a.direction != direction
                || (entree - a.prix_entree).abs() > 1e-9 * entree.abs().max(1.0)
                || (open_ts - a.cree_le).abs() > DERIVE_NAISSANCE_MAX_SEC
            {
                continue;
            }
            match e.evenement {
                T::Fill => {
                    if let Err(err) = db
                        .marquer_remplie_par_cle(&a.cle_moteur, asset, e.debut_barre)
                        .await
                    {
                        tracing::warn!("Réconciliation proximité ({}): {}", a.cle_moteur, err);
                    }
                }
                T::Cloture => {
                    let verdict = e.detail.split('|').next().unwrap_or("Expire");
                    let r = e
                        .detail
                        .split('|')
                        .nth(1)
                        .and_then(|s| s.parse::<f64>().ok())
                        .unwrap_or(0.0);
                    match db
                        .fermer_signal_par_cle(
                            &a.cle_moteur,
                            asset,
                            verdict,
                            e.prix,
                            r,
                            e.emis_le.timestamp(),
                        )
                        .await
                    {
                        Ok(n) if n > 0 => {
                            tracing::info!(
                                "Réconciliation proximité : {} {} fermé « {} » ({:.2}R) — genèse replay dérivée, clé exacte muette",
                                asset,
                                tf.as_str(),
                                verdict,
                                r
                            );
                            total += n;
                        }
                        Ok(_) => {}
                        Err(err) => {
                            tracing::warn!("Réconciliation proximité ({}): {}", a.cle_moteur, err)
                        }
                    }
                }
                _ => {}
            }
        }
    }
    total
}

/// Dernier filet — cycle de vie SQL : pour les lignes Actives SMC qu'aucun
/// replay ne régénère (genèse de zone irréproduisible après plusieurs jours
/// d'arrêt), on rejoue la vraie machine `TradeLifecycle` sur LES niveaux de
/// la ligne, depuis les bougies stockées. Parité moteur par construction :
/// mêmes précédences (SL → BE → TP2-SL → TP3 → expire), même BE forcé
/// supprimé que la production, même R que `Trade::realized_r`.
async fn reconstruire_cycles_vie(db: &Arc<Database>, asset: &str, tf: Timeframe) -> u64 {
    use smc::v12::calibration::AssetCalibration;
    use smc::v12::durees::{tp3_max_mins, trade_max_mins};
    use smc::v12::lifecycle::{ModeBeForce, TradeLifecycle};
    use smc::v12::scoring_v11::ScoringV11;
    use smc::v12::trade::{CloseReason, Side, Trade, TradeSource, TradeState, Verdict};
    use smc::v12::types::{BarInput, SmcOutput};

    let actifs: Vec<db::signaux::SignalActifCle> =
        match db::signaux::lister_actifs_avec_cle(db.pool()).await {
            Ok(a) => a
                .into_iter()
                .filter(|s| {
                    s.asset == asset && s.timeframe == tf.as_str() && s.strategie == "SMC"
                })
                .collect(),
            Err(_) => return 0,
        };
    if actifs.is_empty() {
        return 0;
    }

    let tf_mins = tf.minutes() as u32;
    let cal = AssetCalibration::detect(asset, tf.as_str());
    let mut lifecycle = TradeLifecycle::new(
        trade_max_mins(tf_mins) * 60,
        tp3_max_mins(&cal, tf_mins) * 60,
    );
    // Production (runtime_tick) : BE forcé sur BOS opposé supprimé.
    lifecycle.definir_mode_be_force(ModeBeForce::Supprime);
    let mut scoring = ScoringV11::new(&cal, tf_mins);
    let out_vide = SmcOutput::default();

    let maintenant = chrono::Utc::now().timestamp();
    let plus_vieux = actifs
        .iter()
        .map(|a| decoder_cle_v12(&a.cle_moteur).map(|c| c.0).unwrap_or(a.cree_le))
        .min()
        .unwrap_or(maintenant);
    let tf_secs = (tf_mins as i64) * 60;
    let limite = ((maintenant - plus_vieux) / tf_secs + 64).max(10);
    let actif = match Asset::try_from(asset) {
        Ok(a) => a,
        Err(_) => return 0,
    };
    let bougies = match db.obtenir_bougies(&actif, &tf, limite).await {
        Ok(b) => b,
        Err(_) => return 0,
    };

    let mut total = 0u64;
    for a in &actifs {
        let Some((open_ts, cote, source, _)) = decoder_cle_v12(&a.cle_moteur) else {
            continue;
        };
        let risk0 = (a.prix_entree - a.stop_loss).abs();
        if risk0 <= 0.0 {
            continue;
        }
        let mut carnet = [Trade {
            id: 0,
            side: if cote { Side::Sell } else { Side::Buy },
            source: if source == 1 {
                TradeSource::BsZones
            } else {
                TradeSource::Ob
            },
            entry: a.prix_entree,
            sl: a.stop_loss,
            tp1: a.take_profit.first().copied().unwrap_or(a.prix_entree),
            tp2: a.take_profit.get(1).copied().unwrap_or(a.prix_entree),
            tp3: a.take_profit.get(2).copied().unwrap_or(a.prix_entree),
            score: 0,
            risk0,
            open_ts,
            bar_created: 0,
            ob_key: None,
            filled: false,
            tp1_hit: false,
            tp1_price_touched: false,
            tp2_ts: 0,
            tp3_touched: false,
            be_forced: false,
            mfe_armed: false,
            state: TradeState::Pending,
            fill_ts: None,
            close_reason: None,
            close_ts: None,
            close_bar: None,
            close_r: None,
        }];
        let mut dernier_close = a.prix_entree;
        for (i, b) in bougies.iter().enumerate() {
            if b.timestamp.timestamp() < open_ts {
                continue;
            }
            let bar = BarInput {
                timestamp: b.timestamp.timestamp(),
                open: b.open,
                high: b.high,
                low: b.low,
                close: b.close,
                volume: b.volume,
            };
            dernier_close = bar.close;
            lifecycle.update(&mut carnet, &out_vide, &bar, i, &cal, &mut scoring, &[], &[]);
            if carnet[0].state == TradeState::Closed {
                break;
            }
        }
        let t = &carnet[0];
        if t.state != TradeState::Closed {
            continue; // encore vivant selon le moteur → Actif légitime
        }
        // Chaînes canoniques de lifecycle_diff::verdict_texte (engine_v12).
        let verdict = match t.verdict() {
            Verdict::Tp3 => "TP3",
            Verdict::Tp2 => "TP2+BE",
            Verdict::Tp1 => "TP1+BE",
            Verdict::Sl => "SL",
            Verdict::Be => "BE",
            Verdict::Expire => "Expire",
        };
        let r = t.realized_r();
        let prix = match t.close_reason {
            Some(CloseReason::Sl) => t.sl,
            Some(CloseReason::Be) => t.entry,
            Some(CloseReason::Tp2Sl) => t.tp1,
            Some(CloseReason::Tp3) => t.tp3,
            _ => dernier_close,
        };
        // Remplissage AVANT clôture (marquer_remplie exige statut='Actif').
        if t.filled && a.heure_entree.is_none() {
            if let Some(f) = t.fill_ts {
                let _ = db.marquer_remplie_par_cle(&a.cle_moteur, asset, f).await;
            }
        }
        match db
            .fermer_signal_par_cle(
                &a.cle_moteur,
                asset,
                verdict,
                prix,
                r,
                t.close_ts.unwrap_or(maintenant),
            )
            .await
        {
            Ok(n) if n > 0 => {
                tracing::info!(
                    "Réconciliation cycle de vie : {} {} fermé « {} » ({:.2}R)",
                    asset,
                    tf.as_str(),
                    verdict,
                    r
                );
                total += n;
            }
            Ok(_) => {}
            Err(err) => tracing::warn!("Réconciliation cycle de vie ({}): {}", a.cle_moteur, err),
        }
    }
    total
}
