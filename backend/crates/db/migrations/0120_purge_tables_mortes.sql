-- 0120 (05/10, étape 8 roadmap audit) — deux tables mortes.
-- straddle_creneaux (0015) : ancien système de créneaux manuels, 0 SELECT/
--   INSERT en Rust depuis des semaines (seule mention : un commentaire).
-- sentiment_historique : écrite à chaque cycle 30 min depuis le 14/08
--   (7 399 lignes) mais JAMAIS lue — la « référence veille » réellement
--   servie vient de figer_veille_marche (clôtures figées, décision du
--   18/08) : le design décrit dans le commentaire d'écriture a été
--   remplacé avant d'entrer en service. L'écriture est retirée avec la
--   table.
DROP TABLE IF EXISTS straddle_creneaux;
DROP TABLE IF EXISTS sentiment_historique;
