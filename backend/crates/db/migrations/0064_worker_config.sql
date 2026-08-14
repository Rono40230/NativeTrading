-- 0064 — Pilotage du data pipeline depuis l'UI : mapping workers + config.
--
-- Nouvelles colonnes de routing sur `assets` :
--   symbol_bybit : symbole Bybit linear (topics kline du worker WS)
--   epic_ig      : epic IG Markets (worker REST /prices/{epic})
-- Un asset est ingéré par un worker si et seulement si sa colonne de mapping
-- est renseignée (et qu'il est actif) — plus aucun hardcoding côté Rust.

ALTER TABLE assets ADD COLUMN symbol_bybit TEXT;
ALTER TABLE assets ADD COLUMN epic_ig TEXT;

-- Les métaux sont ingérés via Bybit WS (linear XAUUSDT/XAGUSDT) : aligner la
-- source avec le routing réel (le filtre du worker Bybit est source='binance').
UPDATE assets SET source = 'binance' WHERE id IN ('XAUUSD', 'XAGUSD');

-- Pré-remplissage (assets actifs existants)
UPDATE assets SET symbol_bybit = 'BTCUSDT' WHERE id = 'BTC';
UPDATE assets SET symbol_bybit = 'ETHUSDT' WHERE id = 'ETH';
UPDATE assets SET symbol_bybit = 'SOLUSDT' WHERE id = 'SOL';
UPDATE assets SET symbol_bybit = 'BNBUSDT' WHERE id = 'BNB';
UPDATE assets SET symbol_bybit = 'XRPUSDT' WHERE id = 'XRP';
UPDATE assets SET symbol_bybit = 'ADAUSDT' WHERE id = 'ADA';
UPDATE assets SET symbol_bybit = 'DOGEUSDT' WHERE id = 'DOGE';
UPDATE assets SET symbol_bybit = 'AVAXUSDT' WHERE id = 'AVAX';
UPDATE assets SET symbol_bybit = 'LINKUSDT' WHERE id = 'LINK';
UPDATE assets SET symbol_bybit = 'DOTUSDT' WHERE id = 'DOT';
UPDATE assets SET symbol_bybit = 'XAUUSDT' WHERE id = 'XAUUSD';
UPDATE assets SET symbol_bybit = 'XAGUSDT' WHERE id = 'XAGUSD';
UPDATE assets SET epic_ig = 'CS.D.EURUSD.CFD.IP' WHERE id = 'EURUSD';
UPDATE assets SET epic_ig = 'CS.D.GBPJPY.CFD.IP' WHERE id = 'GBPJPY';
UPDATE assets SET epic_ig = 'CS.D.USDJPY.CFD.IP' WHERE id = 'USDJPY';
UPDATE assets SET epic_ig = 'CS.D.GBPUSD.CFD.IP' WHERE id = 'GBPUSD';
UPDATE assets SET epic_ig = 'CS.D.USDCHF.CFD.IP' WHERE id = 'USDCHF';
UPDATE assets SET epic_ig = 'CS.D.AUDUSD.CFD.IP' WHERE id = 'AUDUSD';
UPDATE assets SET epic_ig = 'CS.D.USDCAD.CFD.IP' WHERE id = 'USDCAD';
UPDATE assets SET epic_ig = 'CS.D.NZDUSD.CFD.IP' WHERE id = 'NZDUSD';
UPDATE assets SET epic_ig = 'CS.D.CADJPY.CFD.IP' WHERE id = 'CADJPY';
UPDATE assets SET epic_ig = 'CS.D.NZDJPY.CFD.IP' WHERE id = 'NZDJPY';
UPDATE assets SET epic_ig = 'CS.D.EURJPY.CFD.IP' WHERE id = 'EURJPY';
UPDATE assets SET epic_ig = 'CS.D.EURGBP.CFD.IP' WHERE id = 'EURGBP';
UPDATE assets SET epic_ig = 'IX.D.DAX.IFD.IP' WHERE id = 'DAX';
UPDATE assets SET epic_ig = 'IX.D.NASDAQ.IFD.IP' WHERE id = 'NAS100';
UPDATE assets SET epic_ig = 'IX.D.SPTRD.IFD.IP' WHERE id = 'SP500';
UPDATE assets SET epic_ig = 'IX.D.DOW.IFD.IP' WHERE id = 'US30';
UPDATE assets SET epic_ig = 'IX.D.FTSE.IFD.IP' WHERE id = 'FTSE100';
UPDATE assets SET epic_ig = 'IX.D.CAC.IFD.IP' WHERE id = 'CAC40';
UPDATE assets SET epic_ig = 'IX.D.NIKKEI.IFD.IP' WHERE id = 'JP225';

-- Config worker par défaut (maj_le est NOT NULL sans défaut → fourni explicitement)
INSERT OR REPLACE INTO configuration (cle, valeur, maj_le) VALUES ('worker_timeframes', '["M5","M15","H1","D1"]', strftime('%s','now'));
INSERT OR REPLACE INTO configuration (cle, valeur, maj_le) VALUES ('worker_historique_mois', '6', strftime('%s','now'));
INSERT OR REPLACE INTO configuration (cle, valeur, maj_le) VALUES ('worker_actif_bybit', '1', strftime('%s','now'));
INSERT OR REPLACE INTO configuration (cle, valeur, maj_le) VALUES ('worker_actif_ig', '1', strftime('%s','now'));
