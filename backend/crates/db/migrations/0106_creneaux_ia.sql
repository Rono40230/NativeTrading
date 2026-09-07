-- §16 (07/09) — agenda intelligent straddle : créneaux de volatilité
-- récurrents découverts statistiquement (heure×jour, 24 mois de M15),
-- proposés par l'analyste IA, armés par le propriétaire SEUL. Un créneau
-- armé devient une annonce synthétique (🤖) : timer T-10 s, 2 jambes,
-- moteur unifié — le rail M1 existant fait le reste sans modification.
CREATE TABLE IF NOT EXISTS creneaux_ia (
    asset         TEXT    NOT NULL,
    jour          INTEGER NOT NULL,  -- ISO : 1=lundi … 7=dimanche
    heure         INTEGER NOT NULL,  -- heure Paris 0-23
    vol_pct       REAL    NOT NULL,  -- range moyen des M15 du créneau, % du prix
    ratio         REAL    NOT NULL,  -- vol du créneau / vol moyenne de l'asset
    fiabilite     REAL    NOT NULL,  -- part des semaines livrant un mouvement notable
    nb_semaines   INTEGER NOT NULL,  -- effectif (semaines présentes)
    verdict_ia    TEXT,              -- 'ARMER' | 'IGNORER' | NULL (pas encore évalué)
    conviction    INTEGER,           -- 0-100
    justification TEXT,
    arme          INTEGER NOT NULL DEFAULT 0,  -- 1 = annonce synthétique active — TOI seul
    arme_le       INTEGER,
    maj_le        INTEGER NOT NULL,
    PRIMARY KEY (asset, jour, heure)
);
