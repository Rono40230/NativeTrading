ALTER TABLE rockets_signaux ADD COLUMN statut TEXT NOT NULL DEFAULT 'attente';
ALTER TABLE rockets_signaux ADD COLUMN prix_peak REAL;
ALTER TABLE rockets_signaux ADD COLUMN atr14 REAL;
