-- Migration 0050 : Enrichissement des colonnes feedback (P9)
-- Ajoute prix d'entrée/sortie réels, session de sortie, et notes libres
-- sur les 3 tables de mémoire apprenante.
-- Ces colonnes sont nullables : remplies lors de la clôture par le worker.

ALTER TABLE rockets_feedback  ADD COLUMN prix_entree_reel REAL;
ALTER TABLE rockets_feedback  ADD COLUMN prix_sortie_reel  REAL;
ALTER TABLE rockets_feedback  ADD COLUMN session_sortie    TEXT;
ALTER TABLE rockets_feedback  ADD COLUMN notes_trader      TEXT;

ALTER TABLE straddle_feedback ADD COLUMN prix_entree_reel REAL;
ALTER TABLE straddle_feedback ADD COLUMN prix_sortie_reel  REAL;
ALTER TABLE straddle_feedback ADD COLUMN session_sortie    TEXT;
ALTER TABLE straddle_feedback ADD COLUMN notes_trader      TEXT;

ALTER TABLE smc_feedback      ADD COLUMN prix_entree_reel REAL;
ALTER TABLE smc_feedback      ADD COLUMN prix_sortie_reel  REAL;
ALTER TABLE smc_feedback      ADD COLUMN session_sortie    TEXT;
ALTER TABLE smc_feedback      ADD COLUMN notes_trader      TEXT;
