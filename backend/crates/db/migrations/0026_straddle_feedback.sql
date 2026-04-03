-- Migration 0026 : Table straddle_feedback — mémoire apprenante de la stratégie Straddle
-- Mémorise le résultat de chaque signal Straddle avec son contexte de déclenchement.
-- Alimentée par straddle_feedback_job.rs (réconciliation toutes les 5 min).

CREATE TABLE IF NOT EXISTS straddle_feedback (
    id                      INTEGER PRIMARY KEY AUTOINCREMENT,
    signal_id               TEXT    NOT NULL UNIQUE,    -- FK → signaux.id
    pic_id                  INTEGER,                    -- FK → straddle_pics.id (peut être NULL)
    asset                   TEXT    NOT NULL,
    timeframe               TEXT    NOT NULL,
    timestamp_signal        INTEGER NOT NULL,           -- Unix UTC de création du signal
    -- Contexte au moment du signal (copié depuis straddle_pics si disponible)
    categorie               TEXT    NOT NULL DEFAULT 'choc_isole',
    evenement_nom           TEXT,
    session_active          TEXT,
    ratio_atr               REAL    NOT NULL DEFAULT 0.0,
    score_llm               REAL    NOT NULL,           -- score /10 donné par Ollama
    -- Résultat réel (NULL tant que le signal est ouvert)
    verdict                 TEXT,     -- 'tp1' | 'tp2' | 'tp3' | 'sl' | 'expire'
    amplitude_reelle_pct    REAL,     -- mouvement effectif en % depuis prix_entree
    duree_trade_min         INTEGER,  -- durée jusqu'à clôture (en minutes)
    pnl_r                   REAL,     -- résultat en R (1R = risque initial)
    gagnant                 INTEGER,  -- 1 si TP1 ou mieux, 0 si SL ou expire, NULL si ouvert
    -- Meta
    cree_le                 INTEGER NOT NULL DEFAULT (unixepoch()),
    ferme_le                INTEGER   -- NULL tant que position ouverte
);

CREATE INDEX IF NOT EXISTS idx_sf_asset_cat   ON straddle_feedback(asset, categorie);
CREATE INDEX IF NOT EXISTS idx_sf_signal      ON straddle_feedback(signal_id);
CREATE INDEX IF NOT EXISTS idx_sf_verdict     ON straddle_feedback(verdict, gagnant);
CREATE INDEX IF NOT EXISTS idx_sf_cree_le     ON straddle_feedback(cree_le DESC);
