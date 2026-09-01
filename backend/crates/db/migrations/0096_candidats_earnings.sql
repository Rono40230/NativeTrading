-- Étape D (01/09) : avertissement earnings sur les candidats actions.
-- Date ISO (YYYY-MM-DD) extraite par l'analyste news SEULEMENT si une
-- dépêche la mentionne explicitement — jamais devinée.
ALTER TABLE rockets_candidats ADD COLUMN earnings_le TEXT;
