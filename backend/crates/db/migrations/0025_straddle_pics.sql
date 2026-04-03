-- Table des pics de volatilité détectés par le scan Straddle.
-- Enregistrée à chaque scan (toutes les 5 min) si ratio_atr > 1.3,
-- même sans génération de signal. Constitue la mémoire brute du système.
CREATE TABLE IF NOT EXISTS straddle_pics (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    asset               TEXT    NOT NULL,           -- "BTC", "XAUUSD", ...
    timeframe           TEXT    NOT NULL,           -- "M5", "M15"
    timestamp_pic       INTEGER NOT NULL,           -- Unix UTC de la bougie du pic
    prix                REAL    NOT NULL,
    atr_actuel          REAL    NOT NULL,
    atr_moyen_14        REAL    NOT NULL,
    ratio_atr           REAL    NOT NULL,           -- atr_actuel / atr_moyen_14
    -- Catégorisation de la cause
    categorie           TEXT    NOT NULL,           -- voir enum dans le code
    evenement_nom       TEXT,                       -- "NFP", "FOMC", "London Open", ...
    evenement_devise    TEXT,                       -- "USD", "EUR", ...
    evenement_impact    TEXT,                       -- "High", "Medium"
    minutes_avant_evt   INTEGER,                    -- distance en min à l'événement (négatif = passé)
    session_active      TEXT    NOT NULL DEFAULT 'Off', -- "London", "NewYork", "Overlap", "Tokyo", "Off"
    kill_zone_active    INTEGER NOT NULL DEFAULT 0, -- 0/1
    -- Lien vers le signal généré (nullable si aucun signal produit)
    signal_id           TEXT,                       -- FK → signaux.id
    -- Horodatage d'insertion
    cree_le             INTEGER NOT NULL DEFAULT (unixepoch())
);

CREATE INDEX IF NOT EXISTS idx_straddle_pics_asset_ts
    ON straddle_pics(asset, timestamp_pic DESC);

CREATE INDEX IF NOT EXISTS idx_straddle_pics_categorie
    ON straddle_pics(categorie, asset);

CREATE INDEX IF NOT EXISTS idx_straddle_pics_recent
    ON straddle_pics(cree_le DESC);
