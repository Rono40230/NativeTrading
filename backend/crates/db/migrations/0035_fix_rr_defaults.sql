-- Phase 1.2 : correction des multiplicateurs R:R existants en DB
-- Règle : TP1 = SL+1, TP2 = SL+2
-- Straddle : sl=0.5 → tp1 doit être 1.5, tp2 doit être 2.5
UPDATE straddle_params SET tp_mult_1 = 1.5, tp_mult_2 = 2.5 WHERE id = 1;
-- SMC : sl=1.0 → tp1 doit être 2.0 (tp2=3.0 déjà correct)
UPDATE smc_params SET atr_tp1 = 2.0 WHERE id = 1;
