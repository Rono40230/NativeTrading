-- Migration 0043 : trailing_coeff dynamique et ventes partielles adaptatives par score
-- Implémente la logique R:R unifiée : SL=R-1, TP1=R+1, TP2=R+2, trailing dès TP2 atteint

-- ── Table rockets_signaux : colonnes de gestion de position ─────────────────
ALTER TABLE rockets_signaux ADD COLUMN trailing_coeff REAL;
ALTER TABLE rockets_signaux ADD COLUMN pct_tp1 REAL NOT NULL DEFAULT 0.25;
ALTER TABLE rockets_signaux ADD COLUMN pct_tp2 REAL NOT NULL DEFAULT 0.25;
ALTER TABLE rockets_signaux ADD COLUMN pct_trailing REAL NOT NULL DEFAULT 0.50;

-- ── Table rockets_config : nouveaux paramètres de trailing et split ──────────
ALTER TABLE rockets_config ADD COLUMN trailing_coeff_min REAL NOT NULL DEFAULT 1.5;
ALTER TABLE rockets_config ADD COLUMN trailing_coeff_max REAL NOT NULL DEFAULT 5.0;
ALTER TABLE rockets_config ADD COLUMN seuil_score_faible INTEGER NOT NULL DEFAULT 65;
ALTER TABLE rockets_config ADD COLUMN seuil_score_fort INTEGER NOT NULL DEFAULT 80;

-- Corriger les multiplicateurs TP existants pour cohérence R:R (sl_mult=1.0)
-- TP1 = sl_mult = 1.0, TP2 = sl_mult+1 = 2.0, TP3 trigger = sl_mult+2 = 3.0
UPDATE rockets_config SET
    tp_mult_1 = 1.0,
    tp_mult_2 = 2.0,
    tp_mult_3 = 3.0
WHERE id = 1;
