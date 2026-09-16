-- 0113 — Laboratoire de simulation (15/09 nuit, décision propriétaire).
-- Bibliothèque des essais : chaque simulation lancée depuis la page
-- 🧪 Simulation y est conservée (paramètres + résultats), pour comparer
-- plusieurs réglages d'un coup. Les chiffres officiels (vécu) ne vivent
-- jamais ici — cette table est un carnet de laboratoire.
CREATE TABLE IF NOT EXISTS simulation_essais (
    id           TEXT PRIMARY KEY,   -- UUID v4
    strategie    TEXT NOT NULL,      -- 'SMC', 'straddle', ...
    params_json  TEXT NOT NULL,      -- paramètres virtuels de l'essai
    resultats_json TEXT NOT NULL,    -- résultats agrégés de l'essai
    cree_le      INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_simulation_essais_strategie
    ON simulation_essais (strategie, cree_le DESC);
