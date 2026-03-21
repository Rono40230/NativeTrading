-- Table des assets gérés par l'application
-- Gestion dynamique : ajout/suppression via les paramètres
CREATE TABLE IF NOT EXISTS assets (
    id      TEXT PRIMARY KEY,           -- "BTC", "XAUUSD", "EURUSD"
    nom     TEXT NOT NULL,              -- "Bitcoin", "Or (Gold)"
    type    TEXT NOT NULL,              -- "crypto" | "metal" | "forex" | "indice"
    source  TEXT NOT NULL DEFAULT 'binance', -- "binance" | "ib"
    actif   INTEGER NOT NULL DEFAULT 1, -- soft delete (0 = supprimé)
    cree_le INTEGER NOT NULL
);

-- Assets par défaut (pré-peuplés)
INSERT OR IGNORE INTO assets (id, nom, type, source, actif, cree_le) VALUES
    ('BTC',    'Bitcoin',            'crypto',  'binance', 1, strftime('%s','now')),
    ('ETH',    'Ethereum',           'crypto',  'binance', 1, strftime('%s','now')),
    ('XAUUSD', 'Or (Gold)',          'metal',   'ib',      1, strftime('%s','now')),
    ('XAGUSD', 'Argent (Silver)',    'metal',   'ib',      1, strftime('%s','now')),
    ('EURUSD', 'Euro / Dollar',      'forex',   'ib',      1, strftime('%s','now')),
    ('GBPJPY', 'Livre / Yen',        'forex',   'ib',      1, strftime('%s','now')),
    ('CADJPY', 'CAD / Yen',          'forex',   'ib',      1, strftime('%s','now')),
    ('NZDJPY', 'NZD / Yen',          'forex',   'ib',      1, strftime('%s','now')),
    ('USDCAD', 'Dollar / CAD',       'forex',   'ib',      1, strftime('%s','now')),
    ('USDJPY', 'Dollar / Yen',       'forex',   'ib',      1, strftime('%s','now')),
    ('DAX',    'DAX 40 (Allemagne)', 'indice',  'ib',      1, strftime('%s','now')),
    ('SP500',  'S&P 500',            'indice',  'ib',      1, strftime('%s','now'));
