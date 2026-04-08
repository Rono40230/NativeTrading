-- Paramètres de sizing par asset : valeur du pip, SL par défaut, risque%
CREATE TABLE IF NOT EXISTS asset_params (
    asset       TEXT PRIMARY KEY,
    valeur_pips REAL NOT NULL DEFAULT 10.0,  -- valeur monétaire d'1 pip en USD
    sl_pips     REAL NOT NULL DEFAULT 20.0,  -- SL par défaut en pips
    risque_pct  REAL NOT NULL DEFAULT 1.0,   -- risque % du capital par trade
    lot_min     REAL NOT NULL DEFAULT 0.01,
    lot_max     REAL NOT NULL DEFAULT 10.0,
    maj_le      TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Seed depuis le tableau de l'utilisateur
INSERT OR IGNORE INTO asset_params (asset, valeur_pips, sl_pips, risque_pct) VALUES
    ('BTC',    1.0,  350.0, 1.0),
    ('ETH',    1.0,  200.0, 1.0),
    ('XAUUSD', 10.0,  50.0, 1.0),
    ('XAGUSD', 50.0,  15.0, 1.0),
    ('EURUSD', 10.0,  15.0, 1.0),
    ('GBPUSD', 10.0,  15.0, 1.0),
    ('USDCAD', 10.0,  15.0, 1.0),
    ('EURGBP', 12.5,  15.0, 1.0),
    ('GBPJPY',  9.13, 20.0, 1.0),
    ('EURJPY',  9.13, 20.0, 1.0),
    ('USDJPY',  9.13, 20.0, 1.0),
    ('CADJPY',  9.13, 20.0, 1.0),
    ('NZDJPY',  6.96, 15.0, 1.0),
    ('CHFJPY',  7.70, 30.0, 1.0),
    ('AUDUSD', 10.0,  15.0, 1.0),
    ('DAX',    10.0,  50.0, 1.0),
    ('SP500',  10.0,  50.0, 1.0);
