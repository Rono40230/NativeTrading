CREATE TABLE IF NOT EXISTS smc_feedback (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    signal_id           TEXT    NOT NULL UNIQUE,
    asset               TEXT    NOT NULL,
    timeframe           TEXT    NOT NULL,
    timestamp_signal    INTEGER NOT NULL,
    categorie           TEXT    NOT NULL DEFAULT 'ob_seul',
    session_active      TEXT    NOT NULL DEFAULT 'Off',
    score_smc           REAL    NOT NULL,
    confiance_ml        REAL    NOT NULL DEFAULT 0.0,
    kill_zone_active    INTEGER NOT NULL DEFAULT 0,
    sweep_detecte       INTEGER NOT NULL DEFAULT 0,
    conviction_llm      INTEGER NOT NULL DEFAULT 0,
    atr14               REAL    NOT NULL DEFAULT 0.0,
    verdict             TEXT,
    pnl_r               REAL,
    gagnant             INTEGER,
    duree_trade_min     INTEGER,
    ferme_le            INTEGER,
    cree_le             INTEGER NOT NULL DEFAULT (unixepoch())
);
CREATE INDEX IF NOT EXISTS idx_smc_feedback_asset   ON smc_feedback(asset, timeframe, categorie);
CREATE INDEX IF NOT EXISTS idx_smc_feedback_verdict ON smc_feedback(verdict, gagnant);
CREATE INDEX IF NOT EXISTS idx_smc_feedback_cree_le ON smc_feedback(cree_le DESC);
