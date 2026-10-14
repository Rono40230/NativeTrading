//! §Rattrapage straddle orphelin (owner 14/10) : les positions `Actif`
//! de la table signaux dont l'asset a QUITTÉ le périmètre ne sont plus
//! gérées par le runtime (il ne nourrit que le périmètre armé). Ces
//! positions doivent pourtant vivre jusqu'à SL/TP/expire — même principe
//! que le garde-fou `assets_garde` côté alimentation, dans l'autre sens :
//! *ouvert ⇒ géré jusqu'au bout*.
//!
//! Le lifecycle rejoue les deux jambes depuis l'entrée (mêmes niveaux que
//! le moteur : SL = E∓1R, TP1/2/3 = ±1/2/3R, comptabilité TP acquis) et
//! ferme la passe au verdict net (somme des jambes), comme le moteur.

use std::sync::Arc;

use db::Database;

/// Un trade straddle Both à rejouer.
struct Passe {
    id: String,
    asset: String,
    entree: f64,
    sl_long: f64,
    /// SL de la jambe SHORT (au-dessus de l'entrée) — stocké en miroir
    /// dans la colonne `sl_short` (ou dérivé : E + (E − SL_long)).
    sl_short: f64,
    tps_long: Vec<f64>,
    tps_short: Vec<f64>,
    heure_entree: i64,
}

/// Passe de rattrapage : ferme les sorties dues des positions Both
/// orphelines (hors périmètre ou non), persiste les vivantes. Appelée au
/// boot + périodiquement (même discipline que rattrapage_smc).
pub async fn rattraper(db: &Arc<Database>) {
    let Ok(rows) = sqlx::query(
        "SELECT id, asset, prix_entree, COALESCE(sl_effectif, stop_loss) AS sl,
                COALESCE(sl_short, stop_loss + (prix_entree - COALESCE(sl_effectif, stop_loss))) AS sl_s,
                COALESCE(take_profit, '[]') AS tps_l,
                COALESCE(take_profit_short, '[]') AS tps_s,
                COALESCE(heure_entree, 0) AS he
         FROM signaux
         WHERE strategie = 'straddle' AND statut = 'Actif'
           AND prix_entree IS NOT NULL AND heure_entree IS NOT NULL",
    )
    .fetch_all(db.pool())
    .await
    else {
        return;
    };

    for r in &rows {
        use sqlx::Row;
        let tps_l: Vec<f64> = r
            .try_get::<String, _>("tps_l")
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        let tps_s: Vec<f64> = r
            .try_get::<String, _>("tps_s")
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        let p = Passe {
            id: r.get("id"),
            asset: r.get("asset"),
            entree: r.get("prix_entree"),
            sl_long: r.get("sl"),
            sl_short: r.get("sl_s"),
            tps_long: tps_l,
            tps_short: tps_s,
            heure_entree: r.get("he"),
        };
        if let Err(e) = rattraper_un(db, &p).await {
            tracing::warn!("rattrapage_straddle {}: {}", p.asset, e);
        }
    }
}

/// R d'une jambe selon où le prix a terminé (comptabilité TP acquis :
/// un TP touché reste acquis même si l'autre côté meurt ensuite).
fn r_jambe(sl: f64, tp1: f64, tp2: f64, tp3: f64, short: bool, high: f64, low: f64) -> Option<(f64, f64)> {
    // Retourne (R, prix de sortie) ; None = jambe vivante.
    if short {
        if low <= tp3 { return Some((3.0, tp3)); }
        if low <= tp2 { return Some((2.0, tp2)); }
        if low <= tp1 { return Some((1.0, tp1)); }
        if high >= sl { return Some((-1.0, sl)); }
    } else {
        if high >= tp3 { return Some((3.0, tp3)); }
        if high >= tp2 { return Some((2.0, tp2)); }
        if high >= tp1 { return Some((1.0, tp1)); }
        if low <= sl { return Some((-1.0, sl)); }
    }
    None
}

async fn rattraper_un(db: &Arc<Database>, p: &Passe) -> anyhow::Result<()> {
    let jours = ((chrono::Utc::now().timestamp() - p.heure_entree) / 86_400 + 2).max(1) as u32;
    let asset = common::Asset::from(p.asset.as_str());
    let bougies = db
        .obtenir_bougies_depuis_jours(&asset, &common::Timeframe::M1, jours)
        .await
        .unwrap_or_default();
    let fenetre: Vec<&common::Candle> = bougies
        .iter()
        .filter(|b| b.timestamp.timestamp() >= p.heure_entree)
        .collect();
    if fenetre.is_empty() {
        return Ok(());
    }

    // Le risque est dérivé : E − SL_long = distance SL (le R du moteur).
    let r = (p.entree - p.sl_long).abs();
    if r <= 0.0 {
        return Ok(());
    }

    // Rejouer les deux jambes : le premier bar qui touche un niveau gèle
    // cette jambe (TP acquis), l'autre continue. Priorité au pire cas sur
    // un même bar : SL d'abord (le moteur regarde le range du bar).
    let mut r_long: Option<f64> = None;
    let mut r_short: Option<f64> = None;
    let mut ferme_le: i64 = 0;
    let tp1_l = p.tps_long.first().copied().unwrap_or(p.entree + r);
    let tp2_l = p.tps_long.get(1).copied().unwrap_or(p.entree + 2.0 * r);
    let tp3_l = p.tps_long.get(2).copied().unwrap_or(p.entree + 3.0 * r);
    let tp1_s = p.tps_short.first().copied().unwrap_or(p.entree - r);
    let tp2_s = p.tps_short.get(1).copied().unwrap_or(p.entree - 2.0 * r);
    let tp3_s = p.tps_short.get(2).copied().unwrap_or(p.entree - 3.0 * r);

    for b in &fenetre {
        let ts = b.timestamp.timestamp();
        if r_long.is_none() {
            if b.low <= p.sl_long {
                r_long = Some(-1.0);
                if ferme_le == 0 { ferme_le = ts; }
            } else if b.high >= tp3_l {
                r_long = Some(3.0);
                if ferme_le == 0 { ferme_le = ts; }
            } else if b.high >= tp2_l {
                r_long = Some(2.0);
                if ferme_le == 0 { ferme_le = ts; }
            } else if b.high >= tp1_l {
                r_long = Some(1.0);
                if ferme_le == 0 { ferme_le = ts; }
            }
        }
        if r_short.is_none() {
            if b.high >= p.sl_short {
                r_short = Some(-1.0);
                if ferme_le == 0 { ferme_le = ts; }
            } else if b.low <= tp3_s {
                r_short = Some(3.0);
                if ferme_le == 0 { ferme_le = ts; }
            } else if b.low <= tp2_s {
                r_short = Some(2.0);
                if ferme_le == 0 { ferme_le = ts; }
            } else if b.low <= tp1_s {
                r_short = Some(1.0);
                if ferme_le == 0 { ferme_le = ts; }
            }
        }
        // Les deux jambes fermées : la passe est terminée à ce bar.
        if r_long.is_some() && r_short.is_some() {
            break;
        }
    }

    // Time-stop : 60 min par défaut (params moteur) — si les deux jambes
    // ne sont pas mortes mais que le délai est passé, la passe expire.
    let time_stop = p.heure_entree + 3600;
    let maintenant = chrono::Utc::now().timestamp();
    if r_long.is_some() && r_short.is_some() {
        // Verdict net = somme des jambes (même convention que le moteur).
        // Les deux sont Some (condition du if) — map_or pour le preaudit.
        let net = r_long.map_or(0.0, |v| v) + r_short.map_or(0.0, |v| v);
        let (verdict, r_net) = if net > 1e-9 {
            ("tp2", net)
        } else if net < -1e-9 {
            ("sl", net)
        } else {
            ("be", 0.0)
        };
        let _ = sqlx::query(
            "UPDATE signaux SET statut = 'Fermé', verdict = ?, r_realise = ?, ferme_le = ?
             WHERE id = ?",
        )
        .bind(verdict)
        .bind(r_net)
        .bind(ferme_le)
        .bind(&p.id)
        .execute(db.pool())
        .await;
        tracing::info!(
            "📯 rattrapage_straddle {} fermé ({} net {:+.2}R, ouvert le {})",
            p.asset, verdict, r_net,
            chrono::DateTime::from_timestamp(p.heure_entree, 0)
                .map(|t| t.format("%d/%m %H:%M").to_string())
                .unwrap_or_default()
        );
    } else if maintenant > time_stop && (r_long.is_some() || r_short.is_some()) {
        // Time-stop : le moteur ferme les deux jambes au prix courant.
        // La jambe morte garde son R (TP acquis ou SL), la vivante est
        // fermée au dernier close connu de la fenêtre.
        let dernier_close = fenetre.last().map(|b| b.close).unwrap_or(p.entree);
        let r_vivante = if r_long.is_some() {
            // jambe SHORT vivante : R = (E − prix) / risque
            (p.entree - dernier_close) / r
        } else {
            // jambe LONG vivante : R = (prix − E) / risque
            (dernier_close - p.entree) / r
        };
        let r_net = r_long.or(r_short).unwrap_or(0.0) + r_vivante;
        let _ = sqlx::query(
            "UPDATE signaux SET statut = 'Fermé', verdict = 'expire', r_realise = ?, ferme_le = ?
             WHERE id = ?",
        )
        .bind(r_net)
        .bind(time_stop)
        .bind(&p.id)
        .execute(db.pool())
        .await;
        tracing::info!("📯 rattrapage_straddle {} expiré (net {:+.2}R)", p.asset, r_net);
    }
    // Les deux jambes vivantes OU une seule morte sans time-stop : la
    // position reste Actif (gérée au prochain passage — le runtime vivant
    // la prendra s'il revient dans le périmètre, sinon ce rattrapage).
    Ok(())
}
