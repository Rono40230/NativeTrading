CREATE TABLE IF NOT EXISTS news_traductions (
  hash_titre TEXT    PRIMARY KEY,
  titre_fr   TEXT    NOT NULL,
  traduit_le INTEGER NOT NULL
);
