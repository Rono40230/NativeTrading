-- §4 (09/09, décision propriétaire) — véto unlocks rockets : un candidat
-- crypto avec un déverrouillage de tokens daté à moins de N jours (kv
-- `rockets_unlock_jours`, défaut 30) est ÉLIMINATOIRE — l'offre libérée
-- casse la dynamique de cassure. Sources : analyste IA lisant les dépêches
-- déjà collectées (titre FR) + saisie propriétaire (endpoint). Les API
-- publiques (DefiLlama/CryptoRank) sont derrière paywall/Cloudflare —
-- zéro dépendance externe fragile.
CREATE TABLE IF NOT EXISTS unlocks_prochains (
    symbole     TEXT    NOT NULL,   -- ticker base sans USDT (ex ARB)
    date_unlock INTEGER NOT NULL,   -- epoch jour 00:00 UTC
    usd_estime  REAL,               -- si mentionné par la source
    source      TEXT    NOT NULL,   -- titre de la dépêche (tronqué) ou 'saisie propriétaire'
    maj_le      INTEGER NOT NULL,
    PRIMARY KEY (symbole, date_unlock)
);
