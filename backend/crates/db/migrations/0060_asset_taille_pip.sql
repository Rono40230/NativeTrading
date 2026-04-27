-- Ajout de taille_pip : la taille d'un pip en variation de prix (ex: 0.0001 pour EURUSD, 0.01 pour USDJPY)
ALTER TABLE asset_params ADD COLUMN taille_pip REAL NOT NULL DEFAULT 0.0001;

UPDATE asset_params SET taille_pip = 0.01 WHERE asset LIKE '%JPY';
UPDATE asset_params SET taille_pip = 1.0 WHERE asset IN ('BTC', 'ETH', 'DAX', 'SP500', 'XAUUSD', 'XAGUSD');
