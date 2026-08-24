-- Purge des 41 « Expire » administratifs du 23/08 (ferme_le = 1787509948) :
-- artefacts des bugs A/B/C réparés (doublons intrabar, fantômes, clés
-- périmées) — vendus administrativement par la migration 0076, ces lignes
-- ne sont pas des événements de marché et polluaient les statistiques
-- (question propriétaire 24/08 : 43/45 expirés dans le taux de réussite).
-- L'historique de l'incident reste documenté au journal de la roadmap.
DELETE FROM signaux WHERE verdict = 'Expire' AND ferme_le = 1787509948;

-- Rétro-marquage des trades HISTORIQUES réellement remplis : les verdicts
-- TP*/SL/BE ne surviennent que sur des trades remplis (l'événement Fill
-- marque désormais heure_entree en continu — ces lignes prédatent le
-- marquage). heure_entree ≈ heure de création (approximation honnête).
UPDATE signaux
   SET heure_entree = cree_le
 WHERE verdict IN ('TP1', 'TP2', 'TP3', 'SL', 'BE')
   AND heure_entree IS NULL;
