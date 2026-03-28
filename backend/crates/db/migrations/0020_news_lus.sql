-- Migration 0020 : Persistance des articles de news lus
CREATE TABLE IF NOT EXISTS news_lus (
    url   TEXT    PRIMARY KEY,
    lu_le INTEGER NOT NULL   -- timestamp UNIX secondes
);
