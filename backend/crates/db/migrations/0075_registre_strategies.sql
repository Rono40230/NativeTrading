-- Étape 2 (socle stratégique) : registre des stratégies.
-- La partie STRUCTURELLE du manifeste vit dans le code (crate de chaque
-- stratégie) ; cette table porte la partie PILOTABLE : état de vie, son
-- Telegram, allocation (capital + risque).
CREATE TABLE IF NOT EXISTS strategies (
    id             TEXT PRIMARY KEY,            -- 'SMC', 'straddle', 'rockets'
    etat           TEXT NOT NULL DEFAULT 'Construction'
                   CHECK (etat IN ('Officielle', 'Observation', 'Construction')),
    notifications  INTEGER NOT NULL DEFAULT 0, -- son Telegram (découplé des signaux)
    capital        REAL NOT NULL DEFAULT 0,    -- capital alloué (USD)
    risque_pct     REAL NOT NULL DEFAULT 1.0   -- risque par trade (1-3 %)
);

-- Peuplement initial : SMC officielle et notifiée ; straddle en observation ;
-- rockets en construction.
INSERT OR IGNORE INTO strategies (id, etat, notifications, capital, risque_pct)
    VALUES ('SMC', 'Officielle', 1, 0, 1.0);
INSERT OR IGNORE INTO strategies (id, etat, notifications, capital, risque_pct)
    VALUES ('straddle', 'Observation', 0, 0, 1.0);
INSERT OR IGNORE INTO strategies (id, etat, notifications, capital, risque_pct)
    VALUES ('rockets', 'Construction', 0, 0, 1.0);

-- Renommage de l'identifiant historique : la v12 signait « SmcDirectional »
-- (héritage v1). Décision propriétaire : la stratégie s'appelle « SMC ».
UPDATE signaux SET strategie = 'SMC' WHERE strategie = 'SmcDirectional';

-- Architecture 3 couches : le risque par actif disparaît (décision
-- propriétaire — suppression, pas de surcharge). Le lot se calcule par
-- stratégie à l'émission.
ALTER TABLE asset_params DROP COLUMN risque_pct;
