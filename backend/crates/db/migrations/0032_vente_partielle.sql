-- Migration 0032 : Option vente partielle configurable par stratégie
-- Option 1 (défaut, 1 = TRUE)  : vente ⅓ à TP1, ⅓ à TP2, ⅓ vers TP3
-- Option 2 (0 = FALSE) : pas de vente intermédiaire, SL progresse quand même
ALTER TABLE rockets_config  ADD COLUMN vente_partielle INTEGER NOT NULL DEFAULT 1;
ALTER TABLE smc_params      ADD COLUMN vente_partielle INTEGER NOT NULL DEFAULT 1;
ALTER TABLE straddle_params ADD COLUMN vente_partielle INTEGER NOT NULL DEFAULT 1;
