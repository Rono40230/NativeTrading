-- 0069 — Journal des replays du moteur v12 (Phase 2.5 ROADMAP).
--
-- Chaque exécution du replay harness y est archivée avec son journal
-- complet (signaux + événements lifecycle en JSON) et son verdict de
-- parité vs le moteur nu de référence (`conforme_reference`) — c'est la
-- preuve reproductible exigée par la Gate 2 (méthode R : replay).

CREATE TABLE runtime_replay (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    asset TEXT NOT NULL,
    timeframe TEXT NOT NULL,
    simule_ticks INTEGER NOT NULL,      -- 0 = clôtures (parité), 1 = intrabar simulé
    nb_bougies INTEGER NOT NULL,
    periode_de INTEGER NOT NULL,
    periode_a INTEGER NOT NULL,
    nb_signaux INTEGER NOT NULL,
    nb_evenements INTEGER NOT NULL,
    conforme_reference INTEGER NOT NULL, -- 1 = carnet plugin == moteur nu
    nb_trades_reference INTEGER NOT NULL,
    duree_ms INTEGER NOT NULL,
    journal TEXT NOT NULL,               -- JSON {signaux:[], evenements:[]}
    cree_le INTEGER NOT NULL
);

CREATE INDEX idx_runtime_replay_cree ON runtime_replay(cree_le DESC);
