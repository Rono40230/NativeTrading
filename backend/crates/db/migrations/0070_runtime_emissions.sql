-- 0070 — Journal des émissions LIVE du runtime (Phase 2.6 — shadow mode).
--
-- Chaque signal et chaque événement lifecycle publiés par les moteurs du
-- runtime (actuellement : v12 en shadow) y est persisté à l'émission.
-- C'est la matière brute du test de vérité (Gate 2) : comparaison avec ce
-- que TradingView affiche/alerte aux mêmes instants.
--
-- Rétention : même fenêtre que le journal d'observation (90 j par défaut,
-- clé `retention_observation_jours`) — diagnostic, pas historique.

CREATE TABLE runtime_emissions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    type TEXT NOT NULL,             -- 'signal' | 'evenement'
    moteur TEXT NOT NULL,
    asset TEXT NOT NULL,
    timeframe TEXT NOT NULL,
    -- champs signal
    direction TEXT,
    prix REAL NOT NULL,
    stop_loss REAL,
    take_profits TEXT,              -- JSON [tp1,tp2,tp3]
    score INTEGER,
    raison TEXT,
    -- champs événement
    cle_trade TEXT,
    type_evenement TEXT,            -- Fill/Be/Tp1/Tp2/Tp3/Cloture
    detail TEXT,
    debut_barre INTEGER NOT NULL,
    emis_le INTEGER NOT NULL        -- epoch millisecondes
);

CREATE INDEX idx_runtime_emissions_temps ON runtime_emissions(emis_le DESC);
CREATE INDEX idx_runtime_emissions_asset ON runtime_emissions(asset, timeframe, emis_le DESC);
