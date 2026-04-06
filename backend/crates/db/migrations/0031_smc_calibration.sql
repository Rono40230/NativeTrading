CREATE TABLE IF NOT EXISTS smc_calibration (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    asset            TEXT    NOT NULL,
    timeframe        TEXT    NOT NULL,
    categorie        TEXT    NOT NULL,
    score_smc_seuil  REAL    NOT NULL DEFAULT 70.0,
    conviction_seuil INTEGER NOT NULL DEFAULT 70,
    nb_trades        INTEGER NOT NULL DEFAULT 0,
    win_rate         REAL    NOT NULL DEFAULT 0.0,
    pnl_moyen_r      REAL    NOT NULL DEFAULT 0.0,
    fiabilite        TEXT    NOT NULL DEFAULT 'insuffisant',
    invalide         INTEGER NOT NULL DEFAULT 0,
    maj_le           INTEGER NOT NULL DEFAULT (unixepoch()),
    UNIQUE(asset, timeframe, categorie)
);
CREATE INDEX IF NOT EXISTS idx_smc_calibration_asset ON smc_calibration(asset, timeframe, categorie);
