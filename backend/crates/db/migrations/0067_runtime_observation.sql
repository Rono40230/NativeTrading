-- 0067 — Journal d'observation du runtime tick (Phase 1.5 ROADMAP).
--
-- Chaque bougie clôturée par le runtime y est journalisée, puis comparée
-- à la bougie officielle de `bougies` (source bybit_ws) via l'endpoint
-- GET /api/runtime/concordance — l'outil de mesure de la Gate 1 :
-- « 100 % des bougies runtime == bougies officielles sur 24 h ».
--
-- `mode_cloture` : axe diagnostique —
--   'confirmation' : poussée officielle (doit être identique à 100 %) ;
--   'passage'      : clôturée par le premier événement de la période
--                    suivante (divergence possible si updates manquées) ;
--   'forcee'       : flush à l'arrêt du runtime (jamais comparée — la
--                    bougie n'était pas confirmée).

CREATE TABLE runtime_observation (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    asset TEXT NOT NULL,
    timeframe TEXT NOT NULL,
    timestamp INTEGER NOT NULL,          -- début de bougie (epoch sec)
    open REAL NOT NULL,
    high REAL NOT NULL,
    low REAL NOT NULL,
    close REAL NOT NULL,
    volume REAL NOT NULL,
    mode_cloture TEXT NOT NULL,
    cloture_le_ms INTEGER NOT NULL,      -- horodatage réception runtime
    UNIQUE(asset, timeframe, timestamp)
);

CREATE INDEX idx_runtime_observation_ts
    ON runtime_observation(asset, timeframe, timestamp DESC);
