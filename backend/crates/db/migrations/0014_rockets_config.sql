CREATE TABLE IF NOT EXISTS rockets_config (
    id INTEGER PRIMARY KEY CHECK (id = 1),  -- Ligne unique
    score_min          INTEGER NOT NULL DEFAULT 40,
    phases_actives     TEXT    NOT NULL DEFAULT '["breakout","prelancement"]',  -- JSON array
    rsi_max            REAL    NOT NULL DEFAULT 85.0,
    rsi_min            REAL    NOT NULL DEFAULT 0.0,
    ratio_volume_min   REAL    NOT NULL DEFAULT 1.5,
    vol_marche_min     REAL    NOT NULL DEFAULT 500000.0,
    maj_le             TEXT    NOT NULL DEFAULT (datetime('now'))
);

-- Insérer les valeurs par défaut (paramètres actuels du code)
INSERT OR IGNORE INTO rockets_config (id) VALUES (1);
