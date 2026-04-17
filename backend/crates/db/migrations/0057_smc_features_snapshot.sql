-- Snapshot des features ML au moment d'émission d'un signal SMC Directionnel.
-- Pattern identique à straddle_features_snapshot (migration 0053).
CREATE TABLE IF NOT EXISTS smc_features_snapshot (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    signal_id     TEXT    NOT NULL UNIQUE REFERENCES signaux(id),
    ticker        TEXT    NOT NULL,
    cree_le       TEXT    NOT NULL DEFAULT (datetime('now')),
    features_json TEXT    NOT NULL
);
