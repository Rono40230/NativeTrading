-- Alertes de prix (système propriétaire 26/08) : ligne posée sur le chart,
-- déclenchement au franchissement observé par le runtime tick (chaque prix
-- live Bybit/EA), notification Telegram + OS + son. Ponctuelle par défaut
-- (active = 0 après déclenchement), réarmable.
CREATE TABLE IF NOT EXISTS alertes_prix (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    asset           TEXT    NOT NULL,
    prix            REAL    NOT NULL,
    -- Direction de déclenchement : 'au_dessus' (le prix monte à ce niveau)
    -- ou 'en_dessous' (le prix descend à ce niveau).
    sens            TEXT    NOT NULL DEFAULT 'au_dessus',
    note            TEXT,
    active          INTEGER NOT NULL DEFAULT 1,
    cree_le         INTEGER NOT NULL,
    declenchee_le   INTEGER
);
CREATE INDEX idx_alertes_prix_asset ON alertes_prix(asset, active);
