-- Snapshot quotidien du sentiment composite (Phase 1/2 : sentiment technique).
-- Une ligne par classe + une ligne 'global' par calcul. Persistance best-effort
-- (le worker insère au plus un snapshot par jour pour limiter la taille).
CREATE TABLE IF NOT EXISTS sentiment_historique (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    date TEXT NOT NULL,          -- date du snapshot (YYYY-MM-DD)
    classe TEXT NOT NULL,        -- 'global' | 'crypto' | 'forex' | 'metaux' | 'indices'
    score REAL NOT NULL,         -- 0-100
    composantes TEXT,            -- JSON des composantes (rsi, fg, vix...)
    cree_le TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_sentiment_historique_date ON sentiment_historique(date);
CREATE INDEX IF NOT EXISTS idx_sentiment_historique_classe ON sentiment_historique(classe, date);
