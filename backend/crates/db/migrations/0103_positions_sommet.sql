-- Historique rockets (06/09) : colonne `sommet` — le plus haut atteint par
-- le cours pendant la vie du trade (suivi à chaque évaluation de la gestion,
-- y compris avant R1 : juge aussi bien le trailing des TS que la profondeur
-- des SL). Lecture seule pour le moteur : la donnée vit dans la verticale,
-- aucune règle de gestion ne la consomme.
ALTER TABLE rockets_positions ADD COLUMN sommet REAL;
