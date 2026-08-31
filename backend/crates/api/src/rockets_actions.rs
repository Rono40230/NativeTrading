//! Étape A Rockets actions (31/08) — énumération de l'univers US.
//!
//! Source : répertoire officiel NASDAQ Trader (2 fichiers plats gratuits,
//! sans clé) :
//!   - nasdaqlisted.txt  (cotées NASDAQ)
//!   - otherlisted.txt   (NYSE, AMEX, Arca, BATS, IEX)
//! Filtrage : actions communes uniquement (pas d'ETF, pas de symboles de
//! test, pas de preferreds/warrants/units — charset A-Z).
//! Les prix D1 viennent de Tiingo (clé en table `configuration`).

use actix_web::{web, HttpResponse, Responder};
use crate::state::AppState;

const URL_NASDAQ: &str = "https://www.nasdaqtrader.com/dynamic/SymDir/nasdaqlisted.txt";
const URL_OTHER: &str = "https://www.nasdaqtrader.com/dynamic/SymDir/otherlisted.txt";

// ── Filtrage pur (testé) ─────────────────────────────────────────────────────

/// Filtre le pipe-file nasdaqlisted.txt.
/// Champs : Symbol|Security Name|Market Category|Test Issue|Financial Status|Round Lot Size|ETF|NextShares
pub fn filtrer_nasdaq(txt: &str) -> Vec<db::univers_actions::TickerFiltre> {
    let mut out = Vec::new();
    for ligne in txt.lines().skip(1) {
        let c: Vec<&str> = ligne.split('|').collect();
        if c.len() < 8 {
            continue;
        }
        if garder(c[0], c[1], c[3], c[6]) {
            out.push(db::univers_actions::TickerFiltre {
                ticker: c[0].to_string(),
                nom: c[1].to_string(),
                exchange: "NASDAQ".into(),
            });
        }
    }
    out
}

/// Filtre le pipe-file otherlisted.txt.
/// Champs : ACT Symbol|Security Name|Exchange|CQS Symbol|ETF|Round Lot Size|Test Issue|NASDAQ Symbol
pub fn filtrer_other(txt: &str) -> Vec<db::univers_actions::TickerFiltre> {
    let mut out = Vec::new();
    for ligne in txt.lines().skip(1) {
        let c: Vec<&str> = ligne.split('|').collect();
        if c.len() < 8 {
            continue;
        }
        if garder(c[0], c[1], c[6], c[4]) {
            out.push(db::univers_actions::TickerFiltre {
                ticker: c[0].to_string(),
                nom: c[1].to_string(),
                exchange: nom_exchange(c[2]),
            });
        }
    }
    out
}

/// Commun aux deux fichiers : ticker A-Z uniquement, pas d'ETF, pas de
/// symbole de test, nom sans mentions de produits dérivés.
fn garder(ticker: &str, nom: &str, test_issue: &str, etf: &str) -> bool {
    if test_issue.trim() != "N" || etf.trim() != "N" {
        return false; // symbole de test ou ETF/produit tract
    }
    if ticker.is_empty() || !ticker.chars().all(|ch| ch.is_ascii_uppercase()) {
        return false; // preferreds ($), warrants, units, classes spéciales…
    }
    let n = nom.to_uppercase();
    !(n.contains("WARRANT")
        || n.contains("RIGHT")
        || n.contains("UNIT")
        || n.contains("PREFERRED")
        || n.contains("DEPOSITARY"))
}

fn nom_exchange(code: &str) -> String {
    match code.trim() {
        "N" => "NYSE".into(),
        "A" => "AMEX".into(),
        "P" => "NYSE_ARCA".into(),
        "Z" => "BATS".into(),
        "V" => "IEX".into(),
        _ => "AUTRE".into(),
    }
}

// ── Endpoints ────────────────────────────────────────────────────────────────

/// POST /api/rockets/actions/univers/charger — télécharge le répertoire
/// NASDAQ Trader, filtre, upsert l'univers. Le cure propriétaire ('exclu')
/// est préservé. Idempotent, peut être relancé à la main.
pub async fn charger_univers(state: web::Data<AppState>) -> impl Responder {
    let client = &*crate::http_client::HTTP_CLIENT;
    let (nasdaq, other) = match (
        client.get(URL_NASDAQ).send().await,
        client.get(URL_OTHER).send().await,
    ) {
        (Ok(a), Ok(b)) => (a, b),
        (Err(e), _) | (_, Err(e)) => {
            return HttpResponse::BadGateway()
                .json(serde_json::json!({ "error": format!("NASDAQ Trader injoignable: {e}") }))
        }
    };
    let txt_nasdaq = match nasdaq.text().await {
        Ok(t) => t,
        Err(e) => {
            return HttpResponse::BadGateway()
                .json(serde_json::json!({ "error": e.to_string() }))
        }
    };
    let txt_other = match other.text().await {
        Ok(t) => t,
        Err(e) => {
            return HttpResponse::BadGateway()
                .json(serde_json::json!({ "error": e.to_string() }))
        }
    };

    let mut lignes = filtrer_nasdaq(&txt_nasdaq);
    lignes.extend(filtrer_other(&txt_other));

    match state.db.maj_univers_actions(&lignes).await {
        Ok(n) => {
            tracing::info!("🚀 Univers actions : {n} tickers chargés (NASDAQ Trader)");
            HttpResponse::Ok().json(serde_json::json!({ "charges": n }))
        }
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({ "error": e.to_string() })),
    }
}

/// GET /api/rockets/actions/univers — l'univers actif.
pub async fn lire_univers(state: web::Data<AppState>) -> impl Responder {
    match state.db.univers_actions_actives().await {
        Ok(l) => HttpResponse::Ok().json(serde_json::json!({ "total": l.len(), "tickers": l })),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({ "error": e.to_string() })),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NASDAQ_ECH: &str = "Symbol|Security Name|Market Category|Test Issue|Financial Status|Round Lot Size|ETF|NextShares\n\
        AAPL|Apple Incorporated|Q|N|N|100|N|N\n\
        TSLA|Tesla, Inc. Common Stock|Q|N|N|100|N|N\n\
        ZVZZT|Nasdaq Test Symbol|Q|Y|N|100|N|N\n\
        SPY|SPDR S&P 500 ETF Trust|Q|N|N|100|Y|N\n\
        ABC$.|Preferred Test|Q|N|N|100|N|N\n\
        WKHSW|Workhorse Group Inc. Warrant|Q|N|N|100|N|N\n";

    const OTHER_ECH: &str = "ACT Symbol|Security Name|Exchange|CQS Symbol|ETF|Round Lot Size|Test Issue|NASDAQ Symbol\n\
        A|Agilent Technologies, Inc. Common Stock|N|A|N|100|N|A\n\
        BRK.B|Berkshire Hathaway Inc. Class B|N|BRK/B|N|100|N|BRKB\n\
        IBM|International Business Machine|N|IBM|N|100|N|IBM\n";

    #[test]
    fn filtre_nasdaq_garde_les_communes() {
        let v = filtrer_nasdaq(NASDAQ_ECH);
        let tickers: Vec<&str> = v.iter().map(|t| t.ticker.as_str()).collect();
        assert!(tickers.contains(&"AAPL"));
        assert!(tickers.contains(&"TSLA"));
    }

    #[test]
    fn filtre_nasdaq_exclut_tests_etf_et_speciaux() {
        let v = filtrer_nasdaq(NASDAQ_ECH);
        let tickers: Vec<&str> = v.iter().map(|t| t.ticker.as_str()).collect();
        assert!(!tickers.contains(&"ZVZZT"), "symbole de test");
        assert!(!tickers.contains(&"SPY"), "ETF");
        assert!(!tickers.contains(&"ABC$."), "preferred");
        assert!(!tickers.contains(&"WKHSW"), "warrant");
    }

    #[test]
    fn filtre_other_garde_communes_exclut_classes_multiples() {
        let v = filtrer_other(OTHER_ECH);
        assert_eq!(v.len(), 2); // A et IBM ; BRK.B exclu (point = classe)
        assert_eq!(v[0].exchange, "NYSE");
        assert_eq!(v[0].ticker, "A");
    }

    #[test]
    fn mapping_exchange() {
        assert_eq!(nom_exchange("A"), "AMEX");
        assert_eq!(nom_exchange("P"), "NYSE_ARCA");
        assert_eq!(nom_exchange("X"), "AUTRE");
    }
}
