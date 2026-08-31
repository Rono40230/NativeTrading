-- Étape A Rockets actions (31/08) : veille actions US en Observation.
-- Décision propriétaire : source = énumération NASDAQ Trader (gratuite,
-- officielle) + prix D1 Tiingo (clé gratuite, volume réel). Le compte Axi
-- n'expose aucun share CFD (catalogue vérifié ce jour) — pas de rail MT5
-- pour les actions en v1.

-- Univers actions : périmètre scanné par la veille. `etat` permet le cure
-- propriétaire ('exclu' = retiré du scan manuellement, survit aux
-- ré-énumérations).
CREATE TABLE IF NOT EXISTS univers_actions (
    ticker   TEXT PRIMARY KEY,
    nom      TEXT NOT NULL,
    exchange TEXT NOT NULL,                     -- 'NASDAQ' | 'NYSE' | 'AMEX' | ...
    etat     TEXT NOT NULL DEFAULT 'actif',     -- 'actif' | 'exclu'
    maj_le   INTEGER NOT NULL DEFAULT 0,        -- dernière énumération où vu
    cree_le  INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_univers_actions_etat ON univers_actions(etat);

-- Bougies D1 actions, stockage DÉDIÉ à la veille (séparé de `bougies` qui
-- porte les séries de trading MT5/Bybit — règle « un actif = une série » :
-- une action a exactement une série, ici). Volume = volume réel de
-- l'échange (Tiingo), pas un tick volume.
CREATE TABLE IF NOT EXISTS bougies_actions (
    ticker TEXT NOT NULL,
    ts     INTEGER NOT NULL,                    -- secondes UTC (jour de cotation)
    open   REAL NOT NULL,
    high   REAL NOT NULL,
    low    REAL NOT NULL,
    close  REAL NOT NULL,
    volume REAL NOT NULL,
    PRIMARY KEY (ticker, ts)
);
