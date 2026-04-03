-- Migration 0027 : Table straddle_calibration
-- Stocke les seuils calculés automatiquement par (asset, categorie)
-- à partir des feedbacks historiques. Rechargés à chaque cycle de la boucle.

CREATE TABLE IF NOT EXISTS straddle_calibration (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    asset           TEXT    NOT NULL,
    categorie       TEXT    NOT NULL,
    -- Seuils calculés
    score_llm_seuil REAL    NOT NULL DEFAULT 6.0,   -- seuil optimal score /10
    atr_seuil       REAL    NOT NULL DEFAULT 1.5,   -- ratio ATR minimal optimal
    -- Métriques
    nb_trades       INTEGER NOT NULL DEFAULT 0,
    win_rate        REAL    NOT NULL DEFAULT 0.0,
    pnl_moyen_r     REAL    NOT NULL DEFAULT 0.0,
    -- Fiabilité : "insuffisant" | "faible" | "correct" | "fort"
    fiabilite       TEXT    NOT NULL DEFAULT 'insuffisant',
    -- Si toutes les catégories testées ont WR < 50% : invalide = 1 → skip signal
    invalide        INTEGER NOT NULL DEFAULT 0,
    -- Timestamp dernière recalibration
    maj_le          INTEGER NOT NULL DEFAULT (unixepoch()),
    UNIQUE(asset, categorie)
);

CREATE INDEX IF NOT EXISTS idx_sc_asset_cat ON straddle_calibration(asset, categorie);
