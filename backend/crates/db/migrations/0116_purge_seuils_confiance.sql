-- 0116 (05/10, étape 2 roadmap audit) — purge des seuils de confiance.
-- Ces trois clés étaient éditables via la whitelist config mais lues par
-- AUCUN code depuis la purge ML du 09/09 (les lecteurs ont disparu avec
-- les suggestions de paramètres) : réglages fantômes. Décision
-- propriétaire 05/10 : retrait. Un vrai filtre de confiance reviendra
-- avec l'advisory ML reconstruit sur données réelles (étape 12).
DELETE FROM configuration WHERE cle IN ('seuil_confiance_rockets', 'seuil_confiance_straddle', 'seuil_confiance_smc');
