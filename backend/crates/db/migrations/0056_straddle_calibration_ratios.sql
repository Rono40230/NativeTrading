-- Migration 0056 : Ajout des ratios SL/TP calibrés dans straddle_calibration.
-- Permet d'avoir des coefficients ATR adaptés par (asset, categorie)
-- au lieu de valeurs fixes dans le code.

ALTER TABLE straddle_calibration ADD COLUMN sl_ratio      REAL NOT NULL DEFAULT 0.5;
ALTER TABLE straddle_calibration ADD COLUMN tp1_ratio     REAL NOT NULL DEFAULT 2.0;
ALTER TABLE straddle_calibration ADD COLUMN tp2_ratio     REAL NOT NULL DEFAULT 3.5;
ALTER TABLE straddle_calibration ADD COLUMN trailing_coeff REAL NOT NULL DEFAULT 2.0;
