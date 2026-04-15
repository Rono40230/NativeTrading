-- P4 : Feature importance XGBoost par permutation (calculée après fine-tuning Rockets)
CREATE TABLE IF NOT EXISTS ml_feature_importance (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    calcule_le   TEXT    NOT NULL DEFAULT (datetime('now')),
    strategie    TEXT    NOT NULL,          -- 'rockets', 'straddle', 'smc'
    feature_idx  INTEGER NOT NULL,          -- index dans le vecteur (0-51)
    feature_nom  TEXT    NOT NULL,          -- nom lisible
    importance   REAL    NOT NULL           -- chute accuracy OOS lors de la permutation
);

CREATE INDEX IF NOT EXISTS idx_ml_feature_importance_strategie
    ON ml_feature_importance (strategie, calcule_le DESC);
