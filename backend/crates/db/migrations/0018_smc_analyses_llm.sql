-- Analyses LLM périodiques des performances SMC Directionnel
CREATE TABLE IF NOT EXISTS smc_analyses_llm (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    nb_trades       INTEGER NOT NULL,
    synthese        TEXT    NOT NULL,
    meilleur_setup  TEXT,           -- ex: "XAUUSD M15 Kill Zone London, score ≥80, winrate 68%"
    pire_setup      TEXT,           -- ex: "EURUSD M5 hors Kill Zone, winrate 22%"
    recommandations TEXT    NOT NULL, -- JSON array
    cree_le         TEXT    NOT NULL DEFAULT (datetime('now'))
);
