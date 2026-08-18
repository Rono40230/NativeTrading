-- Référence veille du bloc Sentiment de Marché (décision propriétaire 2026-08-18 :
-- plus AUCUN flux tendu dans le bloc — les listes USA/Europe/matières/cryptos/VIX
-- affichent les clôtures de la veille, comme les jauges composites).
--
-- Un worker (cycle 30 min, partagé avec le composite) fige les clôtures
-- quotidiennes : Yahoo Finance interval=1d (indices, matières premières, VIX —
-- seules les barres CLÔTURÉES sont lues) et la DB locale D1 Bybit (BTC, ETH).
-- La date est celle de la clôture propre à chaque entité (vendredi pour les
-- indices le lundi matin, la veille pour les cryptos 24/7).
CREATE TABLE IF NOT EXISTS sentiment_marche_veille (
    date TEXT NOT NULL,          -- jour de la clôture (UTC, YYYY-MM-DD)
    entite TEXT NOT NULL,        -- 'S&P500', 'Nasdaq', 'Bitcoin', 'VIX', ...
    groupe TEXT NOT NULL,        -- 'usa' | 'europe' | 'matieres_premieres' | 'cryptos' | 'vix'
    prix REAL NOT NULL,
    variation_pct REAL NOT NULL,
    PRIMARY KEY (date, entite)
) WITHOUT ROWID;
