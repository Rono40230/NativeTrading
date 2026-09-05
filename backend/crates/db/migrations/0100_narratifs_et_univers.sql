-- §7 roadmap — univers actions par narratif + liquidité (septembre 2026).
--
-- Trois décisions propriétaires :
--   1. La file de backfill n'est plus alphabétique : les pionniers narratifs
--      passent avant le reste (le quota Tiingo gratuit — 500 symboles uniques
--      par mois — va d'abord aux zones à décollages).
--   2. L'univers ACTIF est plafonné (défaut 450) par liquidité : dollar-volume
--      moyen ≥ 2 M$/j, prix ≥ 5 $, ≥ 40 séances sur 63 (calendrier de
--      rafraîchissement quotidien dans le quota).
--   3. Un ticker dont Tiingo répond "aucune donnée" est marqué 'sans_donnees'
--      et n'est plus jamais retenté (fin de la file empoisonnée par les
--      delistings — BDCI, BDJ…).
--
-- Thèmes : T1 = structurels (cœur), T2 = cycles (surpondération en phase
-- ascendante), T3 = hypes (queue d'univers, jamais en cœur). L'étiquette est
-- informative et priorise la file ; l'appartenance à l'univers reste soumise
-- au filtre de liquidité (personne n'entre sans être tradable). Maintenue par
-- le propriétaire — l'IA propose des candidats, lui seul ajoute.

CREATE TABLE IF NOT EXISTS narratifs (
    ticker TEXT NOT NULL,
    theme  TEXT NOT NULL,
    PRIMARY KEY (ticker, theme)
);

-- T1 — électricité pour l'IA : le goulot n'est plus le GPU, c'est le mégawatt.
INSERT OR IGNORE INTO narratifs (ticker, theme) VALUES
    ('CEG','electricite_ia'), ('VST','electricite_ia'), ('TLN','electricite_ia'),
    ('NRG','electricite_ia'), ('GEV','electricite_ia'), ('AES','electricite_ia'),
    ('OKLO','electricite_ia'), ('SMR','electricite_ia'), ('NNE','electricite_ia'),
    ('ETN','electricite_ia'), ('PWR','electricite_ia'), ('HUBB','electricite_ia'),
    ('SRE','electricite_ia'), ('AEE','electricite_ia'), ('XEL','electricite_ia'),
    ('WEC','electricite_ia'), ('NEE','electricite_ia'), ('D','electricite_ia'),
    ('ED','electricite_ia'), ('SO','electricite_ia');

-- T1 — infrastructure IA round 2 : optique, refroidissement, mémoire, semi.
INSERT OR IGNORE INTO narratifs (ticker, theme) VALUES
    ('ANET','infra_ia'), ('COHR','infra_ia'), ('LITE','infra_ia'), ('CIEN','infra_ia'),
    ('CRDO','infra_ia'), ('ALAB','infra_ia'), ('FABL','infra_ia'), ('VRT','infra_ia'),
    ('MU','infra_ia'), ('CRUS','infra_ia'), ('ARM','infra_ia'), ('AVGO','infra_ia'),
    ('NVDA','infra_ia'), ('AMD','infra_ia'), ('MRVL','infra_ia'), ('AMAT','infra_ia'),
    ('LRCX','infra_ia'), ('KLAC','infra_ia'), ('ASML','infra_ia'), ('TER','infra_ia'),
    ('ENTG','infra_ia'), ('CDNS','infra_ia'), ('SNPS','infra_ia');

-- T1 — défense & drones : budgets structurels Europe/US.
INSERT OR IGNORE INTO narratifs (ticker, theme) VALUES
    ('KTOS','defense_drones'), ('AVAV','defense_drones'), ('PLTR','defense_drones'),
    ('RTX','defense_drones'), ('NOC','defense_drones'), ('GD','defense_drones'),
    ('LMT','defense_drones'), ('LDOS','defense_drones'), ('BWXT','defense_drones'),
    ('HII','defense_drones'), ('TXT','defense_drones');

-- T1 — space : constellations D2C, lanceurs, imagerie.
INSERT OR IGNORE INTO narratifs (ticker, theme) VALUES
    ('ASTS','space'), ('RKLB','space'), ('IRDM','space'), ('GSAT','space'),
    ('LUNR','space'), ('RDW','space'), ('PL','space'), ('SATL','space');

-- T2 — mémoire & stockage (cycle — tenir tant que les CapEx IA montent).
INSERT OR IGNORE INTO narratifs (ticker, theme) VALUES
    ('MU','memoire'), ('STX','memoire'), ('WDC','memoire');

-- T2 — crypto cotée (corrélée au BTC — synergie avec les signaux SMC crypto).
INSERT OR IGNORE INTO narratifs (ticker, theme) VALUES
    ('COIN','crypto'), ('MSTR','crypto'), ('CIFR','crypto'), ('WULF','crypto'),
    ('CLSK','crypto'), ('MARA','crypto'), ('RIOT','crypto'), ('HUT','crypto'),
    ('CORZ','crypto');

-- T2 — santé : GLP-1 et oraux 2027, chirurgie robotique.
INSERT OR IGNORE INTO narratifs (ticker, theme) VALUES
    ('LLY','sante'), ('VKTX','sante'), ('AMGN','sante'), ('ISRG','sante');

-- T3 — quantum : décollages violents, revenus dérisoires — queue d'univers.
INSERT OR IGNORE INTO narratifs (ticker, theme) VALUES
    ('IONQ','quantum'), ('RGTI','quantum'), ('QBTS','quantum');

-- T3 — robotique : option value, micro-caps — queue d'univers.
INSERT OR IGNORE INTO narratifs (ticker, theme) VALUES
    ('TER','robotique');
