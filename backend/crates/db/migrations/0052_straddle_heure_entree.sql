-- Migration 0052 : Heure d'entrée cible pour les signaux Straddle
-- Un signal Straddle peut être émis en avance (ex: annonce CPI dans 20min).
-- L'heure d'entrée est le timestamp Unix UTC de l'événement déclencheur.
-- NULL = entrée immédiate (comportement actuel — aucune régression Rockets/SMC).
ALTER TABLE signaux ADD COLUMN heure_entree INTEGER; -- Unix UTC, NULL = immédiat
