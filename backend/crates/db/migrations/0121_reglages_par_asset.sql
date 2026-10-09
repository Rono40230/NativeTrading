-- 0121 — Réglages de stratégie PAR ASSET (spec docs/spec_reglages_par_asset.md).
-- Surcharge optionnelle : NULL = repli sur le défaut global. Aucune ligne = comportement
-- d'aujourd'hui (non-régression par construction). Fractions : si surchargées, les trois
-- doivent être fournies et sommer à 1 (validé côté API/DB).

CREATE TABLE IF NOT EXISTS smc_reglages_asset (
    asset      TEXT PRIMARY KEY,
    sl_max     REAL,            -- clamp haut du SL (NULL = défaut calibration)
    trailing_r REAL,            -- k du trailing après TP2 (NULL = défaut global)
    tp1        REAL,            -- multiplicateur TP1
    tp2        REAL,
    tp3_mode   TEXT,            -- 'lointaine' | 'fixe'
    tp3_rfixe  REAL,
    frac_tp1   REAL,            -- fractions : les 3 ensemble, somme = 1
    frac_tp2   REAL,
    frac_tp3   REAL,
    maj_le     INTEGER NOT NULL DEFAULT (unixepoch())
);

CREATE TABLE IF NOT EXISTS straddle_reglages_asset (
    asset         TEXT PRIMARY KEY,
    sl_mult       REAL,         -- SL = n × ATR H1
    trailing_r    REAL,
    placement_sec INTEGER,
    maj_le        INTEGER NOT NULL DEFAULT (unixepoch())
);

CREATE TABLE IF NOT EXISTS kdj_reglages_asset (
    asset      TEXT PRIMARY KEY,
    period     INTEGER,
    signal     INTEGER,
    amplitude  INTEGER,
    ratio_risk REAL,
    adx_min    REAL,            -- < 0 = filtre désactivé
    maj_le     INTEGER NOT NULL DEFAULT (unixepoch())
);
