-- 0125 — SL fixe (owner 19/10) : la colonne sl_max (clamp ×ATR) est morte
-- avec la purge du chemin ATR. Rebuild SQLite sans elle.
CREATE TABLE smc_reglages_asset_new (
    asset      TEXT PRIMARY KEY,
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
INSERT INTO smc_reglages_asset_new
    (asset, trailing_r, tp1, tp2, tp3_mode, tp3_rfixe, frac_tp1, frac_tp2, frac_tp3, maj_le)
SELECT asset, trailing_r, tp1, tp2, tp3_mode, tp3_rfixe, frac_tp1, frac_tp2, frac_tp3, maj_le
FROM smc_reglages_asset;
DROP TABLE smc_reglages_asset;
ALTER TABLE smc_reglages_asset_new RENAME TO smc_reglages_asset;
