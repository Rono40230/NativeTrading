-- Page Scanner (01/09) : les candidats périmés ne sont plus SUPPRIMÉS mais
-- marqués éliminés (date de sortie) — l'historique des setups reste visible.
-- La ré-candidature (INSERT OR REPLACE) remet elimine_le à NULL d'office.
ALTER TABLE rockets_candidats ADD COLUMN elimine_le INTEGER;
