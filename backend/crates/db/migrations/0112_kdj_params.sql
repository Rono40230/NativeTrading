-- KDJ/Halftrend — paramètres moteur (7.E). Ligne unique id=1.
-- adx_min < 0 = filtre tendance désactivé (fidélité étalon).
CREATE TABLE IF NOT EXISTS kdj_params (
    id          INTEGER PRIMARY KEY CHECK (id = 1),
    period      INTEGER NOT NULL DEFAULT 20,
    signal      INTEGER NOT NULL DEFAULT 7,
    amplitude   INTEGER NOT NULL DEFAULT 2,
    ratio_risk  REAL    NOT NULL DEFAULT 2.0,
    adx_min     REAL    NOT NULL DEFAULT -1
);
INSERT OR IGNORE INTO kdj_params (id) VALUES (1);
