-- 0117 (05/10, étape 3 roadmap audit) — sortie de stagnation rockets.
-- Une position qui n'atteint jamais R1 ne sortait QUE sur invalidation
-- −1R : trois positions dormantes depuis 13 jours (owner 05/10). Désormais
-- : au-delà de N jours ouvrés sans décoller, clôture au prix courant,
-- verdict STAG (R latent capté tel quel). Défaut propriétaire : 10 jours.
ALTER TABLE rockets_params ADD COLUMN stagnation_max_jours INTEGER NOT NULL DEFAULT 10;
