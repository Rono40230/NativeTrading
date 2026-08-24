-- Étape 6 — rôle « ranker de faux pivots » : conviction de l'analyste sur
-- chaque cassure candidate au signal. Seuil réglable (défaut 40 ; 0 = l'avis
-- est purement informatif et n'écarte rien).
ALTER TABLE rockets_params ADD COLUMN conviction_min INTEGER NOT NULL DEFAULT 40;

ALTER TABLE rockets_candidats ADD COLUMN conviction_ia      INTEGER;
ALTER TABLE rockets_candidats ADD COLUMN conviction_raison  TEXT;
