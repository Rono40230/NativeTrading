-- Étape C Rockets actions (31/08) : scanner quotidien en Observation
-- silencieuse. Les candidats actions partagent la table crypto — une
-- colonne univers les distingue (la purge de chaque scanner est cloisonnée
-- à son univers).

ALTER TABLE rockets_candidats ADD COLUMN univers TEXT NOT NULL DEFAULT 'crypto';

-- Entonnoir du pré-screen : chaque ticker évalué par le trend template avec
-- ≥ 6 conditions sur 8 (passants 8 + approchants 6-7) — la photo quotidienne
-- de qui approche du périmètre scanné.
CREATE TABLE IF NOT EXISTS prescreen_actions (
    ticker      TEXT PRIMARY KEY,
    nom         TEXT NOT NULL DEFAULT '',
    conditions  INTEGER NOT NULL,             -- 0..8 (8 = trend template réussi)
    points      INTEGER NOT NULL DEFAULT 0,   -- classement /10 (si évalué)
    perf_4s_pct REAL NOT NULL DEFAULT 0,      -- performance 4 semaines du ticker
    maj_le      INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_prescreen_conditions ON prescreen_actions(conditions DESC, maj_le DESC);
