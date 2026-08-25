-- Phase 5.2 — l'or et l'argent quittent Bybit pour MT5/Axi (prix du vrai
-- broker + historique profond : Bybit ne liste les métaux que depuis
-- mars 2026, Axi en a des années).
-- La série de vérité doit rester UNIQUE : purge complète de l'historique
-- Bybit de ces deux actifs — l'EA repoussera l'historique Axi (par TF)
-- à son premier passage.

UPDATE assets SET source = 'mt5', symbol_mt5 = 'XAUUSD' WHERE id = 'XAUUSD';
UPDATE assets SET source = 'mt5', symbol_mt5 = 'XAGUSD' WHERE id = 'XAGUSD';

DELETE FROM bougies WHERE asset IN ('XAUUSD', 'XAGUSD');
