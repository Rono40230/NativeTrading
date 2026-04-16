-- Snapshot des 56 features ML calculées au moment de l'émission de chaque signal Straddle.
-- 52 features OHLCV standard + 4 features de contexte Straddle :
--   ratio_atr (f64), categorie (encodée), session_active (encodée), score_llm (f64)
-- FK sur signaux.id (UUID TEXT) — ON DELETE CASCADE pour éviter les orphelins.
CREATE TABLE IF NOT EXISTS straddle_features_snapshot (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    signal_id     TEXT    NOT NULL UNIQUE REFERENCES signaux(id) ON DELETE CASCADE,
    ticker        TEXT    NOT NULL,
    cree_le       TEXT    NOT NULL DEFAULT (datetime('now')),
    -- Vec<f64> 56 éléments sérialisé en JSON compact
    features_json TEXT    NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_sfs_signal_id ON straddle_features_snapshot(signal_id);
CREATE INDEX IF NOT EXISTS idx_sfs_ticker    ON straddle_features_snapshot(ticker);
