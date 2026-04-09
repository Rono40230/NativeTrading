-- Phase 8.1 — Collecte des outcomes de trades pour le réentraînement incrémental ML.
-- Chaque trade clôturé (TP/SL/expire) alimente cette table.
CREATE TABLE IF NOT EXISTS ml_training_samples (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    strategie   TEXT    NOT NULL,  -- "SMC" | "ROCKETS" | "STRADDLE"
    asset       TEXT    NOT NULL,
    timeframe   TEXT    NOT NULL,
    direction   TEXT    NOT NULL,  -- "Long" | "Short" | "LONG" | "STRADDLE"
    prix_entree REAL    NOT NULL,
    prix_sortie REAL    NOT NULL,
    stop_loss   REAL    NOT NULL,
    outcome     TEXT    NOT NULL,  -- "tp1"|"tp2"|"tp3"|"sl"|"invalide"|"expire"
    rr_realise  REAL,              -- R:R effectif (positif = gain, négatif = perte)
    cree_le     TEXT    NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_ml_samples_strategie ON ml_training_samples(strategie);
CREATE INDEX IF NOT EXISTS idx_ml_samples_cree_le   ON ml_training_samples(cree_le);
