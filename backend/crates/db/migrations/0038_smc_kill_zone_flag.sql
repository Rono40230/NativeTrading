-- Migration 0038 : ajout du flag kill_zone_active dans smc_params
-- Permet de désactiver le filtre Kill Zone ICT pour les tests/diagnostic.
ALTER TABLE smc_params
    ADD COLUMN kill_zone_filtre INTEGER NOT NULL DEFAULT 1;
-- 1 = Kill Zone requise (comportement actuel prod)
-- 0 = Kill Zone ignorée (diagnostic / tests hors-session)
