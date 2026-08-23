-- Étape 3 — colonne R réalisé par signal clôturé.
--
-- La courbe de trades du dashboard (bloc par stratégie) trace le R cumulé :
-- chaque clôture porte son R exact depuis le moteur v12 (detail « verdict|R »,
-- TP3 = distance réelle, pas un multiple arrondi). Retour-arrière pour les
-- lignes antérieures : approximation par verdict (TP1/TP2/TP3/SL/BE/Expire).
ALTER TABLE signaux ADD COLUMN r_realise REAL;

UPDATE signaux
   SET r_realise = CASE verdict
        WHEN 'TP1' THEN 1.0
        WHEN 'TP2' THEN 2.0
        WHEN 'TP3' THEN 3.0
        WHEN 'SL'  THEN -1.0
        ELSE 0.0
      END
 WHERE statut = 'Fermé';
