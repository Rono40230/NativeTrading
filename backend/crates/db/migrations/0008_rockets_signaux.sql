CREATE TABLE IF NOT EXISTS rockets_signaux (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    ticker       TEXT    NOT NULL,
    phase        TEXT    NOT NULL,
    score        INTEGER NOT NULL,
    prix_entree  REAL    NOT NULL,
    stop_loss    REAL    NOT NULL,
    target       REAL    NOT NULL,
    ratio_volume REAL    NOT NULL,
    atr_ratio    REAL    NOT NULL,
    rsi          REAL    NOT NULL,
    verdict      TEXT,
    prix_verdict REAL,
    cree_le      TEXT    NOT NULL DEFAULT (datetime('now')),
    maj_le       TEXT
);

CREATE INDEX IF NOT EXISTS idx_rockets_ticker  ON rockets_signaux(ticker);
CREATE INDEX IF NOT EXISTS idx_rockets_verdict ON rockets_signaux(verdict);
