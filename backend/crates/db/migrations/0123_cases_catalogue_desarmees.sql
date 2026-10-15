-- 0123 — Correctif chantier EVT (owner 15/10 soir) : « désarmé à la
-- création » avait été implémenté comme AUCUNE ligne creneaux_evenements —
-- or la matrice ne dessine une cellule que si la ligne existe : impossible
-- d'armer la « Réouverture or ». La bonne sémantique : les cellules
-- EXISTENT, désarmées (arme = 0, compteurs vierges), prêtes au clic.
--
-- Rattrapage générique et idempotent : une ligne désarmée pour tout couple
-- (asset du périmètre × créneau du catalogue) qui manque — les lignes
-- existantes (armées ou non) ne sont JAMAIS touchées.

INSERT OR IGNORE INTO creneaux_evenements (asset, evenement, arme)
SELECT je.value, c.ident, 0
FROM evenements_catalogue c,
     configuration cfg,
     json_each(cfg.valeur) je
WHERE cfg.cle = 'perimetre_straddle';
