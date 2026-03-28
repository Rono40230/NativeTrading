-- Migration 0019 : Persistance complète des deux jambes du signal Straddle
-- Avant cette migration, seule la jambe LONG était sauvegardée en DB
-- (sl_long stocké dans stop_loss, tps long dans take_profit).
-- Les niveaux SHORT étaient retournés dans la réponse JSON mais perdus.
ALTER TABLE signaux ADD COLUMN sl_short          REAL;   -- SL de la jambe SHORT (>prix_entree)
ALTER TABLE signaux ADD COLUMN take_profit_short TEXT;   -- JSON [tp1_short, tp2_short, tp3_short] (<prix_entree)
