//! Rattrapage des positions SMC et KDJ orphelines (incident 18/09).
//!
//! Le suivi des trades SMC vit dans l'état mémoire du moteur runtime : un
//! redémarrage de l'app orphelinise toute position encore ouverte (le
//! redémarrage du 17/09 19h30 a laissé 5 positions M15 sans gestion — LTC a
//! touché TP3 le lendemain sans jamais clôturer). Ce poste reprend le relais :
//! à chaque tick, toute position SMC remplie encore Active est re-jouée depuis
//! son entrée par le MÊME lifecycle que le moteur (gestion_trades, validé
//! miroir Pine) — si sa condition de sortie est atteinte, elle est clôturée
//! officiellement (même format que le runtime) ; sinon son suivi progressif
//! (sl_effectif / tps_atteints — la fonction restée morte jusqu'ici) est
//! persisté. Idempotent : une position déjà Fermée n'est jamais retouchée,
//! et le moteur vivant clôturant le premier rend l'écriture de l'autre no-op.
//!
//! Extension KDJ (incident 25/10 — 18 Actifs dont 10 jamais remplis, faux
//! vivants de 15 jours, doublons par asset) : la position moteur vit en
//! mémoire pure, et la chauffe au redémarrage interdit toute re-dérivation
//! (garde anti-doublon). Trois passes réparent la classe entière :
//! ① REMPLISSAGE RÉTROACTIF : un signal KDJ se remplit TOUJOURS à l'open de
//!    la barre suivante (modèle papier) — si l'app s'est arrêtée dans
//!    l'heure entre signal et remplissage, on marque le fill à la première
//!    barre H1 postérieure ; le rejeu TP/SL prend le relais au même tick.
//! ② FIN DE SÉRIE : franchissement sur la dernière barre connue d'un asset
//!    RETIRÉ du pipeline (la barre suivante n'existera jamais — cas ADA
//!    24/10) → clôture au dernier close connu au lieu d'attendre à vie.
//! ③ DÉDOUBLONNAGE : la stratégie ne porte qu'UNE position par asset — des
//!    Actifs remplis multiples signifient des sorties perdues aux arrêts
//!    (retournement non reconstituable). Chaque ancienne se clôture
//!    « Retournement » à l'open de la barre d'entrée de la suivante (le
//!    modèle : sortie et nouvelle entrée au même open).


use std::sync::Arc;

/// Verdict canonique en base — miroir de engine_v12::lifecycle_diff.
fn verdict_texte(t: &gestion_trades::Trade) -> String {
    use gestion_trades::Verdict;
    match t.verdict() {
        Verdict::Tp3 => "TP3",
        Verdict::Ts => "TS",
        Verdict::Tp2 => "TP2+BE",
        Verdict::Tp1 => "TP1+BE",
        Verdict::Sl => "SL",
        Verdict::Be => "BE",
        Verdict::Expire => "Expire",
    }
    .to_string()
}

/// Prix de sortie selon la cause (même logique que le moteur).
fn prix_de_sortie(t: &gestion_trades::Trade, close_bougies: f64) -> f64 {
    use gestion_trades::CloseReason;
    match t.close_reason {
        Some(CloseReason::Ts) => t.ts_px.unwrap_or(close_bougies),
        Some(CloseReason::Tp2Sl) => t.tp1,
        Some(CloseReason::Be) => t.entry,
        Some(CloseReason::Sl) => t.sl,
        Some(CloseReason::Tp3) => t.tp3,
        _ => close_bougies,
    }
}

/// Une position ouverte à rejouer.
struct Orphelin {
    id: String,
    tf_nom: String,
    cle: String,
    asset: String,
    /// Branche de rattrapage : KDJ = TP unique, franchissement à la barre,
    /// exécution à l'open suivant (fix 09/10 — l'extension KDJ de la
    /// requête sautait ces trades via le garde « ≥ 2 TPs » du modèle SMC).
    kdj: bool,
    tf_mins: u32,
    long: bool,
    entree: f64,
    sl: f64,
    tps: Vec<f64>,
    heure_entree: i64,
    sl_effectif_actuel: Option<f64>,
    tps_atteints_actuel: Option<String>,
}

/// Passe de rattrapage : clôture les sorties dues, persiste le suivi des
/// vivantes. Appelée à chaque tick runtime (coût : lecture des Actifs + rejeu
/// des bougies depuis l'entrée — quelques centaines au pire).
pub async fn rattraper(db: &Arc<db::Database>) {
    use sqlx::Row;
    // ① Remplissages KDJ rétroactifs AVANT le rejeu : les signaux ainsi
    // récupérés entrent dans le passage TP/SL du même tick.
    remplir_kdj_retroactifs(db).await;

    // Assets actifs du pipeline (passe ② : fin de série des retirés).
    let actifs: std::collections::HashSet<String> =
        sqlx::query("SELECT id FROM assets WHERE actif = 1")
            .fetch_all(db.pool())
            .await
            .map(|rows| {
                rows.iter()
                    .map(|r| r.get::<String, _>("id"))
                    .collect()
            })
            .unwrap_or_default();

    let rows = match sqlx::query(
        "SELECT id, cle_moteur, asset, timeframe, strategie, direction, prix_entree, stop_loss,
                take_profit, heure_entree, sl_effectif, tps_atteints
         FROM signaux
         WHERE strategie IN ('SMC', 'kdj_halftrend') AND statut = 'Actif' AND heure_entree IS NOT NULL
           AND cle_moteur IS NOT NULL",
    )
    .fetch_all(db.pool())
    .await
    {
        Ok(r) => r,
        Err(_) => return,
    };

    for r in rows {
        use sqlx::Row;
        let tps: Vec<f64> = r
            .try_get::<String, _>("take_profit")
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        let o = Orphelin {
            id: r.get("id"),
            cle: r.get("cle_moteur"),
            asset: r.get("asset"),
            kdj: r.get::<String, _>("strategie") == "kdj_halftrend",
            tf_mins: match r.get::<String, _>("timeframe").as_str() {
                "M1" => 1,
                "M5" => 5,
                "M30" => 30,
                "H1" => 60,
                "H4" => 240,
                "D1" => 1440,
                _ => 15,
            },
            tf_nom: r.get("timeframe"),
            long: r.get::<String, _>("direction").eq_ignore_ascii_case("long"),
            entree: r.try_get("prix_entree").unwrap_or(0.0),
            sl: r.try_get("stop_loss").unwrap_or(0.0),
            tps,
            heure_entree: r.get("heure_entree"),
            sl_effectif_actuel: r.try_get::<Option<f64>, _>("sl_effectif").ok().flatten(),
            tps_atteints_actuel: r
                .try_get::<Option<String>, _>("tps_atteints")
                .ok()
                .flatten(),
        };
        // Modèle SMC : TP1+TP2 minimum (BE, time-stops). KDJ : TP unique.
        let tps_ok = if o.kdj { !o.tps.is_empty() } else { o.tps.len() >= 2 };
        if !tps_ok || o.entree <= 0.0 {
            continue;
        }
        if let Err(e) = rattraper_un(db, &o, actifs.contains(&o.asset)).await {
            tracing::warn!("Rattrapage SMC {} {}: {}", o.asset, o.id, e);
        }
    }

    // ③ Dédoublonnage : une seule position KDJ remplie par asset.
    dedoublonner_kdj(db).await;
}

async fn rattraper_un(db: &Arc<db::Database>, o: &Orphelin, asset_actif: bool) -> anyhow::Result<()> {
    let jours = ((chrono::Utc::now().timestamp() - o.heure_entree) / 86_400 + 2).max(1) as u32;
    let asset = common::Asset::from(o.asset.as_str());
    let Ok(tf) = common::Timeframe::try_from(o.tf_nom.as_str()) else {
        return Ok(());
    };
    let bougies = db
        .obtenir_bougies_depuis_jours(&asset, &tf, jours)
        .await
        .unwrap_or_default();
    let fenetre: Vec<&common::Candle> = bougies
        .iter()
        .filter(|b| b.timestamp.timestamp() >= o.heure_entree)
        .collect();
    if fenetre.is_empty() {
        return Ok(());
    }

    // Branche KDJ (fix 09/10) : TP unique, franchissement à la barre,
    // exécution à l'open suivant — le lifecycle SMC (BE/trailing/time-stops)
    // ne s'applique pas.
    if o.kdj {
        return rattraper_un_kdj(db, o, &fenetre, asset_actif).await;
    }

    // Trade reconstruit à l'identique du moteur (niveaux du signal, risk0).
    let bar0 = gestion_trades::BarInput {
        timestamp: o.heure_entree,
        open: o.entree, high: o.entree, low: o.entree, close: o.entree,
        volume: 0.0,
    };
    let mut trade = if o.long {
        gestion_trades::Trade::new_buy(
            1, gestion_trades::TradeSource::Ob, o.entree, o.sl,
            o.tps[0], o.tps[1], *o.tps.get(2).unwrap_or(&o.tps[1]),
            10, (o.entree - o.sl).abs(), &bar0, 0, None,
        )
    } else {
        gestion_trades::Trade::new_sell(
            1, gestion_trades::TradeSource::Ob, o.entree, o.sl,
            o.tps[0], o.tps[1], *o.tps.get(2).unwrap_or(&o.tps[1]),
            10, (o.entree - o.sl).abs(), &bar0, 0, None,
        )
    };
    trade.filled = true;
    trade.fill_ts = Some(o.heure_entree);

    // MÊMES time-stops que le moteur v12 (durees.rs).
    let trade_max = smc::v12::durees::trade_max_mins(o.tf_mins) * 60;
    let cal = smc::v12::calibration::AssetCalibration::detect(&o.asset, &o.tf_nom);
    let tp3_max = smc::v12::durees::tp3_max_mins(&cal, o.tf_mins) * 60;
    let mut lifecycle = gestion_trades::TradeLifecycle::new(trade_max, tp3_max);
    lifecycle.definir_be_offset_r(0.0); // sémantique SMC

    let mut jambes = [trade];
    let mut ferme: Option<(String, f64, f64, i64)> = None;
    for (i, b) in fenetre.iter().enumerate() {
        let bar = gestion_trades::BarInput {
            timestamp: b.timestamp.timestamp(),
            open: b.open, high: b.high, low: b.low, close: b.close,
            volume: 0.0,
        };
        lifecycle.update(&mut jambes, &bar, i + 1, &mut gestion_trades::HookVide);
        let trade = &jambes[0];
        if trade.close_reason.is_some() {
            let verdict = verdict_texte(&trade);
            let prix = prix_de_sortie(&trade, b.close);
            let r = trade.realized_r();
            ferme = Some((verdict, prix, r, b.timestamp.timestamp()));
            break;
        }
    }

    let trade = &jambes[0];
    if let Some((verdict, prix, r, ferme_le)) = ferme {
        let n = db
            .fermer_signal_par_cle(&o.cle, &o.asset, &verdict, prix, r, ferme_le)
            .await
            .unwrap_or(0);
        if n > 0 {
            tracing::info!(
                "🩹 Rattrapage SMC : {} {} clôturé {} ({:+.2}R) — position orpheline réparée",
                o.asset, o.tf_nom, verdict, r
            );
        }
        return Ok(());
    }

    // Toujours vivante : persister le suivi progressif (fonction historiquement
    // morte — branchée ici) uniquement s'il a changé.
    let mut tps_touches: Vec<&str> = Vec::new();
    if trade.tp1_hit {
        tps_touches.push("tp1");
    }
    if trade.tp2_ts > 0 {
        tps_touches.push("tp2");
    }
    let tps_json = serde_json::to_string(&tps_touches).unwrap_or_default();
    let inchangé = o.sl_effectif_actuel == Some(trade.sl)
        && o.tps_atteints_actuel.as_deref() == Some(tps_json.as_str());
    if !inchangé {
        let _ = db::signaux::maj_suivi_progressif_smc(
            db.pool(),
            &o.id,
            trade.sl,
            &tps_touches,
        )
        .await;
    }
    Ok(())
}



/// Où la sortie KDJ s'est déclenchée : (tp ?, index de la barre de
/// franchissement). Fidèle au moteur live — croisement prec→courant (la
/// barre d'entrée a pour « prec » son open : le prix y était au fill),
/// priorité TP si double franchissement dans la même barre.
fn franchissement_kdj(bars: &[&common::Candle], long: bool, sl: f64, tp: f64) -> Option<(bool, usize)> {
    for i in 0..bars.len() {
        let b = bars[i];
        let (h_prec, l_prec) = if i == 0 { (b.open, b.open) } else { (bars[i - 1].high, bars[i - 1].low) };
        let (tp_c, sl_c) = if long {
            (h_prec <= tp && b.high > tp, l_prec >= sl && b.low < sl)
        } else {
            (l_prec >= tp && b.low < tp, h_prec <= sl && b.high > sl)
        };
        if tp_c {
            return Some((true, i));
        }
        if sl_c {
            return Some((false, i));
        }
    }
    None
}

/// Rattrapage KDJ : clôture à l'OPEN de la barre suivant le franchissement
/// (le moteur diffère l'exécution — même convention de R), verdict TP/SL.
/// Franchissement sur la dernière barre connue → attendre la barre suivante,
/// SAUF si l'asset a été retiré du pipeline (elle n'existera jamais) : on
/// clôture alors au dernier close connu (convention fin de série — cas ADA
/// 24/10 : TP franchi sur la dernière barre avant retrait). Retournement non
/// reconstituable (état indicateurs) : le trade reste Actif jusqu'à TP/SL.
async fn rattraper_un_kdj(
    db: &Arc<db::Database>,
    o: &Orphelin,
    fenetre: &[&common::Candle],
    asset_actif: bool,
) -> anyhow::Result<()> {
    let tp = o.tps[0];
    let risque = (o.entree - o.sl).abs();
    if risque <= 0.0 {
        return Ok(());
    }
    let Some((est_tp, i)) = franchissement_kdj(fenetre, o.long, o.sl, tp) else {
        return Ok(()); // toujours vivante
    };
    let verdict = if est_tp { "TP" } else { "SL" };
    let dir = if o.long { 1.0 } else { -1.0 };
    let Some(suivante) = fenetre.get(i + 1) else {
        if !asset_actif {
            if let Some(derniere) = fenetre.last() {
                let r = dir * (derniere.close - o.entree) / risque;
                let n = db
                    .fermer_signal_par_cle(&o.cle, &o.asset, verdict, derniere.close, r, derniere.timestamp.timestamp())
                    .await
                    .unwrap_or(0);
                if n > 0 {
                    tracing::info!(
                        "🩹 Rattrapage KDJ : {} {} clôturé {} fin de série ({:+.2}R) — asset retiré, dernière barre",
                        o.asset, o.tf_nom, verdict, r
                    );
                }
            }
        }
        return Ok(()); // asset actif : la barre suivante viendra
    };
    let prix = suivante.open;
    let r = dir * (prix - o.entree) / risque;
    let n = db
        .fermer_signal_par_cle(&o.cle, &o.asset, verdict, prix, r, suivante.timestamp.timestamp())
        .await
        .unwrap_or(0);
    if n > 0 {
        tracing::info!(
            "🩹 Rattrapage KDJ : {} {} clôturé {} ({:+.2}R) — position orpheline réparée",
            o.asset, o.tf_nom, verdict, r
        );
    }
    Ok(())
}

// ── Passe ① : remplissages KDJ rétroactifs ──────────────────────────────────

/// Un signal KDJ émis à la clôture N se remplit TOUJOURS à l'open de la
/// barre N+1 (modèle papier, aucun ordre). Si l'app s'est arrêtée dans
/// l'heure entre les deux, le Fill moteur n'est jamais venu — au
/// redémarrage la chauffe pré-charge N et N+1 depuis la base et la garde
/// anti-doublon interdit la re-dérivation : le signal croupissait Actif
/// sans heure d'entrée, hors rattrapage (10 cas constatés 14-23/10).
/// On marque le remplissage à la première barre H1 postérieure au signal
/// (l'horodatage du signal vit dans la clé `kdj-{ASSET}-{ts}`).
async fn remplir_kdj_retroactifs(db: &Arc<db::Database>) {
    let Ok(rows) = sqlx::query(
        "SELECT cle_moteur, asset FROM signaux \
         WHERE strategie = 'kdj_halftrend' AND statut = 'Actif' AND heure_entree IS NULL \
           AND cle_moteur IS NOT NULL",
    )
    .fetch_all(db.pool())
    .await
    else {
        return;
    };
    for r in rows {
        use sqlx::Row;
        let cle: String = r.get("cle_moteur");
        let asset: String = r.get("asset");
        let Some(ts_signal) = cle.rsplit('-').next().and_then(|s| s.parse::<i64>().ok()) else {
            continue;
        };
        let Ok(a) = common::Asset::try_from(asset.as_str()) else {
            continue;
        };
        let Ok(bougies) = db
            .obtenir_bougies_depuis_jours(&a, &common::Timeframe::H1, 60)
            .await
        else {
            continue;
        };
        let Some(suivante) = bougies.iter().find(|b| b.timestamp.timestamp() > ts_signal) else {
            continue; // la barre de remplissage n'existe pas encore
        };
        let ts = suivante.timestamp.timestamp();
        if let Ok(n) = db.marquer_remplie_par_cle(&cle, &asset, ts).await {
            if n > 0 {
                tracing::info!(
                    "🩹 Rattrapage KDJ : {asset} rempli rétroactivement (barre {ts}) — signal orphelin récupéré"
                );
            }
        }
    }
}

// ── Passe ③ : dédoublonnage KDJ ─────────────────────────────────────────────

/// Open de la barre portant `ts` — prix de sortie du modèle retournement.
fn open_de_la_barre(bars: &[&common::Candle], ts: i64) -> Option<f64> {
    bars.iter().find(|b| b.timestamp.timestamp() == ts).map(|b| b.open)
}

/// La stratégie KDJ ne porte qu'UNE position par asset. Des Actifs remplis
/// multiples sur un même asset = sorties perdues aux arrêts de l'app (le
/// retournement n'est pas reconstituable). Reconstruction mécanique : chaque
/// ancienne se clôture « Retournement » à l'OPEN de la barre d'entrée de la
/// SUIVANTE (le modèle : sortie et nouvelle entrée au même open). La plus
/// récente reste la position vivante.
async fn dedoublonner_kdj(db: &Arc<db::Database>) {
    use sqlx::Row;
    let Ok(rows) = sqlx::query(
        "SELECT cle_moteur, asset, direction, prix_entree, stop_loss, heure_entree \
         FROM signaux \
         WHERE strategie = 'kdj_halftrend' AND statut = 'Actif' AND heure_entree IS NOT NULL \
         ORDER BY asset, heure_entree",
    )
    .fetch_all(db.pool())
    .await
    else {
        return;
    };

    // Groupes par asset (au moins 2 remplis) : fermer chaque trade à
    // l'entrée du suivant, garder le dernier.
    let mut idx = 0;
    while idx < rows.len() {
        let asset: String = rows[idx].get("asset");
        let mut fin = idx;
        while fin < rows.len() && rows[fin].get::<String, _>("asset") == asset {
            fin += 1;
        }
        if fin - idx >= 2 {
            let Ok(a) = common::Asset::try_from(asset.as_str()) else {
                idx = fin;
                continue;
            };
            let bougies = db
                .obtenir_bougies(&a, &common::Timeframe::H1, 300)
                .await
                .unwrap_or_default();
            let refs: Vec<&common::Candle> = bougies.iter().collect();
            for k in idx..fin - 1 {
                let ancienne = &rows[k];
                let suivante_row = &rows[k + 1];
                let ts_sortie: i64 = suivante_row.get("heure_entree");
                let cle: String = ancienne.get("cle_moteur");
                let entree: f64 = ancienne.try_get("prix_entree").unwrap_or(0.0);
                let sl: f64 = ancienne.try_get("stop_loss").unwrap_or(0.0);
                let risque = (entree - sl).abs();
                if risque <= 0.0 || entree <= 0.0 {
                    continue;
                }
                let long = ancienne
                    .get::<String, _>("direction")
                    .eq_ignore_ascii_case("long");
                let dir = if long { 1.0 } else { -1.0 };
                // Sortie à l'open de la barre d'entrée de la suivante ;
                // repli sur son prix d'entrée si la barre n'est plus en
                // fenêtre (trade ancien).
                let prix = open_de_la_barre(&refs, ts_sortie)
                    .unwrap_or_else(|| suivante_row.try_get("prix_entree").unwrap_or(entree));
                let r = dir * (prix - entree) / risque;
                let n = db
                    .fermer_signal_par_cle(&cle, &asset, "Retournement", prix, r, ts_sortie)
                    .await
                    .unwrap_or(0);
                if n > 0 {
                    tracing::info!(
                        "🩹 Rattrapage KDJ : {asset} doublon clôturé Retournement ({r:+.2}R) à l'entrée du signal suivant"
                    );
                }
            }
        }
        idx = fin;
    }
}

#[cfg(test)]
mod tests_kdj {
    use super::*;
    use chrono::{TimeZone, Utc};

    fn bougie(ts: i64, o: f64, h: f64, l: f64, c: f64) -> common::Candle {
        common::Candle {
            timestamp: Utc.timestamp_opt(ts, 0).single().unwrap_or_default(),
            open: o, high: h, low: l, close: c, volume: 0.0,
        }
    }

    /// BNB/SOL (incident 09/10) : long dont le SL est franchi à la 3e barre
    /// → verdict SL, sortie à l'open de la barre suivante, R négatif.
    #[test]
    fn sl_franche_puis_open_suivant() {
        let bars = vec![
            bougie(3600, 100.0, 101.0, 99.5, 100.5),  // entrée (open 100)
            bougie(7200, 100.5, 102.0, 100.0, 101.5),
            bougie(10800, 101.5, 101.8, 94.0, 95.0),  // low 94 < SL 96
            bougie(14400, 94.5, 96.0, 93.0, 95.0),    // open 94.5 = prix de sortie
        ];
        let refs: Vec<&common::Candle> = bars.iter().collect();
        let (est_tp, i) = franchissement_kdj(&refs, true, 96.0, 110.0).expect("SL franchi");
        assert!(!est_tp);
        assert_eq!(i, 2);
        // R = (94.5 − 100) / (100 − 96) = −1.375
        let r = (94.5_f64 - 100.0) / (100.0 - 96.0);
        assert!((r - (-1.375)).abs() < 1e-9);
    }

    /// XPTUSD : long dont le TP (unique) est dépassé → verdict TP.
    #[test]
    fn tp_franche() {
        let bars = vec![
            bougie(0, 1700.0, 1702.0, 1698.0, 1701.0),
            bougie(3600, 1701.0, 1660.0, 1655.0, 1658.0), // short : TP 1658 franchi
        ];
        let refs: Vec<&common::Candle> = bars.iter().collect();
        let (est_tp, _) = franchissement_kdj(&refs, false, 1720.0, 1658.5).expect("TP franchi");
        assert!(est_tp);
    }

    /// Double franchissement dans la même barre → TP prioritaire (moteur).
    #[test]
    fn tp_prioritaire_sur_sl_dans_la_meme_barre() {
        let bars = vec![
            bougie(0, 100.0, 100.5, 99.5, 100.0),
            bougie(3600, 100.0, 111.0, 94.0, 95.0), // high > TP 110 ET low < SL 96
        ];
        let refs: Vec<&common::Candle> = bars.iter().collect();
        let (est_tp, _) = franchissement_kdj(&refs, true, 96.0, 110.0).expect("franchi");
        assert!(est_tp, "TP gagne comme dans le moteur live");
    }

    /// Aucun franchissement → None : la trade reste Active (rattrapage no-op).
    #[test]
    fn vivante_sans_franchemement() {
        let bars = vec![
            bougie(0, 100.0, 101.0, 99.0, 100.5),
            bougie(3600, 100.5, 103.0, 99.5, 102.0),
        ];
        let refs: Vec<&common::Candle> = bars.iter().collect();
        assert!(franchissement_kdj(&refs, true, 96.0, 110.0).is_none());
    }

    /// Franchissement sur la DERNIÈRE barre : l'appelant attend la barre
    /// suivante (pas de clôture au prix intrabarôme — fidélité moteur).
    #[test]
    fn franche_sur_derniere_barre_attend_la_suivante() {
        let bars = vec![
            bougie(0, 100.0, 101.0, 99.0, 100.5),
            bougie(3600, 100.5, 112.0, 100.0, 111.0), // TP sur la dernière
        ];
        let refs: Vec<&common::Candle> = bars.iter().collect();
        let (_, i) = franchissement_kdj(&refs, true, 96.0, 110.0).expect("franchi");
        assert_eq!(i, refs.len() - 1, "l'appelant vérifiera fenetre.get(i+1)");
    }

    /// Passe ③ : l'open de la barre d'entrée de la suivante est le prix de
    /// sortie du retournement ; introuvable → repli sur son prix d'entrée.
    #[test]
    fn open_de_la_barre_et_repli() {
        let bars = vec![
            bougie(3600, 100.0, 101.0, 99.0, 100.5),
            bougie(7200, 105.0, 106.0, 104.0, 105.5),
        ];
        let refs: Vec<&common::Candle> = bars.iter().collect();
        assert_eq!(open_de_la_barre(&refs, 7200), Some(105.0));
        assert_eq!(open_de_la_barre(&refs, 9999), None);
    }
}
