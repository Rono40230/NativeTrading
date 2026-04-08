-- Phase 1.2 : multipliers SL/TP configurables pour rockets_config
-- R:R rule : TP1 = SL+1, TP2 = SL+2, TP3 = trailing
ALTER TABLE rockets_config ADD COLUMN sl_mult    REAL NOT NULL DEFAULT 1.0;
ALTER TABLE rockets_config ADD COLUMN tp_mult_1  REAL NOT NULL DEFAULT 2.0;
ALTER TABLE rockets_config ADD COLUMN tp_mult_2  REAL NOT NULL DEFAULT 3.0;
ALTER TABLE rockets_config ADD COLUMN tp_mult_3  REAL NOT NULL DEFAULT 4.0;
ALTER TABLE rockets_config ADD COLUMN trailing_atr REAL NOT NULL DEFAULT 2.0;
