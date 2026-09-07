-- §8 (07/09) — journalisation du détail de qualification des setups SMC.
-- Un instantané JSON par signal émis : source (OB/BSZones), score de la
-- zone, force, qualité de zone, sweep frais, premium/discount, score live
-- et SES composantes actives (miroir du diagFlags MQL5). La matière
-- première de l'analyse des setups qui meurent (§8) et des 7 features
-- contextuelles ML (§11). Journalisation en lecture seule — jamais
-- consommée par la décision.
CREATE TABLE IF NOT EXISTS smc_scoring_detail (
    signal_id   TEXT PRIMARY KEY REFERENCES signaux(id),
    detail_json TEXT NOT NULL,
    cree_le     INTEGER NOT NULL
);
