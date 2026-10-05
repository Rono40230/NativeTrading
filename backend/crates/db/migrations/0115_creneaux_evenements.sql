-- 0115 (28/09) — créneaux ÉVÉNEMENTIELS straddle (phase 3).
-- Décisions propriétaire 28/09 : (1) balayage large — tous les événements
-- de la taxonomie armés sur tous les assets du périmètre ; (2) REMPLACEMENT
-- des créneaux statistiques — la table creneaux_ia devient une ARCHIVE
-- (lignes conservées, désarmées) ; (3) XAGUSD entre au périmètre (seed en
-- Rust au boot : la liste vit dans la config).
--
-- Contrairement aux créneaux statistiques (jour+heure Paris figés), un
-- créneau événement est défini par l'IDENTIFIANT de l'événement : l'heure
-- Paris est recalculée à la volée depuis le fuseau d'origine — les
-- bascules d'heure d'été sont suivies automatiquement.

CREATE TABLE IF NOT EXISTS creneaux_evenements (
  asset TEXT NOT NULL,
  evenement TEXT NOT NULL,           -- ident de la taxonomie (evenements.rs)
  arme INTEGER NOT NULL DEFAULT 0,
  arme_le INTEGER,                   -- armement = nouveau test, compteurs à 0
  occurrences INTEGER NOT NULL DEFAULT 0,
  somme_r REAL NOT NULL DEFAULT 0,
  verdict_test TEXT,                 -- valide | refute | incertain
  conclut_le INTEGER,
  PRIMARY KEY (asset, evenement)
);
CREATE INDEX IF NOT EXISTS idx_creneaux_evenements_armes
  ON creneaux_evenements(arme);

-- Remplacement : les créneaux statistiques armés sont désarmés une fois
-- pour toutes. Les lignes (et leurs verdicts §16-b) restent lisibles en
-- archive côté API.
UPDATE creneaux_ia SET arme = 0 WHERE arme = 1;
