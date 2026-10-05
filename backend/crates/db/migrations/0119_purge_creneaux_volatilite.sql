-- 0119 (05/10, étape 7 roadmap audit) — chaîne créneaux-volatilite morte.
-- Le job quotidien écrivait la table, l'unique lecteur était l'endpoint
-- /api/creneaux-volatilite — lui-même doublon de /api/volatility/patterns-jour
-- (seule source du bloc Créneaux du dashboard). Job, endpoint, module et
-- table retirés ensemble.
DROP TABLE IF EXISTS creneaux_volatilite;
