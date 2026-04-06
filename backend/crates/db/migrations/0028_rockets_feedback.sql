-- Mémoire apprenante de la stratégie Rockets.
-- Chaque signal Rockets est enregistré à sa création (verdict=NULL).
-- Le worker `rockets_suivi` met à jour le verdict lors de la clôture TP/SL.
CREATE TABLE IF NOT EXISTS rockets_feedback (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    signal_id       INTEGER NOT NULL UNIQUE,        -- FK → rockets_signaux.id
    ticker          TEXT    NOT NULL,
    phase           TEXT    NOT NULL,               -- 'breakout'|'prelancement'|'momentum'
    session_active  TEXT    NOT NULL DEFAULT 'Off', -- session au moment du signal
    timestamp_signal INTEGER NOT NULL,
    score_scan      INTEGER NOT NULL,               -- score /100 au moment du scan
    conviction_llm  INTEGER NOT NULL,               -- conviction /100 retournée par LLM
    ratio_volume    REAL    NOT NULL DEFAULT 0.0,
    atr_ratio       REAL    NOT NULL DEFAULT 0.0,
    rsi             REAL    NOT NULL DEFAULT 50.0,
    verdict         TEXT,                           -- 'tp1'|'tp2'|'tp3'|'sl'|'expire'
    pnl_r           REAL,                           -- R multiple réalisé
    gagnant         INTEGER,                        -- 1|0|NULL
    duree_trade_min INTEGER,
    ferme_le        INTEGER,
    cree_le         INTEGER NOT NULL DEFAULT (unixepoch())
);
CREATE INDEX IF NOT EXISTS idx_rockets_feedback_ticker
    ON rockets_feedback(ticker, phase);
CREATE INDEX IF NOT EXISTS idx_rockets_feedback_verdict
    ON rockets_feedback(verdict, gagnant);
CREATE INDEX IF NOT EXISTS idx_rockets_feedback_cree_le
    ON rockets_feedback(cree_le DESC);
