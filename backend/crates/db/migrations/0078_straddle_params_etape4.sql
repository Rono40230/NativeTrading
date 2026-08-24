-- Étape 4 — paramètres de la définition straddle actée (24/08) :
--   placement à T-10 s (secondes, réglable), trailing en ×R (dès TP2).
-- TP1/TP2 deviennent canoniques (1R / 2R) ; TP3 supprimé — les anciens
-- tp_mult_* restent en place (inutilisés par le moteur, cleaned à terme).
ALTER TABLE straddle_params ADD COLUMN placement_sec INTEGER NOT NULL DEFAULT 10;
ALTER TABLE straddle_params ADD COLUMN trailing_r   REAL    NOT NULL DEFAULT 1.0;
