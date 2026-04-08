-- Ajout de pip_to_points : facteur de conversion pips → points MT5
-- Forex standard (5 décimales) : 10 | Métaux + Crypto (2 décimales) : 100
ALTER TABLE asset_params ADD COLUMN pip_to_points REAL NOT NULL DEFAULT 10.0;

-- Correction des assets à 2 décimales (pip = 1 unité price → 100 points MT5)
UPDATE asset_params SET pip_to_points = 100.0
WHERE asset IN ('XAUUSD', 'XAGUSD', 'BTC', 'ETH');
