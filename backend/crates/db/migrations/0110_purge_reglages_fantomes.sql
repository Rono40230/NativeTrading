-- Purge des réglages fantômes (09/09) — tables écrites par des panneaux UI
-- ou la couche ML suggestions, mais lues par AUCUN moteur :
--  • smc_params         : « Paramètres Moteur SMC » (page Analyse SMC) — le
--    moteur v12 vit sur sa calibration figée + la kv `configuration`
--    (smc_tp*_mult, smc_frac_*, smc_tp3_*) via SmcParamsCard.
--  • rockets_config     : « Réglages scan » (page Analyse Rockets) — le
--    scanner réel lit `rockets_params` (/api/rockets/params).
--  • ml_suggestions_log : journal des applications de suggestions ML —
--    toutes les cibles étaient fantômes (la chaîne a été supprimée).
DROP TABLE IF EXISTS smc_params;
DROP TABLE IF EXISTS rockets_config;
DROP TABLE IF EXISTS ml_suggestions_log;
