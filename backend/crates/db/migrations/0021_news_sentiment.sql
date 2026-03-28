-- Migration 0021 : Cache du sentiment Ollama par article
CREATE TABLE IF NOT EXISTS news_sentiment (
    hash_titre TEXT    PRIMARY KEY,
    sentiment  TEXT    NOT NULL CHECK (sentiment IN ('haussier', 'neutre', 'baissier')),
    analyse_le INTEGER NOT NULL
);
