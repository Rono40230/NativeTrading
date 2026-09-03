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
    ecritures_db += reconstruire_straddles(db, asset.as_str(), tf).await;
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
            tp2_extremum: None,
            ts_px: None,
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
            Verdict::Ts => "TS",
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
            Some(CloseReason::Ts) => t.ts_px.unwrap_or(t.tp2),
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

/// Tampon après TP1 : le SL passe à E∓0,5R (décision 27/08 — moteur straddle).
const STRADDLE_TAMPON_R: f64 = 0.5;
/// Time-stop canonique après l'ouverture (minutes) — ParamsStraddle.
const STRADDLE_TIME_STOP_MIN: i64 = 60;

/// Dernier filet straddle — cycle de vie SQL sur les niveaux de la ligne.
///
/// Le moteur straddle vit uniquement dans `on_tick` : le replay de
/// redémarrage (on_close) ne régénère JAMAIS une passe — ouverte avant un
/// arrêt, elle devient orpheline (personne n'évalue ses jambes, la ligne
/// reste « Actif » à vie). On rejoue ici la mécanique exacte des 2 jambes
/// du moteur (time-stop 60 min → SL/trailing → TP1 tampon 0,5R → TP2 +
/// trailing 1R) sur les bougies M1 stockées, barre par barre : sortie au
/// SL de la barre précédente, puis resserrements sur les extrêmes de la
/// barre courante — et le verdict NET du moteur (« tp2 »/« sl »/« be »/
/// « expire » + R net des 2 jambes). Les passes straddle vivent sur M1.
async fn reconstruire_straddles(db: &Arc<Database>, asset: &str, tf: Timeframe) -> u64 {
    if !matches!(tf, common::Timeframe::M1) {
        return 0;
    }
    let actifs: Vec<db::signaux::SignalActifCle> =
        match db::signaux::lister_actifs_avec_cle(db.pool()).await {
            Ok(a) => a
                .into_iter()
                .filter(|s| {
                    s.asset == asset && s.timeframe == tf.as_str() && s.strategie == "straddle"
                })
                .collect(),
            Err(_) => return 0,
        };
    if actifs.is_empty() {
        return 0;
    }
    let params = db::strategies_params::lire_straddle_params(db.pool()).await;

    let maintenant = chrono::Utc::now().timestamp();
    let plus_vieux = actifs.iter().map(|a| a.heure_entree.unwrap_or(a.cree_le)).min().unwrap_or(maintenant);
    let limite = ((maintenant - plus_vieux) / 60 + 64).max(10);
    let Ok(actif) = Asset::try_from(asset) else {
        return 0;
    };
    let bougies = match db.obtenir_bougies(&actif, &tf, limite).await {
        Ok(b) => b,
        Err(_) => return 0,
    };

    /// Une jambe du straddle (LONG ou SHORT).
    struct Jambe {
        long: bool,
        sl: f64,
        tp1: f64,
        tp2: f64,
        meilleur_depuis_tp2: Option<f64>,
        fermee: Option<(String, f64)>,
    }

    let mut total = 0u64;
    for a in &actifs {
        let Some(ouverture) = a.heure_entree else { continue };
        let entree = a.prix_entree;
        let r = entree - a.stop_loss;
        if r <= 0.0 {
            continue;
        }
        let distance_trail = params.trailing_r * r;
        let mut jambes = [
            Jambe {
                long: true,
                sl: a.stop_loss,
                tp1: entree + r,
                tp2: entree + 2.0 * r,
                meilleur_depuis_tp2: None,
                fermee: None,
            },
            Jambe {
                long: false,
                sl: entree + r,
                tp1: entree - r,
                tp2: entree - 2.0 * r,
                meilleur_depuis_tp2: None,
                fermee: None,
            },
        ];
        let r_a = |long: bool, prix: f64| -> f64 {
            if long { (prix - entree) / r } else { (entree - prix) / r }
        };

        let mut prix_cloture = entree;
        let mut ts_cloture = ouverture;
        'barres: for b in &bougies {
            let ts = b.timestamp.timestamp();
            if ts < ouverture {
                continue;
            }
            prix_cloture = b.close;
            ts_cloture = ts;
            for j in jambes.iter_mut() {
                if j.fermee.is_some() {
                    continue;
                }
                // 1. Time-stop : sortie à l'heure, au prix de clôture.
                if ts - ouverture >= STRADDLE_TIME_STOP_MIN * 60 {
                    j.fermee = Some(("TimeStop".into(), r_a(j.long, b.close)));
                    continue;
                }
                // 2. SL / trailing stop (niveau de la barre précédente) —
                //    TS au-delà de TP1 (trailing armé), SL sinon.
                if (j.long && b.low <= j.sl) || (!j.long && b.high >= j.sl) {
                    let verdict = if (j.long && j.sl > j.tp1) || (!j.long && j.sl < j.tp1) {
                        "TS"
                    } else {
                        "SL"
                    };
                    j.fermee = Some((verdict.to_string(), r_a(j.long, j.sl)));
                    continue;
                }
                // 3. TP1 : SL resserré au tampon E∓0,5R.
                if (j.long && b.high >= j.tp1) || (!j.long && b.low <= j.tp1) {
                    let tampon = if j.long { entree - r * STRADDLE_TAMPON_R } else { entree + r * STRADDLE_TAMPON_R };
                    if (j.long && j.sl < tampon) || (!j.long && j.sl > tampon) {
                        j.sl = tampon;
                    }
                }
                // 4. TP2 : SL à TP1 + trailing sur le meilleur extrême.
                if (j.long && b.high >= j.tp2) || (!j.long && b.low <= j.tp2) {
                    if (j.long && j.sl < j.tp1) || (!j.long && j.sl > j.tp1) {
                        j.sl = j.tp1;
                    }
                    let meilleur = match j.meilleur_depuis_tp2 {
                        Some(m) if j.long => m.max(b.high),
                        Some(m) => m.min(b.low),
                        None if j.long => b.high,
                        None => b.low,
                    };
                    j.meilleur_depuis_tp2 = Some(meilleur);
                    let cible = if j.long { meilleur - distance_trail } else { meilleur + distance_trail };
                    if (j.long && cible > j.sl) || (!j.long && cible < j.sl) {
                        j.sl = cible;
                    }
                }
            }
            if jambes.iter().all(|j| j.fermee.is_some()) {
                break 'barres;
            }
        }

        if jambes.iter().any(|j| j.fermee.is_none()) {
            continue; // au moins une jambe vivante → passe légitimement ouverte
        }
        // Verdict net du moteur (verdict_net, à l'identique).
        let net: f64 = jambes
            .iter()
            .map(|j| j.fermee.as_ref().map(|(_, rr)| *rr).unwrap_or(0.0))
            .sum();
        let un_tp1 = jambes.iter().any(|j| {
            j.meilleur_depuis_tp2.is_some()
                || matches!(j.fermee.as_ref().map(|(v, _)| v.as_str()), Some("TS") | Some("BE"))
        });
        let verdict = if net > 1e-9 {
            "tp2"
        } else if net < -1e-9 {
            "sl"
        } else if un_tp1 {
            "be"
        } else {
            "expire"
        };
        match db
            .fermer_signal_par_cle(&a.cle_moteur, asset, verdict, prix_cloture, net, ts_cloture)
            .await
        {
            Ok(n) if n > 0 => {
                tracing::info!(
                    "Réconciliation straddle : {} fermé « {} » ({:.2}R net)",
                    asset,
                    verdict,
                    net
                );
                total += n;
            }
            Ok(_) => {}
            Err(err) => tracing::warn!("Réconciliation straddle ({}): {}", a.cle_moteur, err),
        }
    }
    total
}
