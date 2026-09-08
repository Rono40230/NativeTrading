-- §3 (08/09) — cache quotidien des analystes LLM par stratégie (pattern du
-- Rapport d'activité) : calculé une fois par jour (boot + laziment au
-- premier accès + bouton ↻), servi instantanément aux pages d'analyse
-- plein écran. Un avis par (strategie, jour) — heure Paris.
CREATE TABLE IF NOT EXISTS analyse_cache (
    strategie   TEXT    NOT NULL,   -- 'straddle' | 'smc' | 'rockets'
    jour        TEXT    NOT NULL,   -- YYYY-MM-DD (Paris)
    n_passes    INTEGER NOT NULL,   -- effectif analysé
    avis        TEXT    NOT NULL,   -- JSON complet {n_passes, kpis, analyse}
    calcule_le  INTEGER NOT NULL,
    PRIMARY KEY (strategie, jour)
);
