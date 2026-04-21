import re
with open('backend/crates/db/src/lib.rs', 'r') as f:
    text = f.read()

replacement = r'''
use sqlx::{sqlite::SqliteConnectOptions, ConnectOptions, SqlitePool};
'''

text = text.replace("use sqlx::{sqlite::SqliteConnectOptions, SqlitePool};", replacement)

replacement2 = r'''        let options = SqliteConnectOptions::from_str(&format!("sqlite:{}", chemin))
            .map_err(|e| TradingError::Database(e.to_string()))?
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .busy_timeout(std::time::Duration::from_secs(10))
            .disable_statement_logging();
'''

text = re.sub(
    r'\s+let options = SqliteConnectOptions::from_str\(&format!\("sqlite:\{\}", chemin\)\)\s+\.map_err\(\|e\| TradingError::Database\(e\.to_string\(\)\)\)\?\s+\.create_if_missing\(true\)\s+\.journal_mode\(sqlx::sqlite::SqliteJournalMode::Wal\)\s+\.busy_timeout\(std::time::Duration::from_secs\(10\)\);',
    replacement2, 
    text
)

with open('backend/crates/db/src/lib.rs', 'w') as f:
    f.write(text)
