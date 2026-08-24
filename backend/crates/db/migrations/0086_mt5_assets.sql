-- Phase 5 — source MT5/Axi : les trois indices passent de Dukascopy (inactif)
-- à MT5 (live broker), actifs, avec leur symbole Axi réel (Market Watch).
-- IG déjà absent du code ; la colonne epic_ig reste inerte (SQLite ne
-- drope pas une colonne sans reconstruire la table).
ALTER TABLE assets ADD COLUMN symbol_mt5 TEXT;

UPDATE assets SET source = 'mt5', symbol_mt5 = 'dax40.fs', actif = 1 WHERE id = 'DAX';
UPDATE assets SET source = 'mt5', symbol_mt5 = 'nas100.fs', actif = 1 WHERE id = 'NAS100';
UPDATE assets SET source = 'mt5', symbol_mt5 = 'S&P.fs',  actif = 1 WHERE id = 'SP500';
