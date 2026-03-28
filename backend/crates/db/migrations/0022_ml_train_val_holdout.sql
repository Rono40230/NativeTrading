-- Ajoute les métriques train/validation holdout (phase 3.1)
ALTER TABLE historique_entrainements
ADD COLUMN accuracy_train REAL NOT NULL DEFAULT 0.0;

ALTER TABLE historique_entrainements
ADD COLUMN accuracy_val REAL NOT NULL DEFAULT 0.0;
