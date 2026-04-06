-- Calibration automatique des seuils de filtrage Rockets.
-- Stocke les seuils optimaux (score_scan, conviction_llm) par (phase, session)
-- calculés par grid search sur les feedbacks réels toutes les 6 heures.
CREATE TABLE IF NOT EXISTS rockets_calibration (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    phase           TEXT    NOT NULL,               -- 'breakout'|'prelancement'|'momentum'
    session         TEXT    NOT NULL DEFAULT 'all', -- 'london'|'ny'|'asia'|'all'
    score_min       INTEGER NOT NULL DEFAULT 65,    -- seuil score scan optimal
    conviction_min  INTEGER NOT NULL DEFAULT 65,    -- seuil conviction LLM optimal
    nb_trades       INTEGER NOT NULL DEFAULT 0,
    win_rate        REAL    NOT NULL DEFAULT 0.0,
    pnl_moyen_r     REAL    NOT NULL DEFAULT 0.0,
    fiabilite       TEXT    NOT NULL DEFAULT 'insuffisant',
    invalide        INTEGER NOT NULL DEFAULT 0,
    maj_le          INTEGER NOT NULL DEFAULT (unixepoch()),
    UNIQUE(phase, session)
);
CREATE INDEX IF NOT EXISTS idx_rockets_calib_phase
    ON rockets_calibration(phase, session);
