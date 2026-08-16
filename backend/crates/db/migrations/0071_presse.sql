-- 0071 — Revue de presse (Phase 4.1, spec 2026-08-15).
--
-- presse_sources : flux RSS pilotables par l'utilisateur (comme les assets).
-- presse_articles : la bibliothèque. statut_traduction : 'non_tente' | 'ok' ;
--   les échecs ×2 mènent à la SUPPRESSION de la ligne (règle "porte d'entrée").
-- presse_briefs : briefs générés à la demande (aucun écrasement).

CREATE TABLE presse_sources (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    nom         TEXT NOT NULL,
    url_rss     TEXT NOT NULL UNIQUE,
    poids_score INTEGER NOT NULL DEFAULT 30,
    categorie   TEXT NOT NULL DEFAULT 'marches',
    actif       INTEGER NOT NULL DEFAULT 1,
    cree_le     INTEGER NOT NULL
);

CREATE TABLE presse_articles (
    hash_titre          TEXT PRIMARY KEY,
    titre               TEXT NOT NULL,
    url                 TEXT NOT NULL,
    source_nom          TEXT NOT NULL,
    publie_le           TEXT NOT NULL,
    score               INTEGER NOT NULL,
    theme               TEXT NOT NULL,
    assets_concernes    TEXT NOT NULL DEFAULT '[]',
    impact              TEXT NOT NULL DEFAULT 'faible',
    statut_traduction   TEXT NOT NULL DEFAULT 'non_tente',
    tentatives_traduction INTEGER NOT NULL DEFAULT 0,
    lu                  INTEGER NOT NULL DEFAULT 0,
    ajoute_le           INTEGER NOT NULL
);
CREATE INDEX idx_presse_articles_ajoute ON presse_articles(ajoute_le DESC);
CREATE INDEX idx_presse_articles_theme ON presse_articles(theme);

CREATE TABLE presse_briefs (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    genere_le   INTEGER NOT NULL,
    fenetre_de  INTEGER NOT NULL,
    fenetre_a   INTEGER NOT NULL,
    nb_articles INTEGER NOT NULL,
    contenu     TEXT NOT NULL
);

-- Point de départ : les 9 flux historiques (modifiables/supprimables ensuite)
INSERT INTO presse_sources (nom, url_rss, poids_score, categorie, cree_le) VALUES
('Reuters Business',  'https://feeds.reuters.com/reuters/businessNews', 40, 'marches', strftime('%s','now')),
('CNBC Markets',      'https://search.cnbc.com/rs/search/combinedcms/view.xml?partnerId=wrss01&id=10000664', 35, 'marches', strftime('%s','now')),
('MarketWatch',       'https://feeds.marketwatch.com/marketwatch/marketpulse/', 35, 'marches', strftime('%s','now')),
('FXStreet',          'https://www.fxstreet.com/rss/news', 38, 'forex', strftime('%s','now')),
('Kitco Métaux',      'https://www.kitco.com/rss/KitcoNewsRSS.xml', 38, 'metaux', strftime('%s','now')),
('CoinTelegraph',     'https://cointelegraph.com/rss', 30, 'crypto', strftime('%s','now')),
('CryptoNews',        'https://cryptonews.com/news/feed', 28, 'crypto', strftime('%s','now')),
('Decrypt',           'https://decrypt.co/feed', 30, 'crypto', strftime('%s','now')),
('Yahoo Finance',     'https://finance.yahoo.com/news/rssindex', 28, 'marches', strftime('%s','now'));

INSERT INTO configuration (cle, valeur, maj_le) VALUES
('retention_presse_mois', '12', strftime('%s','now'))
ON CONFLICT(cle) DO UPDATE SET valeur = excluded.valeur, maj_le = excluded.maj_le;
