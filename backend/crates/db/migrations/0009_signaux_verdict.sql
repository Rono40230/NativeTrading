-- Migration 0009 : Ajout verdict + prix_verdict sur les signaux SMC/Straddle
-- Permet le suivi pyramidal (TP1/TP2/TP3) et le calcul des R-multiples

ALTER TABLE signaux ADD COLUMN verdict      TEXT;    -- 'SL' | 'TP1' | 'TP2' | 'TP3' | 'expire'
ALTER TABLE signaux ADD COLUMN prix_verdict REAL;    -- Prix réel à la clôture
