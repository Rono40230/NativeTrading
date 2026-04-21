use std::env;
use std::time::Duration;
use dotenvy::dotenv;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    tracing_subscriber::fmt::init();

    let db_url = env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:../data/trading.db".to_string());
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(2)
        .connect(&db_url)
        .await?;

    let rows = sqlx::query(
        "SELECT id, ticker, cree_le
         FROM rockets_signaux
         WHERE statut = 'ferme'
           AND verdict IS NOT NULL
           AND id NOT IN (SELECT signal_id FROM rockets_features_snapshot)"
    )
    .fetch_all(&pool)
    .await?;

    println!("{} trades orphelins à backfiller...", rows.len());

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;

    let mut backfilled = 0;
    let mut err_data = 0;

    for row in rows {
        use sqlx::Row;
        let signal_id: i64 = row.get("id");
        let ticker: String = row.get("ticker");
        let cree_le_str: String = row.get("cree_le");
        let parsed_date = chrono::NaiveDateTime::parse_from_str(&cree_le_str, "%Y-%m-%d %H:%M:%S")?;
        let ts_ms = parsed_date.and_utc().timestamp_millis();

        let url = format!(
            "https://api.binance.com/api/v3/klines?symbol={}USDT&interval=1h&limit=150&endTime={}",
            ticker, ts_ms
        );

        let res = client.get(&url).send().await;
        if res.is_err() {
            println!("Id {} ({}): Erreur réseau", signal_id, ticker);
            err_data += 1;
            continue;
        }

        let raw: Result<Vec<Vec<serde_json::Value>>, _> = res.unwrap().json().await;
        let raw = match raw {
            Ok(data) => data,
            Err(_e) => {
                println!("Id {} ({}): Erreur JSON/Binance (peut-être delisté ?)", signal_id, ticker);
                err_data += 1;
                continue;
            }
        };

        if raw.len() < 60 {
            err_data += 1;
            println!("Id {} ({}): {} bougies H1 trouvées < 60", signal_id, ticker, raw.len());
            continue;
        }

        let parse_f64 = |val: &serde_json::Value| -> f64 {
            if let Some(s) = val.as_str() {
                s.parse().unwrap_or(0.0)
            } else if let Some(f) = val.as_f64() {
                f
            } else if let Some(i) = val.as_i64() {
                i as f64
            } else {
                0.0
            }
        };

        let mut bougies = Vec::with_capacity(raw.len());
        // L'API Binance retourne dans l'ordre chronologique (ASC), extraire_features attend ASC
        for r in raw {
            if r.len() < 6 { continue; }
            let ts_open = r[0].as_i64().unwrap_or(0);
            
            bougies.push(common::Candle {
                timestamp: chrono::DateTime::from_timestamp_millis(ts_open).unwrap_or_default(),
                open: parse_f64(&r[1]),
                high: parse_f64(&r[2]),
                low: parse_f64(&r[3]),
                close: parse_f64(&r[4]),
                volume: parse_f64(&r[5]),
            });
        }

        if let Some(features) = ml::features::extraire_features(&bougies) {
            let json = serde_json::to_string(&features)?;
            sqlx::query(
                "INSERT INTO rockets_features_snapshot (signal_id, ticker, features_json)
                 VALUES (?, ?, ?)"
            )
            .bind(signal_id)
            .bind(&ticker)
            .bind(&json)
            .execute(&pool)
            .await?;
            backfilled += 1;
            // Anti rate limit
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        } else {
            err_data += 1;
            println!("Id {}: impossible d'extraire les features (ex: NaNs)", signal_id);
        }
    }

    println!("\nTerminé ! {} backfilled, {} ignorés (manque data)", backfilled, err_data);

    Ok(())
}
