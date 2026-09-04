-- §14 roadmap : historisation des rapports d'activité — un snapshot par
-- stratégie et par jour (écrit paresseusement au premier calcul du jour,
-- mis à jour si les réglages changent dans la journée), avis IA du jour
-- rattaché (JSON) pour suivre l'évolution jour après jour.
CREATE TABLE IF NOT EXISTS analyses_snapshots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    strategie TEXT NOT NULL,
    jour TEXT NOT NULL,
    capital_depart REAL NOT NULL DEFAULT 0,
    capital_actuel REAL NOT NULL DEFAULT 0,
    r_total REAL NOT NULL DEFAULT 0,
    taux_reussite REAL NOT NULL DEFAULT 0,
    nb_trades INTEGER NOT NULL DEFAULT 0,
    hier_dollars REAL,
    calcule_le INTEGER NOT NULL,
    avis_ia TEXT,
    avis_le INTEGER,
    UNIQUE(strategie, jour)
);
