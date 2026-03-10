-- Migration 0001 : Schéma initial Native Trading AI
-- Bougies OHLCV par asset/timeframe
CREATE TABLE IF NOT EXISTS bougies (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    asset       TEXT    NOT NULL,           -- "BTC", "ETH", "XAUUSD", "XAGUSD"
    timeframe   TEXT    NOT NULL,           -- "M1", "M5", "M15", "H1", "H4", "D1"
    timestamp   INTEGER NOT NULL,           -- Unix timestamp UTC (secondes)
    open        REAL    NOT NULL,
    high        REAL    NOT NULL,
    low         REAL    NOT NULL,
    close       REAL    NOT NULL,
    volume      REAL    NOT NULL,
    UNIQUE(asset, timeframe, timestamp)
);

CREATE INDEX IF NOT EXISTS idx_bougies_asset_tf_ts
    ON bougies(asset, timeframe, timestamp DESC);

-- Signaux générés par les stratégies
CREATE TABLE IF NOT EXISTS signaux (
    id              TEXT    PRIMARY KEY,    -- UUID v4
    asset           TEXT    NOT NULL,
    timeframe       TEXT    NOT NULL,
    direction       TEXT    NOT NULL,       -- "Long", "Short", "Both"
    score           REAL    NOT NULL,
    prix_entree     REAL    NOT NULL,
    stop_loss       REAL    NOT NULL,
    take_profit     TEXT    NOT NULL,       -- JSON: [tp1, tp2, tp3]
    strategie       TEXT    NOT NULL,       -- "Straddle", "SMCDirectionnel"
    statut          TEXT    NOT NULL DEFAULT 'Actif', -- "Actif", "Fermé", "Annulé"
    cree_le         INTEGER NOT NULL,
    ferme_le        INTEGER
);

CREATE INDEX IF NOT EXISTS idx_signaux_asset_statut
    ON signaux(asset, statut, cree_le DESC);

-- Positions ouvertes et historique
CREATE TABLE IF NOT EXISTS positions (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    signal_id       TEXT    REFERENCES signaux(id),
    asset           TEXT    NOT NULL,
    direction       TEXT    NOT NULL,
    taille          REAL    NOT NULL,       -- En unités d'asset
    prix_entree     REAL    NOT NULL,
    prix_sortie     REAL,
    pnl             REAL,                   -- P&L en USD
    statut          TEXT    NOT NULL DEFAULT 'Ouverte', -- "Ouverte", "Fermée"
    ouverte_le      INTEGER NOT NULL,
    fermee_le       INTEGER
);

-- Configuration utilisateur (clé/valeur)
CREATE TABLE IF NOT EXISTS configuration (
    cle     TEXT PRIMARY KEY,
    valeur  TEXT NOT NULL,
    maj_le  INTEGER NOT NULL
);
