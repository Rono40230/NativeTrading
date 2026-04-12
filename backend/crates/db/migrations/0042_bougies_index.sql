-- Index composite sur bougies pour accélérer les requêtes chart (asset + timeframe + timestamp).
-- Sans cet index, les requêtes WHERE asset=? AND timeframe=? AND source!='mt5'
-- effectuaient un full table scan → 4-6 secondes par requête.
CREATE INDEX IF NOT EXISTS idx_bougies_asset_tf_ts
    ON bougies (asset, timeframe, timestamp DESC);

-- Index partiel pour obtenir_bougies_chart (source non-mt5) si SQLite >= 3.8.9
CREATE INDEX IF NOT EXISTS idx_bougies_chart
    ON bougies (asset, timeframe, timestamp DESC)
    WHERE source != 'mt5';
