-- §10/16 (05/09) — pilotage des positions rockets ouvertes : la table des
-- encours affiche le poste d'observation du Journal de Trading (décision
-- propriétaire : lecture seule, le moteur décide). Métadonnées d'affichage :
--   qty           : quantité ouverte (formule officielle du lot, enregistrée
--                   à l'ouverture — NULL pour les 2 positions préexistantes,
--                   l'endpoint dérive au capital courant)
--   capital_epoque: capital composé de la stratégie à l'ouverture (base du
--                   Risque % et du Montant position)
-- (trailing et prix_r1 existaient déjà ; qty partielle = qty/2, dérivée.)
ALTER TABLE rockets_positions ADD COLUMN qty REAL;
ALTER TABLE rockets_positions ADD COLUMN capital_epoque REAL;
