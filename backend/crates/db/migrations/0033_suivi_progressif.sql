-- Migration 0033 : Suivi stateful des positions SMC et Straddle
-- Permet de conserver l'état du SL progressif (break-even → TP1) entre chaque
-- cycle du worker de réconciliation, sans avoir à rejouer depuis zéro.

-- SMC Directionnel : SL courant + TPs déjà atteints
ALTER TABLE signaux ADD COLUMN sl_effectif       REAL;           -- SL en vigueur (NULL = stop_loss d'origine)
ALTER TABLE signaux ADD COLUMN tps_atteints      TEXT DEFAULT '[]'; -- JSON: [] | ["tp1"] | ["tp1","tp2"]

-- Straddle : suivi indépendant par jambe
ALTER TABLE signaux ADD COLUMN sl_long_effectif   REAL;           -- SL courant jambe LONG  (NULL = stop_loss d'origine)
ALTER TABLE signaux ADD COLUMN sl_short_effectif  REAL;           -- SL courant jambe SHORT (NULL = sl_short d'origine)
ALTER TABLE signaux ADD COLUMN tps_long_atteints  TEXT DEFAULT '[]'; -- JSON TPs atteints jambe LONG
ALTER TABLE signaux ADD COLUMN tps_short_atteints TEXT DEFAULT '[]'; -- JSON TPs atteints jambe SHORT
