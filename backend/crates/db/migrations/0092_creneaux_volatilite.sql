-- Créneaux de volatilité (dashboard 27/08) : par asset et heure Paris,
-- statistiques glissantes sur l'historique (24 mois de M15). Rafraîchi
-- chaque nuit par le job creneaux_job — le dashboard lit ce cache.
CREATE TABLE IF NOT EXISTS creneaux_volatilite (
    asset           TEXT NOT NULL,
    -- Heure Paris 0-23 (le créneau couvre [heure, heure+1[)
    heure           INTEGER NOT NULL,
    -- Range moyen des bougies M15 du créneau, en % du prix (comparable
    -- entre XAU, BTC, DAX…).
    vol_pct         REAL NOT NULL,
    -- Part des jours où le créneau a livré un mouvement notable
    -- (barre ≥ 1,5 × la barre médiane de l'asset).
    fiabilite       REAL NOT NULL,
    nb_jours        INTEGER NOT NULL,
    maj_le          INTEGER NOT NULL,
    PRIMARY KEY (asset, heure)
);
