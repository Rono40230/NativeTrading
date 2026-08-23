-- Étape 3 (préalable) — nettoyage one-shot des signaux « Actif » pollués.
--
-- Incident 23/08 (3 bugs conjoints, réparés dans le même commit) :
--   A. clé moteur stockée au format Debug (« (2018, 0, 0, …) ») ≠ clé des
--      événements de clôture (« 2018:0:0:… ») → AUCUNE clôture jamais
--      écrite, tout restait Actif à vie ;
--   B. signaux émis sur la bougie EN FORMATION (le Pine crée sur barre
--      confirmée uniquement) → trades fantômes + ré-émissions en boucle
--      quand le prix oscillait (même trade jusqu'à ×9) ;
--   C. clé indexée sur le rang de barre → glissait à chaque redémarrage.
--
-- Ces lignes sont irrécupérables (doublons, verdicts inconnus, clés
-- périmées) : on les solde en « Expire » pour repartir sur un carnet
-- sain. Les signaux post-correctif suivent le cycle complet (création
-- confirmée → clôture avec verdict TP1/TP2/TP3/SL/BE/Expire).
UPDATE signaux
   SET statut = 'Fermé',
       verdict = 'Expire',
       ferme_le = strftime('%s', 'now')
 WHERE statut = 'Actif';
