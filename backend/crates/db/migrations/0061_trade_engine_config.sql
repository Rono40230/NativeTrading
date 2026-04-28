-- Migration pour le Moteur Centralisé de Trade Management (Standardisation des TP)

ALTER TABLE straddle_params ADD COLUMN pct_cloture_tp1 REAL NOT NULL DEFAULT 0.33;
ALTER TABLE straddle_params ADD COLUMN pct_cloture_tp2 REAL NOT NULL DEFAULT 0.33;

ALTER TABLE smc_params ADD COLUMN pct_cloture_tp1 REAL NOT NULL DEFAULT 0.33;
ALTER TABLE smc_params ADD COLUMN pct_cloture_tp2 REAL NOT NULL DEFAULT 0.33;

ALTER TABLE rockets_config ADD COLUMN pct_cloture_tp1 REAL NOT NULL DEFAULT 0.33;
ALTER TABLE rockets_config ADD COLUMN pct_cloture_tp2 REAL NOT NULL DEFAULT 0.33;
