CREATE TABLE IF NOT EXISTS straddle_creneaux (
    id                     INTEGER PRIMARY KEY AUTOINCREMENT,
    asset                  TEXT    NOT NULL,
    jour_semaine           INTEGER,          -- 0=Lundi...6=Dimanche, NULL=tous les jours
    heure_debut            TEXT    NOT NULL, -- "14:00" UTC
    heure_fin              TEXT    NOT NULL, -- "16:00" UTC
    atr_moyen              REAL,             -- amplitude moyenne (% ou pips)
    frequence              REAL,             -- fréquence de dépassement seuil (0.0–1.0)
    llm_raison             TEXT,
    llm_conviction         INTEGER,          -- 0–100
    statut                 TEXT    NOT NULL DEFAULT 'a_tester', -- 'a_tester' | 'valide' | 'invalide'
    cree_le                TEXT    NOT NULL DEFAULT (datetime('now')),
    backtest_winrate       REAL,
    backtest_profit_factor REAL
);
