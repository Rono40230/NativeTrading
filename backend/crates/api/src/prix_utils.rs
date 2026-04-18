#![allow(unused_variables, dead_code)]
//! Fetch de prix spot dispatché par type d'asset.
//! Crypto → Binance | Métaux / Forex / Indices → IG Markets REST
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

use crate::ig_session::IgSession;

// ── Désérialisation Binance ──────────────────────────────────────────────────

#[derive(serde::Deserialize)]
struct BinancePrix {
    price: String,
}

// ── Désérialisation IG Markets (prix snapshot) ───────────────────────────────

#[derive(serde::Deserialize)]
struct IgSnapshotPrix {
    bid: Option<f64>,
    offer: Option<f64>,
}

#[derive(serde::Deserialize)]
struct IgPrixResponse {
    snapshot: IgSnapshotPrix,
}

// ── Mapping asset → symbole Binance ──────────────────────────────────────────

fn binance_symbol(asset: &str) -> Option<&'static str> {
    match asset {
        "BTC" => Some("BTCUSDT"),
        "ETH" => Some("ETHUSDT"),
        "SOL" => Some("SOLUSDT"),
        "BNB" => Some("BNBUSDT"),
        "XRP" => Some("XRPUSDT"),
        "ADA" => Some("ADAUSDT"),
        "DOGE" => Some("DOGEUSDT"),
        "AVAX" => Some("AVAXUSDT"),
        "LINK" => Some("LINKUSDT"),
        "DOT" => Some("DOTUSDT"),
        _ => None,
    }
}

/// Epic IG pour un asset (string brut).
pub fn ig_epic_str(asset: &str) -> Option<&'static str> {
    match asset {
        "EURUSD" => Some("CS.D.EURUSD.CFD.IP"),
        "GBPUSD" => Some("CS.D.GBPUSD.CFD.IP"),
        "USDJPY" => Some("CS.D.USDJPY.CFD.IP"),
        "USDCHF" => Some("CS.D.USDCHF.CFD.IP"),
        "AUDUSD" => Some("CS.D.AUDUSD.CFD.IP"),
        "USDCAD" => Some("CS.D.USDCAD.CFD.IP"),
        "NZDUSD" => Some("CS.D.NZDUSD.CFD.IP"),
        "GBPJPY" => Some("CS.D.GBPJPY.CFD.IP"),
        "CADJPY" => Some("CS.D.CADJPY.CFD.IP"),
        "NZDJPY" => Some("CS.D.NZDJPY.CFD.IP"),
        "EURJPY" => Some("CS.D.EURJPY.CFD.IP"),
        "EURGBP" => Some("CS.D.EURGBP.CFD.IP"),
        "DAX" => Some("IX.D.DAX.IFD.IP"),
        "NAS100" => Some("IX.D.NASDAQ.IFD.IP"),
        "SP500" => Some("IX.D.SPTRD.IFD.IP"),
        "US30" => Some("IX.D.DOW.IFD.IP"),
        "FTSE100" => Some("IX.D.FTSE.IFD.IP"),
        "CAC40" => Some("IX.D.CAC.IFD.IP"),
        "JP225" => Some("IX.D.NIKKEI.IFD.IP"),
        _ => None,
    }
}

// ── Fonctions fetch internes ─────────────────────────────────────────────────

/// Fetch prix spot Binance.
pub async fn fetch_binance(client: &reqwest::Client, symbole: &str) -> Option<f64> {
    let sym = if symbole.ends_with("USDT") {
        symbole.to_string()
    } else {
        format!("{}USDT", symbole)
    };
    let url = format!(
        "https://api.binance.com/api/v3/ticker/price?symbol={}",
        sym
    );
    let resp: BinancePrix = client.get(&url).send().await.ok()?.json().await.ok()?;
    resp.price.parse::<f64>().ok()
}

/// Fetch prix spot IG via GET /markets/{epic} (snapshot bid/offer).
async fn fetch_ig(
    client: &reqwest::Client,
    session: &Arc<Mutex<IgSession>>,
    db: &Arc<db::Database>,
    epic: &str,
) -> Option<f64> {
    // try_lock : si le mutex est tenu par le login Lightstreamer (~25s), on skip ce tick
    // plutôt que de bloquer et créer un burst de mises à jour en rafale après
    let (url_base, headers) = {
        let mut sess = match session.try_lock() {
            Ok(s) => s,
            Err(_) => return None,
        };
        let url_base = sess.url();
        let headers = sess.headers(db).await.ok()?;
        (url_base, headers)
    };
    let url = format!("{}/markets/{}", url_base, epic);
    let resp: IgPrixResponse = client
        .get(&url)
        .headers(headers)
        .header("Version", "1")
        .send()
        .await
        .ok()?
        .json()
        .await
        .ok()?;
    let mut p = match (resp.snapshot.bid, resp.snapshot.offer) {
        (Some(b), Some(o)) => Some((b + o) / 2.0),
        (Some(b), None) => Some(b),
        (None, Some(o)) => Some(o),
        _ => None,
    };
    if let Some(ref mut price) = p {
        if epic == "CS.D.CFDSILVER.CFDSI.IP" {
            *price /= 100.0;
        }
    }
    p
}

// ── API publique ─────────────────────────────────────────────────────────────

/// Retourne le prix spot d'un asset selon sa source :
/// crypto → Binance | métaux / forex / indices → IG Markets REST.
/// Retourne `None` si l'asset est inconnu ou si la source est inaccessible.
pub async fn fetch_prix_asset(
    client: &reqwest::Client,
    asset: &str,
    ig: &Arc<Mutex<IgSession>>,
    db: &Arc<db::Database>,
) -> Option<f64> {
    if let Some(sym) = binance_symbol(asset) {
        return fetch_binance(client, sym).await;
    }
    if let Some(epic) = ig_epic_str(asset) {
        return fetch_ig(client, ig, db, epic).await;
    }
    // Fallback : tout asset inconnu est tenté comme paire USDT sur Binance (FRONT, RNDR, etc.)
    let sym = format!("{}USDT", asset);
    fetch_binance(client, &sym).await
}

/// Retourne `true` si l'asset est géré par IG Markets (pas Binance).
pub fn est_asset_ig(asset: &str) -> bool {
    ig_epic_str(asset).is_some()
}

/// Dernier close connu en DB pour un asset (fallback quand IG REST est inaccessible).
/// Requête ultra-rapide grâce à l'index (asset, timeframe, timestamp DESC).
pub async fn dernier_prix_db(asset: &str, db: &Arc<db::Database>) -> Option<f64> {
    use common::{Asset, Timeframe};
    let a = Asset::try_from(asset).ok()?;
    // M1 = bougies les plus récentes, sinon M5
    for tf in [Timeframe::M1, Timeframe::M5] {
        if let Ok(bougies) = db.obtenir_bougies(&a, &tf, 1).await {
            if let Some(b) = bougies.last() {
                return Some(b.close);
            }
        }
    }
    None
}

/// Crée un client HTTP réutilisable avec timeout 1.5s.
/// Doit être inférieur à l'intervalle de polling (2s WS, 3s dashboard).
pub fn client_http() -> Result<reqwest::Client, reqwest::Error> {
    reqwest::Client::builder()
        .timeout(Duration::from_millis(1500))
        .build()
}

pub async fn fetch_ig_multi(
    client: &reqwest::Client,
    session: &Arc<Mutex<IgSession>>,
    db: &Arc<db::Database>,
    epics: &[&str],
) -> std::collections::HashMap<String, f64> {
    let mut results = std::collections::HashMap::new();
    for epic in epics {
        if let Some(p) = fetch_ig(client, session, db, epic).await {
            results.insert(epic.to_string(), p);
        }
    }
    results
}
