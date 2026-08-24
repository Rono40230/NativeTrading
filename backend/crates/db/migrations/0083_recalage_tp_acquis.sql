-- Comptabilité propriétaire (rectification 24/08 soir) : le TP touché est
-- ACQUIS — TP1+BE = 1R, TP2+BE = 2R. Le BE forcé (BOS opposé avant TP1,
-- stop ramené à l'entrée) reste un verdict légitime à 0R.
-- Annule le recalage « sans prise partielle » de la migration 0082.

UPDATE signaux SET r_realise = 1.0 WHERE verdict = 'TP1+BE';
UPDATE signaux SET r_realise = 2.0 WHERE verdict = 'TP2+BE';
