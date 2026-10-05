-- 0118 (05/10, étape 5 roadmap audit) — ML straddle : nouvelles bases.
-- Le vécu straddle d'avant le pivot événementiel a été supprimé sur ordre
-- propriétaire le 28/09 soir (backup froid conservé), mais 101 échantillons
-- d'entraînement ML de l'ancien régime restaient dans ml_training_samples —
-- deux règnes mélangés, contraire à la décision « repartir à zéro ».
-- Frontière : 17:30 UTC le 28/09 (dernier échantien ancien 15:12, premier
-- du nouveau régime 21:01 — trou net de 6 h).
DELETE FROM ml_training_samples
 WHERE strategie = 'straddle' AND cree_le < '2026-09-28 17:30';
