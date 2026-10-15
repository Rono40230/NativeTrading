-- 0122 — Catalogue des créneaux fixes ÉDITABLE (owner 15/10) : les événements
-- structurels quittent le code (evenements.rs) pour la base — la modale
-- « Gérer les créneaux fixes » (CRUD) les crée/modifie/supprime et la matrice
-- suit automatiquement. Les 9 types 📅 (cal_*) restent en code : leur heure
-- vient de la VRAIE annonce du calendrier (doctrine 14/10), pas d'un créneau.
--
-- Seed = les 8 structurels existants (mêmes heures/fuseaux/jours que le code
-- du 14/10) + le nouveau créneau owner 15/10 : réouverture de la cotation or
-- Axi, lundi 00h00 Paris (données réelles : pause quotidienne 22h58→00h00,
-- reprise hebdo lundi minuit — voir étude 15/10).
-- AUCUNE ligne creneaux_evenements n'est créée pour or_reouverture :
-- doctrine « désarmé à la création » (l'armement reste au propriétaire seul).
-- tz = nom IANA ; heure/minute = heure LOCALE du fuseau (DST par chrono-tz) ;
-- jours = JSON [1..7] ISO lundi=1..dimanche=7 dans le fuseau de l'événement.

CREATE TABLE IF NOT EXISTS evenements_catalogue (
    ident       TEXT PRIMARY KEY,   -- slug stable (clé des creneaux_evenements)
    nom         TEXT NOT NULL,
    detail      TEXT NOT NULL DEFAULT '',
    tz          TEXT NOT NULL,      -- IANA : 'Europe/Paris', 'America/New_York'…
    heure       INTEGER NOT NULL CHECK (heure BETWEEN 0 AND 23),
    minute      INTEGER NOT NULL DEFAULT 0 CHECK (minute BETWEEN 0 AND 59),
    jours       TEXT NOT NULL DEFAULT '[1,2,3,4,5]',
    actif       INTEGER NOT NULL DEFAULT 1,
    ordre       INTEGER NOT NULL DEFAULT 0,
    modifie_le  INTEGER NOT NULL DEFAULT 0
);

INSERT OR IGNORE INTO evenements_catalogue (ident, nom, detail, tz, heure, minute, jours, actif, ordre) VALUES
('eu_ouverture', 'Ouverture Francfort + Londres',
 'Xetra ouvre à 09:00 Paris et la session forex de Londres à 08:00 UK — la même minute. Le DAX y prend son plus gros range de la matinée européenne.',
 'Europe/Paris', 9, 0, '[1,2,3,4,5]', 1, 1),
('lbma_am', 'Fix LBMA matin (or)',
 'Enchère de fixing de l''or de Londres (10:30 UK) : les banques concentrent leurs ordres, l''or et l''argent bougent à la minute.',
 'Europe/London', 10, 30, '[1,2,3,4,5]', 1, 2),
('nyse_ouverture', 'Ouverture NYSE',
 'Ouverture de la séance actions américaine (9:30 New York) : les indices et tout ce qui s''y réfère cherchent leur prix d''équilibre.',
 'America/New_York', 9, 30, '[1,2,3,4,5]', 1, 3),
('nyse_cloture', 'Clôture NYSE',
 'Sonnerie de clôture (16:00 New York) : derniers équilibres du jour sur les indices US.',
 'America/New_York', 16, 0, '[1,2,3,4,5]', 1, 4),
('londres_cloture', 'Clôture Londres (forex)',
 'Fin de la session européenne à 17:00 UK : la liquidité forex se raréfie d''un coup avant le relais américain de l''après-midi.',
 'Europe/London', 17, 0, '[1,2,3,4,5]', 1, 5),
('cme_reouverture', 'Réouverture CME (or, indices)',
 'Fin de la pause quotidienne 17:00-18:00 New York : les futures métaux et indices reprennent — première minute la plus nerveuse de la journée pour l''or et l''argent.',
 'America/New_York', 18, 0, '[1,2,3,4,5]', 1, 6),
('marche_reouverture_hebdo', 'Réouverture week-end (forex, CFD)',
 'Dimanche 17:00 New York : après ~47 h de fermeture, les cotations MT5 reprennent — le gap de week-end se prix immédiatement.',
 'America/New_York', 17, 0, '[7]', 1, 7),
('asie_ouverture', 'Ouverture Tokyo (session asiatique)',
 '9h00 Tokyo : le relais asiatique prend la liquidité — 2h00 Paris en été, 1h00 en hiver (Tokyo ne bascule jamais d''heure, Paris oui).',
 'Asia/Tokyo', 9, 0, '[1,2,3,4,5]', 1, 8),
('or_reouverture', 'Réouverture or (pause Axi)',
 'Reprise de la cotation XAUUSD chez Axi à 00h00 Paris après la pause quotidienne de 22h58 (données réelles 15/10) — le gap se prix immédiatement. Lundi uniquement pour la reprise hebdo ; à revérifier après la bascule d''heure d''octobre.',
 'Europe/Paris', 0, 0, '[1]', 1, 9);
