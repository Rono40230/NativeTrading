-- Migration 0055 : Suivi temps-réel des jambes Straddle (moniteur live price)
-- Complète le job candle-based : gère trailing stop + état courant par jambe.

CREATE TABLE IF NOT EXISTS straddle_suivi_position (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    signal_id    TEXT    NOT NULL REFERENCES signaux(id),
    jambe        TEXT    NOT NULL,       -- 'LONG' | 'SHORT'
    atr          REAL    NOT NULL,       -- ATR au moment du signal (1R = sl_mult × atr)
    peak         REAL    NOT NULL DEFAULT 0.0, -- prix max favorable depuis entrée
    sl_effectif  REAL    NOT NULL,       -- SL courant (BE → TP1 → trailing)
    statut_jambe TEXT    NOT NULL DEFAULT 'actif', -- 'actif' | 'tp1_touche' | 'tp2_touche' | 'cloturee'
    prix_tp1     REAL,                   -- prix auquel TP1 a été touché
    prix_tp2     REAL,                   -- prix auquel TP2 a été touché
    prix_cloture REAL,                   -- prix de clôture finale
    pnl_r_final  REAL,                   -- P&L final en R (1R = risque initial)
    maj_le       INTEGER NOT NULL DEFAULT (unixepoch()),
    UNIQUE(signal_id, jambe)
);

CREATE INDEX IF NOT EXISTS idx_ssp_signal ON straddle_suivi_position(signal_id);
CREATE INDEX IF NOT EXISTS idx_ssp_statut ON straddle_suivi_position(statut_jambe);
