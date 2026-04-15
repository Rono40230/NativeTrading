-- Migration 0051 : Table des patterns d'échec appris automatiquement (P10).
-- Alimentée par le job patterns_echec_job (toutes les 6h).
-- Les règles actives sont injectées dans les prompts Ollama.
CREATE TABLE IF NOT EXISTS regles_rejet_apprises (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    strategie   TEXT    NOT NULL,           -- 'ROCKETS' | 'SMC' | 'STRADDLE'
    condition   TEXT    NOT NULL,           -- description lisible du pattern
    cle_unique  TEXT    NOT NULL,           -- hash déterministe pour UPSERT
    win_rate    REAL    NOT NULL,
    nb_trades   INTEGER NOT NULL,
    active      INTEGER NOT NULL DEFAULT 1,
    apprise_le  TEXT    NOT NULL DEFAULT (datetime('now')),
    UNIQUE(cle_unique)
);
CREATE INDEX IF NOT EXISTS idx_regles_strategie_active
    ON regles_rejet_apprises(strategie, active);
