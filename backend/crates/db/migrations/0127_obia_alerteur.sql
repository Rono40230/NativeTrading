-- OBIA P2 étape 1 (GO owner 26/10) : alerteur espion maison.
-- obia_alertes = idempotence par (zone, TF, bougie) + historique pour la
-- parité espion↔portail (étape 2) ; obia_contact = famille de canaux,
-- tout actif pour la phase de validation (matrice ajustable dans la modale).
CREATE TABLE IF NOT EXISTS obia_alertes (
    zone_id   TEXT NOT NULL,
    tf        TEXT NOT NULL,
    candle_ts INTEGER NOT NULL,
    action    TEXT NOT NULL,           -- 'alerte' | 'retiree'
    emis_le   INTEGER NOT NULL,
    PRIMARY KEY (zone_id, tf, candle_ts)
);
INSERT OR IGNORE INTO alertes_canaux (famille, app, telegram, mobile) VALUES
    ('obia_contact', 1, 1, 1);
