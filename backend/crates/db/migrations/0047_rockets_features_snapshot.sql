-- Snapshot des 52 features ML calculées au moment de l'émission de chaque signal Rockets.
-- Permet le fine-tuning stratégie-spécifique (P3) : relier features → label réel (TP/SL).
CREATE TABLE IF NOT EXISTS rockets_features_snapshot (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    signal_id   INTEGER NOT NULL UNIQUE REFERENCES rockets_signaux(id) ON DELETE CASCADE,
    ticker      TEXT NOT NULL,
    cree_le     TEXT NOT NULL DEFAULT (datetime('now')),
    -- Vec<f64> sérialisé en JSON compact : [f1, f2, ..., f52]
    features_json TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_rfs_signal_id ON rockets_features_snapshot(signal_id);
CREATE INDEX IF NOT EXISTS idx_rfs_ticker    ON rockets_features_snapshot(ticker);
