-- Historique des entraînements ML (walk-forward métriques out-of-sample)
CREATE TABLE IF NOT EXISTS historique_entrainements (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    cree_le         INTEGER NOT NULL,   -- Unix timestamp UTC
    asset           TEXT    NOT NULL,   -- Asset utilisé (ex: "BTC")
    timeframe       TEXT    NOT NULL,   -- Timeframe (ex: "M15")
    nb_bougies      INTEGER NOT NULL,   -- Total bougies utilisées
    accuracy_rf     REAL    NOT NULL,   -- Accuracy RF sur jeu de test (out-of-sample)
    accuracy_lstm   REAL    NOT NULL,   -- Accuracy LSTM sur jeu de test
    accuracy_finale REAL    NOT NULL,   -- Score fusion (0.6×lstm + 0.4×rf)
    duree_ms        INTEGER NOT NULL,   -- Durée entraînement en millisecondes
    derive_detectee INTEGER NOT NULL DEFAULT 0  -- 1 si dérive détectée (accuracy < seuil 7j)
);

CREATE INDEX IF NOT EXISTS idx_hist_entrainements_date
    ON historique_entrainements(cree_le DESC);
