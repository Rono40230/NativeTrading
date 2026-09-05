-- §16 roadmap : journal de bord du propriétaire — fil de notes horodatées
-- attachées aux trades (contexte, ressenti, décision). Append-only avec
-- suppression d'entrée (pas d'édition : un journal ne se réécrit pas).
-- Matière première future pour l'analyse IA (§8).
CREATE TABLE IF NOT EXISTS journal_bord (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    signal_id TEXT NOT NULL,
    note TEXT NOT NULL,
    cree_le INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_journal_bord_signal ON journal_bord(signal_id, cree_le);
