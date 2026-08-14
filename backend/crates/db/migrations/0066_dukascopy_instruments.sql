-- 0066 — Mapping instruments du datafeed public Dukascopy.
--
-- `datafeed_dukascopy` : nom de l'instrument tel qu'attendu par
-- https://datafeed.dukascopy.com/datafeed/{INSTRUMENT}/{ANNEE}/{MOIS-1}/{JOUR}/BID_candles_min_1.bi5
--
-- Conventions vérifiées empiriquement (2026-08) :
-- - forex / métaux : nom plat (XAUUSD, EURUSD…) — vérifié HTTP 200 ;
-- - indices : nom CONCATÉNÉ sans point ni slash — les formes officielles
--   « USATECH.IDX/USD » ou « usatech.idx/usd » retournent 404, la forme
--   concaténée « USATECHIDXUSD » retourne 200 — vérifié pour USATECHIDXUSD,
--   DEUIDXEUR, USA500IDXUSD ;
-- - BTC → BTCUSD (vérifié 200).
--
-- Les indices non encore vérifiés (USA30/GBR/FRA/JPN) suivent la même règle
-- de concaténation ; si l'un retourne 404 systématiquement, le endpoint de
-- backfill le signale sans bloquer (champ `avertissement` de la réponse).

ALTER TABLE assets ADD COLUMN datafeed_dukascopy TEXT;

-- Métaux précieux (vérifiés)
UPDATE assets SET datafeed_dukascopy = 'XAUUSD'  WHERE id = 'XAUUSD';
UPDATE assets SET datafeed_dukascopy = 'XAGUSD'  WHERE id = 'XAGUSD';
UPDATE assets SET datafeed_dukascopy = 'XPTUSD'  WHERE id = 'XPTUSD';
UPDATE assets SET datafeed_dukascopy = 'XPDUSD'  WHERE id = 'XPDUSD';

-- Forex (noms identiques à l'id DB ; EURUSD vérifié)
UPDATE assets SET datafeed_dukascopy = 'EURUSD'  WHERE id = 'EURUSD';
UPDATE assets SET datafeed_dukascopy = 'GBPUSD'  WHERE id = 'GBPUSD';
UPDATE assets SET datafeed_dukascopy = 'USDCHF'  WHERE id = 'USDCHF';
UPDATE assets SET datafeed_dukascopy = 'AUDUSD'  WHERE id = 'AUDUSD';
UPDATE assets SET datafeed_dukascopy = 'USDCAD'  WHERE id = 'USDCAD';
UPDATE assets SET datafeed_dukascopy = 'NZDUSD'  WHERE id = 'NZDUSD';
UPDATE assets SET datafeed_dukascopy = 'USDJPY'  WHERE id = 'USDJPY';
UPDATE assets SET datafeed_dukascopy = 'GBPJPY'  WHERE id = 'GBPJPY';
UPDATE assets SET datafeed_dukascopy = 'CADJPY'  WHERE id = 'CADJPY';
UPDATE assets SET datafeed_dukascopy = 'NZDJPY'  WHERE id = 'NZDJPY';
UPDATE assets SET datafeed_dukascopy = 'EURJPY'  WHERE id = 'EURJPY';
UPDATE assets SET datafeed_dukascopy = 'EURGBP'  WHERE id = 'EURGBP';

-- Crypto (BTC → cotation Dukascopy, vérifié 200)
UPDATE assets SET datafeed_dukascopy = 'BTCUSD'  WHERE id = 'BTC';

-- Indices (règle de concaténation ; USATECH/DEU/USA500 vérifiés 200)
UPDATE assets SET datafeed_dukascopy = 'USATECHIDXUSD' WHERE id = 'NAS100';
UPDATE assets SET datafeed_dukascopy = 'DEUIDXEUR'     WHERE id = 'DAX';
UPDATE assets SET datafeed_dukascopy = 'USA500IDXUSD'  WHERE id = 'SP500';
UPDATE assets SET datafeed_dukascopy = 'USA30IDXUSD'   WHERE id = 'US30';
UPDATE assets SET datafeed_dukascopy = 'GBRIDXGBP'     WHERE id = 'FTSE100';
UPDATE assets SET datafeed_dukascopy = 'FRAIDXEUR'     WHERE id = 'CAC40';
UPDATE assets SET datafeed_dukascopy = 'JPNIDXJPY'     WHERE id = 'JP225';
