-- 0068 — Choix explicites du propriétaire (2026-08-15).
--
-- 1. worker_timeframes : les 8 timeframes M1→W1. Ce n'est PAS un défaut
--    d'implémentation mais le choix explicite du propriétaire, dicté par la
--    couverture exigée des stratégies (voir ROADMAP Phase 2) :
--      SMC      : M1 → D1
--      Straddle : M1 → D1
--      Rockets  : M30 → W1
--    Modifiable à tout moment depuis la vue Données. Le code ne porte
--    plus AUCUN défaut : liste vide = aucune collecte (l'UI demande de choisir).
--
-- 2. retention_bougies : conservation en mois, par timeframe. Choix initial
--    du propriétaire : 24 mois uniforme. TF absent du JSON = illimité.
--    Aucune valeur imposée par le code — pur choix utilisateur.
--
-- 3. retention_observation_jours : journal de diagnostic du runtime
--    (Gate 1/2), pas un historique de marché — conservation courte,
--    ajustable.

UPDATE configuration
   SET valeur = '["M1","M5","M15","M30","H1","H4","D1","W1"]',
       maj_le = strftime('%s','now')
 WHERE cle = 'worker_timeframes';

INSERT INTO configuration (cle, valeur, maj_le) VALUES
('retention_bougies',
 '{"M1":24,"M5":24,"M15":24,"M30":24,"H1":24,"H4":24,"D1":24,"W1":24}',
 strftime('%s','now')),
('retention_observation_jours', '90', strftime('%s','now'))
ON CONFLICT(cle) DO UPDATE SET
 valeur = excluded.valeur, maj_le = excluded.maj_le;
