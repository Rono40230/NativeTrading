-- Étape 5 — verticale Rockets (définition canonique figée 24/08/2026).
-- Le moteur tourne : la stratégie passe en Observation (journalisée,
-- silencieuse) — décision étape 5 point 9.

CREATE TABLE IF NOT EXISTS rockets_candidats (
    symbole  TEXT PRIMARY KEY,
    points   INTEGER NOT NULL,
    verdict  TEXT    NOT NULL,      -- Alpha | Rocket | Elimine
    pivot    REAL    NOT NULL,
    stop     REAL    NOT NULL,
    cassure  INTEGER NOT NULL,      -- la dernière bougie D1 casse le pivot
    detail   TEXT    NOT NULL,      -- JSON du détail point par point
    maj_le   INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS rockets_positions (
    cle         TEXT PRIMARY KEY,   -- rockets-{symbole}-{ts cassure}
    symbole     TEXT NOT NULL,
    entree      REAL NOT NULL,      -- prix du pivot (stop-limit théorique)
    stop        REAL NOT NULL,      -- invalidation (1R sous l'entrée)
    r1          REAL NOT NULL,
    neutralise  INTEGER NOT NULL DEFAULT 0,  -- 50 % vendus à R1
    trailing    REAL,                          -- trailing stop actif (prix)
    prix_r1     REAL,
    fermee      INTEGER NOT NULL DEFAULT 0,
    verdict     TEXT,                          -- SL | TS
    r_realise   REAL,
    prix_sortie REAL,
    ts_entree   INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS rockets_params (
    id                  INTEGER PRIMARY KEY CHECK (id = 1),
    profil              TEXT  NOT NULL DEFAULT 'Neutre',  -- PeuRisque|Neutre|Risque
    plafond_position_pct REAL NOT NULL DEFAULT 5.0,
    trailing_pct        REAL  NOT NULL DEFAULT 5.0,
    volume_pivot_mult   REAL  NOT NULL DEFAULT 1.5,
    cassure_min_pct     REAL  NOT NULL DEFAULT 3.0
);
INSERT OR IGNORE INTO rockets_params (id) VALUES (1);

UPDATE strategies SET etat = 'Observation' WHERE id = 'rockets';
