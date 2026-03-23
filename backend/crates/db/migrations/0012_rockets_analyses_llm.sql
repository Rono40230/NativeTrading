-- Analyses LLM stratégiques des performances Rockets (Mode 2)
CREATE TABLE IF NOT EXISTS rockets_analyses_llm (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    nb_trades       INTEGER NOT NULL,
    synthese        TEXT    NOT NULL,
    meilleur_setup  TEXT,
    pire_setup      TEXT,
    recommandations TEXT    NOT NULL, -- JSON array
    cree_le         TEXT    NOT NULL DEFAULT (datetime('now'))
);
