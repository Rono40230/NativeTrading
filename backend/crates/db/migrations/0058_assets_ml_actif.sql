-- Migration 0058 : colonne ml_actif + table stats bougies pour éviter COUNT(*) lent
-- Problème : GROUP BY bougies + HAVING COUNT(*) >= 200 scanne 10M+ lignes → 25s
-- Solution : table de stats pré-calculée + colonne ml_actif dédiée au ML

-- 1. Colonne ml_actif sur assets (séparé du soft-delete actif)
--    Par défaut = même valeur que actif (tous les assets actifs sont éligibles au ML)
ALTER TABLE assets ADD COLUMN ml_actif INTEGER NOT NULL DEFAULT 1;
UPDATE assets SET ml_actif = actif;

-- 2. Table de stats pré-calculées (évite le COUNT(*) sur 10M lignes)
CREATE TABLE IF NOT EXISTS bougies_stats (
    asset     TEXT NOT NULL,
    timeframe TEXT NOT NULL,
    nb        INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (asset, timeframe)
);

-- Peupler avec les données actuelles
INSERT OR REPLACE INTO bougies_stats (asset, timeframe, nb)
SELECT asset, timeframe, COUNT(*) FROM bougies GROUP BY asset, timeframe;

-- 3. Index dédié sur bougies_stats pour les jointures
CREATE INDEX IF NOT EXISTS idx_bougies_stats_nb ON bougies_stats(nb);
