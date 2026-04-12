-- Migration 0041 : Ajout colonne source dans bougies
-- Valeurs : 'rest_ig' | 'lightstreamer' | 'mt5' | 'binance'
-- Les charts n'affichent que rest_ig + lightstreamer + binance (jamais mt5)
ALTER TABLE bougies ADD COLUMN source TEXT NOT NULL DEFAULT 'unknown';

-- Marquer les bougies existantes comme mt5 si IG asset (forex/metals/indices)
-- Les bougies crypto restent 'unknown' car elles viennent de Binance REST (déjà correctes)
UPDATE bougies SET source = 'mt5'
WHERE asset IN (
    'EURUSD','GBPUSD','USDJPY','USDCHF','AUDUSD','USDCAD','NZDUSD',
    'GBPJPY','CADJPY','NZDJPY','EURJPY','EURGBP',
    'XAUUSD','XAGUSD','XPTUSD','XPDUSD',
    'DAX','NAS100','SP500','US30','FTSE100','CAC40','JP225'
)
AND source = 'unknown';

-- Les bougies crypto existantes : source binance
UPDATE bougies SET source = 'binance'
WHERE asset IN ('BTC','ETH','SOL','BNB','XRP','ADA','DOGE','AVAX','LINK','DOT')
AND source = 'unknown';
