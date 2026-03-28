-- Paramètres de la stratégie Straddle (BiDi)
CREATE TABLE IF NOT EXISTS straddle_params (
    id               INTEGER PRIMARY KEY CHECK (id = 1),
    atr_periode      INTEGER NOT NULL DEFAULT 14,
    atr_seuil        REAL    NOT NULL DEFAULT 1.5,   -- ATR > seuil × moyenne pour déclencher
    tp_mult_1        REAL    NOT NULL DEFAULT 2.0,
    tp_mult_2        REAL    NOT NULL DEFAULT 3.5,
    tp_mult_3        REAL    NOT NULL DEFAULT 5.0,
    sl_mult          REAL    NOT NULL DEFAULT 0.5,
    horizon_bougies  INTEGER NOT NULL DEFAULT 48,    -- bougies M5 max avant expiration
    trailing_atr     REAL    NOT NULL DEFAULT 1.5,   -- SL trailing = peak − ATR × trailing_atr
    maj_le           TEXT    NOT NULL DEFAULT (datetime('now'))
);
INSERT OR IGNORE INTO straddle_params (id) VALUES (1);

-- Paramètres de la stratégie SMC Directionnelle (Uni)
CREATE TABLE IF NOT EXISTS smc_params (
    id               INTEGER PRIMARY KEY CHECK (id = 1),
    atr_periode      INTEGER NOT NULL DEFAULT 14,
    score_min        INTEGER NOT NULL DEFAULT 70,    -- confluence minimale /100
    atr_tp1          REAL    NOT NULL DEFAULT 1.5,
    atr_tp2          REAL    NOT NULL DEFAULT 3.0,
    atr_tp3          REAL    NOT NULL DEFAULT 5.0,
    atr_sl           REAL    NOT NULL DEFAULT 1.0,
    horizon_bougies  INTEGER NOT NULL DEFAULT 24,    -- bougies M5 max avant expiration
    maj_le           TEXT    NOT NULL DEFAULT (datetime('now'))
);
INSERT OR IGNORE INTO smc_params (id) VALUES (1);
