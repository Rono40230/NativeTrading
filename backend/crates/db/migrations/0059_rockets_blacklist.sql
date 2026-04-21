-- Tickers blacklistés du scan Rockets (asset dynamiques Binance, pas dans la table assets).
-- Un ticker blacklisté est ignoré par le scan avant toute inférence ML.
CREATE TABLE IF NOT EXISTS rockets_blacklist (
    ticker      TEXT    NOT NULL PRIMARY KEY,
    raison      TEXT    NOT NULL,
    banni_le    TEXT    NOT NULL DEFAULT (datetime('now'))
);

-- FRONT : prix figé à 0.88 USDT depuis plusieurs semaines → doublon permanent.
INSERT OR IGNORE INTO rockets_blacklist (ticker, raison)
VALUES ('FRONT', 'Prix figé 0.88 USDT depuis plusieurs semaines — doublon permanent en scan');
