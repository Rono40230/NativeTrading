-- Scanner SMC (11/09) — journal du cycle de vie des setups : chaque annonce
-- intrabar laisse sa trace (le vivier en mémoire était purgé à 2 h sans
-- historique). Alimente le scanner (dissipés grisés, chasse aux faux
-- négatifs) et l'étude §3.2 « voile des setups » (annoncés vs confirmés
-- vs dissipés — caractéristiques des setups qui tiennent).
CREATE TABLE IF NOT EXISTS smc_setups_journal (
    cle        TEXT PRIMARY KEY,   -- identité du setup côté moteur
    strategie  TEXT NOT NULL,
    asset      TEXT NOT NULL,
    tf         TEXT NOT NULL,
    direction  TEXT NOT NULL,
    force_max  INTEGER NOT NULL,   -- force /10 maximale annoncée
    entree     REAL,
    sl         REAL,
    tps        TEXT,               -- JSON [tp1, tp2, tp3]
    debut      INTEGER NOT NULL,   -- ouverture de la bougie d'annonce
    annonce_le INTEGER NOT NULL,
    fin        INTEGER,            -- clôture d'issue (null = vivant)
    issue      TEXT,               -- 'signal' | 'dissipe' | NULL
    signal_id  TEXT                -- ligne signaux liée (issue = signal)
);
CREATE INDEX IF NOT EXISTS idx_setups_journal_recence
    ON smc_setups_journal(annonce_le DESC);
