-- Table pour historiser les suggestions de paramètres générées par le ML
-- et validées par l'utilisateur. Chaque ligne = une suggestion appliquée.
CREATE TABLE IF NOT EXISTS ml_suggestions_log (
    id                   INTEGER PRIMARY KEY AUTOINCREMENT,
    strategie            TEXT    NOT NULL,  -- "SMC" | "ROCKETS" | "STRADDLE"
    param_name           TEXT    NOT NULL,  -- "score_min", "kill_zone_filtre", "atr_sl"
    valeur_avant         REAL    NOT NULL,
    valeur_apres         REAL    NOT NULL,
    gain_winrate_estime  REAL    NOT NULL,  -- % points de WR estimés
    confiance            REAL    NOT NULL,  -- 0.0-1.0
    nb_samples_base      INTEGER NOT NULL,  -- nb trades ayant servi à la suggestion
    appliquee_le         TEXT    NOT NULL DEFAULT (datetime('now'))
);
