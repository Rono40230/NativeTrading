-- Nettoyage audit 04/09 : tables mortes (0 accès code, 0 ou 0 ligne utile).
-- ⚠️ signaux_archive N'EST PAS concernée : archive de la purge SMC du 02/09.
DROP TABLE IF EXISTS smc_analyses_llm;
DROP TABLE IF EXISTS temp_metrics;
DROP TABLE IF EXISTS positions;
