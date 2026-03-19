CREATE TABLE IF NOT EXISTS calendrier_cache (
  id         TEXT    PRIMARY KEY,
  date_heure TEXT    NOT NULL,
  devise     TEXT    NOT NULL,
  titre      TEXT    NOT NULL,
  impact     TEXT    NOT NULL,
  precedent  TEXT,
  prevision  TEXT,
  fetched_at INTEGER NOT NULL
);
