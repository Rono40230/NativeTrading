-- §Relais mobile (chantier MOB owner 17-18/10) — le journal serveur de
-- l'app APK : chaque alerte émise par les moteurs y est écrite, l'app
-- mobile la lit (polling) et décide seule de sonner (muette 21h→8h côté
-- CLIENT — le serveur écrit tout, la ligne « pendant la nuit » du digest
-- 8h30 en dépend).
CREATE TABLE IF NOT EXISTS mobile_alertes (
    id      INTEGER PRIMARY KEY AUTOINCREMENT,
    ts      INTEGER NOT NULL,             -- epoch s émission (UTC)
    type    TEXT    NOT NULL,             -- digest | evenement | presse | smc_approche | smc_contact | prix | sante
    titre   TEXT    NOT NULL,             -- ligne résumée (journal mobile)
    asset   TEXT,
    tf      TEXT,
    payload TEXT    NOT NULL,             -- JSON : détails de la vue mobile
    png     INTEGER NOT NULL DEFAULT 0,   -- 1 = capture servie par /api/mobile/alerte/{id}/png
    cree_le INTEGER NOT NULL DEFAULT (strftime('%s','now'))
);
CREATE INDEX IF NOT EXISTS idx_mobile_alertes_ts ON mobile_alertes(ts DESC);

-- Seuil d'approche de zone aligné sur la spec mobile owner 17/10 (0,5 ATR) :
-- le moteur d'alertes zones est LA source unique (Telegram + app + mobile).
INSERT OR REPLACE INTO configuration (cle, valeur, maj_le)
VALUES ('smc_alertes_zones_seuil_atr', '0.5', strftime('%s','now'));

-- Token d'accès mobile : vide = ouvert (Tailscale privé par conception) ;
-- une valeur exige « Authorization: Bearer <token> » sur /api/mobile/*.
INSERT OR IGNORE INTO configuration (cle, valeur, maj_le)
VALUES ('mobile_token', '', strftime('%s','now'));
