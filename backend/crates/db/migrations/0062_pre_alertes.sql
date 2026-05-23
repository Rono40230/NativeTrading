-- Table des pré-alertes : signaux "en formation" (score approchant le seuil)
CREATE TABLE IF NOT EXISTS pre_alertes (
    id            TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(8)))),
    asset         TEXT NOT NULL,
    strategie     TEXT NOT NULL,       -- 'smc' | 'straddle'
    raison        TEXT NOT NULL,       -- description humaine du setup détecté
    score_actuel  REAL,               -- score SMC actuel (null pour straddle)
    evenement     TEXT,               -- titre événement macro (null pour SMC pur)
    minutes_avant INTEGER,            -- minutes avant l'événement (null si non lié)
    telegram_envoye INTEGER NOT NULL DEFAULT 0,
    cree_le       TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

-- Paramètres dynamiques pour les pré-alertes (table configuration, modifiables par calibration LLM)
INSERT OR IGNORE INTO configuration (cle, valeur)
VALUES
    ('smc_seuil_prealerte',             '55'),
    ('straddle_horizon_macro_min',      '90'),
    ('straddle_atr_pct_prealerte',      '80'),
    ('prealerte_cooldown_straddle_min', '30'),
    ('prealerte_cooldown_smc_min',      '240');
