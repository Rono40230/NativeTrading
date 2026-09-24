-- 6.7 (24/09) : gel des fractions en vigueur sur chaque clôture SMC.
-- JSON {"tp1":0.5,"tp2":0.3,"tp3":0.2} écrit par fermer_signal_par_cle.
-- NULL = clôture historique (avant 24/09) → repli Fractions::default()
-- à la lecture (capital_simule) : la courbe $ du vécu ne change pas.
-- Les autres stratégies n'ont pas de ventes partielles (NULL permanent).
ALTER TABLE signaux ADD COLUMN fractions_json TEXT;
