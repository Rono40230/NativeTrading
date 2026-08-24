-- Étape 6 — rôle « catalyseur news » de Rockets : l'analyste (qwen3:32b)
-- évalue les dépêches des 15 derniers jours pour chaque candidat du scan.
-- Le point News (1/10 du classement Fondamental) devient effectif.
ALTER TABLE rockets_candidats ADD COLUMN points_base       INTEGER;        -- points chiffrables (max 9)
ALTER TABLE rockets_candidats ADD COLUMN news_verdict     TEXT;           -- POUR | CONTRE | NEUTRE | non évalué
ALTER TABLE rockets_candidats ADD COLUMN news_conviction  INTEGER;        -- 0-100
ALTER TABLE rockets_candidats ADD COLUMN news_justification TEXT;
ALTER TABLE rockets_candidats ADD COLUMN news_points     INTEGER NOT NULL DEFAULT 0;  -- 0 ou 1
