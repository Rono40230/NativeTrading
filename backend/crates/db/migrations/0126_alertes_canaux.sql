-- Canaux d'alertes par famille (owner 21/10) : source unique lue par
-- l'app principale (OS+son), Telegram et le mobile. Défauts = état
-- existant + décisions owner 21/10 (T-10, presse 100 et contact de zone
-- deviennent aussi des alarmes BUREAU ; briefing + santé idem).
CREATE TABLE IF NOT EXISTS alertes_canaux (
    famille  TEXT PRIMARY KEY,
    app      INTEGER NOT NULL DEFAULT 0,  -- notification OS + son (bureau)
    telegram INTEGER NOT NULL DEFAULT 0,
    mobile   INTEGER NOT NULL DEFAULT 1
);
INSERT OR IGNORE INTO alertes_canaux (famille, app, telegram, mobile) VALUES
    ('evenement_t10',  1, 0, 1),  -- rappel T-10 High
    ('presse_100',     1, 0, 1),  -- gros titre marquant
    ('smc_approche',   0, 1, 1),  -- approche de zone (Telegram déjà en place)
    ('smc_contact',    1, 0, 1),  -- contact de zone (bureau + mobile, Telegram OFF)
    ('alerte_prix',    1, 1, 1),  -- déclenchée
    ('briefing',       1, 0, 1),  -- briefing du matin 8h30
    ('sante',          1, 0, 1),  -- fraîcheur fournisseurs
    ('signaux_smc',        0, 1, 0),  -- signaux PRIS (doctrine mobile 17/10)
    ('signaux_straddle',   0, 1, 0),
    ('signaux_rockets',    0, 1, 0),
    ('signaux_kdj',        0, 1, 0);
