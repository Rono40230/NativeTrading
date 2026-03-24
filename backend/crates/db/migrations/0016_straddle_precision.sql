-- Migration 0016 : colonnes de précision M5 pour les créneaux Straddle
ALTER TABLE straddle_creneaux ADD COLUMN timing_optimal TEXT;         -- "14:07" UTC
ALTER TABLE straddle_creneaux ADD COLUMN fenetre_entree TEXT;         -- "14:05–14:12"
ALTER TABLE straddle_creneaux ADD COLUMN whipsaw_minutes INTEGER;     -- durée moyenne du faux mouvement avant le pic
ALTER TABLE straddle_creneaux ADD COLUMN precision_nb_occurrences INTEGER; -- nb de créneaux M5 analysés
ALTER TABLE straddle_creneaux ADD COLUMN precision_atr_pic REAL;      -- ATR moyen au moment du pic
