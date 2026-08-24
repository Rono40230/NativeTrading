-- Recalage des verdicts et R historiques (règle propriétaire 24/08 :
-- un trade ne peut JAMAIS être « BE tout seul » ; sans prise partielle,
-- TP1 touché ne banque rien, et après TP2 la sortie se fait à TP1).
-- Bugs systématiques réparés dans le moteur (realized_r, prix de sortie,
-- libellés) — ces lignes portent les anciennes valeurs.

-- TP1 touché puis sortie à l'entrée : 0R, libellé TP1+BE.
UPDATE signaux SET verdict = 'TP1+BE', r_realise = 0.0 WHERE verdict = 'TP1';

-- TP2 touché puis sortie au stop remonté à TP1 : +1R, prix de sortie = TP1.
UPDATE signaux
   SET verdict = 'TP2+BE',
       r_realise = 1.0,
       prix_verdict = CAST(json_extract(take_profit, '$[0]') AS REAL)
 WHERE verdict = 'TP2';
