//! Appel REST IG GET /prices/{epic} pour l'historique des bougies.
#![allow(dead_code)]

// anyhow::Result supprimé (non utilisé)
use chrono::{DateTime, Utc};
use common::{Asset, Candle, Timeframe};
use data::providers::ig as ig_helpers;
use db::Database;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

use crate::ig_session::IgSession;

// ─── Structs de désérialisation JSON IG ──────────────────────────────────────

#[derive(serde::Deserialize)]
struct IgPrix {
    bid: Option<f64>,
    ask: Option<f64>,
}

impl IgPrix {
    fn mid(&self) -> Option<f64> {
        match (self.bid, self.ask) {
            (Some(b), Some(a)) => Some((b + a) / 2.0),
            (Some(b), None) => Some(b),
            (None, Some(a)) => Some(a),
            _ => None,
        }
    }
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct IgBougie {
    #[serde(rename = "snapshotTimeUTC")]
    snapshot_time_utc: String,
    open_price: IgPrix,
    high_price: IgPrix,
    low_price: IgPrix,
    close_price: IgPrix,
    last_traded_volume: Option<f64>,
}

#[derive(serde::Deserialize)]
struct IgResp {
    prices: Vec<IgBougie>,
}

// ─── Appel REST ──────────────────────────────────────────────────────────────

/// Appel REST IG GET /prices/{epic}?resolution={res}&max=200
pub(super) async fn fetch_rest_ig(
    ig_session: &Arc<Mutex<IgSession>>,
    db: &Arc<Database>,
    rest_403: &Arc<RwLock<HashSet<String>>>,
    asset: &Asset,
    timeframe: &Timeframe,
    max: usize,
) -> Vec<Candle> {
    let epic = match ig_helpers::epic_pour_asset(asset) {
        Some(e) => e,
        None => return vec![],
    };
    let resolution = ig_helpers::resolution_pour_tf(timeframe);

    let (url_base, client, headers) = {
        let mut sess = ig_session.lock().await;
        if !sess.est_connecte() {
            match sess.login(db).await {
                Ok(()) => {}
                Err(e) => {
                    tracing::warn!("IG REST historique: login échoué — {}", e);
                    return vec![];
                }
            }
        }
        let base = sess.url().to_string();
        let client = sess.client().clone();
        let hdrs = match sess.headers(db).await {
            Ok(h) => h,
            Err(e) => {
                tracing::warn!("IG REST historique: headers échoués — {}", e);
                return vec![];
            }
        };
        (base, client, hdrs)
    };

    let url = format!(
        "{}/prices/{}?resolution={}&max={}&pageSize=0",
        url_base, epic, resolution, max
    );

    let resp = match client
        .get(&url)
        .headers(headers)
        .header("Version", "3")
        .send()
        .await
    {
        Ok(r) if r.status().is_success() => r,
        Ok(r) => {
            let status = r.status();
            if status.as_u16() == 403 {
                let rest_key = format!("{:?}_{:?}", asset, timeframe);
                rest_403.write().await.insert(rest_key);
                tracing::debug!(
                    "IG REST historique: 403 pour {} — désactivé pour cette session",
                    asset.as_str()
                );
            } else {
                tracing::warn!(
                    "IG REST historique: HTTP {} pour {}",
                    status,
                    asset.as_str()
                );
            }
            return vec![];
        }
        Err(e) => {
            tracing::warn!("IG REST historique: réseau — {}", e);
            return vec![];
        }
    };

    let data = match resp.json::<IgResp>().await {
        Ok(d) => d,
        Err(e) => {
            tracing::warn!("IG REST historique: parse JSON — {}", e);
            return vec![];
        }
    };

    data.prices
        .into_iter()
        .filter_map(|b| {
            let ts = DateTime::parse_from_rfc3339(&format!("{}Z", b.snapshot_time_utc))
                .or_else(|_| DateTime::parse_from_rfc3339(&b.snapshot_time_utc))
                .ok()
                .map(|dt| dt.with_timezone(&Utc))?;
            Some(Candle {
                timestamp: ts,
                open: b.open_price.mid()?,
                high: b.high_price.mid()?,
                low: b.low_price.mid()?,
                close: b.close_price.mid()?,
                volume: b.last_traded_volume.unwrap_or(0.0),
            })
        })
        .collect()
}
