-- Ajout de tous les assets du catalogue (actif = 0 par défaut pour les nouveaux)
INSERT OR IGNORE INTO assets (id, nom, type, source, actif, cree_le) VALUES
    -- Crypto (Binance)
    ('SOL',     'Solana',               'crypto', 'binance', 0, strftime('%s','now')),
    ('BNB',     'Binance Coin',         'crypto', 'binance', 0, strftime('%s','now')),
    ('XRP',     'Ripple',               'crypto', 'binance', 0, strftime('%s','now')),
    ('ADA',     'Cardano',              'crypto', 'binance', 0, strftime('%s','now')),
    ('DOGE',    'Dogecoin',             'crypto', 'binance', 0, strftime('%s','now')),
    ('AVAX',    'Avalanche',            'crypto', 'binance', 0, strftime('%s','now')),
    ('LINK',    'Chainlink',            'crypto', 'binance', 0, strftime('%s','now')),
    ('DOT',     'Polkadot',             'crypto', 'binance', 0, strftime('%s','now')),
    -- Métaux (IB)
    ('XPTUSD',  'Platine / Dollar',     'metal',  'ib',      0, strftime('%s','now')),
    ('XPDUSD',  'Palladium / Dollar',   'metal',  'ib',      0, strftime('%s','now')),
    -- Forex (IB)
    ('GBPUSD',  'Livre / Dollar',       'forex',  'ib',      0, strftime('%s','now')),
    ('USDCHF',  'Dollar / Franc suisse','forex',  'ib',      0, strftime('%s','now')),
    ('AUDUSD',  'Dollar australien / Dollar', 'forex', 'ib', 0, strftime('%s','now')),
    ('NZDUSD',  'NZD / Dollar',         'forex',  'ib',      0, strftime('%s','now')),
    ('EURJPY',  'Euro / Yen',           'forex',  'ib',      0, strftime('%s','now')),
    ('EURGBP',  'Euro / Livre',         'forex',  'ib',      0, strftime('%s','now')),
    -- Indices (IB)
    ('NAS100',  'Nasdaq 100',           'indice', 'ib',      1, strftime('%s','now')),
    ('US30',    'Dow Jones',            'indice', 'ib',      0, strftime('%s','now')),
    ('FTSE100', 'FTSE 100 (UK)',        'indice', 'ib',      0, strftime('%s','now')),
    ('CAC40',   'CAC 40 (France)',      'indice', 'ib',      0, strftime('%s','now')),
    ('JP225',   'Nikkei 225',           'indice', 'ib',      0, strftime('%s','now'));
