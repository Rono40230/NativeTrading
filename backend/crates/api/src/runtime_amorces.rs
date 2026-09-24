//! Amorces et annonces du runtime — extraits de runtime_tick.rs (limite
//! 600 lignes, pre-commit). Amorce MTF : contexte H1→MN du moteur v12.
//! Annonces tier 1 : calendrier High → rail straddle.

use common::Asset;
pub(crate) async fn annonces_tier1(db: &db::Database) -> Vec<straddle::Annonce> {
    let Ok(rows) = db.lire_calendrier_cache(6 * 3600).await else {
        return Vec::new();
    };
    let maintenant = chrono::Utc::now().timestamp();
    rows.iter()
        .filter_map(|r| {
            let impact = r.get("impact").and_then(|v| v.as_str()).unwrap_or("");
            if impact != "High" {
                return None;
            }
            let dh = r.get("date_heure").and_then(|v| v.as_str())?;
            let ts = chrono::DateTime::parse_from_rfc3339(dh)
                .or_else(|_| {
                    chrono::DateTime::parse_from_rfc3339(&format!(
                        "{}:{}",
                        dh[..dh.len() - 2].to_string(),
                        &dh[dh.len() - 2..]
                    ))
                })
                .ok()?
                .timestamp();
            if ts <= maintenant || ts > maintenant + 7 * 24 * 3600 {
                return None;
            }
            Some(straddle::Annonce {
                ts,
                devise: r.get("devise").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                titre: r.get("titre").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            })
        })
        .collect()
}

pub(crate) async fn charger_amorce_mtf_runtime(db: &db::Database, asset: &Asset) -> smc::v12::AmorceMtf {
    use common::Timeframe;
    use smc::v12::{agreger_mensuel, AmorceMtf, BarInput};
    const MAX_BARS: i64 = 600;

    let vers_bars = |bougies: Vec<common::Candle>| -> Vec<BarInput> {
        bougies
            .into_iter()
            .map(|b| BarInput {
                timestamp: b.timestamp.timestamp(),
                open: b.open,
                high: b.high,
                low: b.low,
                close: b.close,
                volume: b.volume,
            })
            .collect()
    };
    let charger = |tf: Timeframe| async move {
        vers_bars(
            db.obtenir_bougies(asset, &tf, MAX_BARS)
                .await
                .unwrap_or_default(),
        )
    };
    let (h1, h4) = tokio::join!(charger(Timeframe::H1), charger(Timeframe::H4));
    // D1 profond (2000) : agrégée en MN pour la confluence +6.
    let d1 = vers_bars(
        db.obtenir_bougies(asset, &Timeframe::D1, 2000)
            .await
            .unwrap_or_default(),
    );
    let w1 = charger(Timeframe::W1).await;
    AmorceMtf {
        h1,
        h4,
        w1,
        mn: agreger_mensuel(&d1),
    }
}
