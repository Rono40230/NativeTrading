-- §16-b (07/09) — boucle de validation des créneaux armés : chaque créneau
-- tire chaque semaine, ses passes (signaux.cle_moteur = straddle-asset-ts)
-- sont agrégées et la boucle quotidienne statue au bout de N occurrences
-- (kv creneaux_test_min, défaut 4) : ΣR > 0 → valide (pilier, reste armé),
-- ΣR ≤ plancher (kv creneaux_test_plancher_r, défaut −1,5) ou 0 gagnant →
-- réfuté (désarmé, slot libéré), entre les deux → incertain (prolongé
-- 2 tirages puis tranché). L'armement reste au propriétaire SEUL.
ALTER TABLE creneaux_ia ADD COLUMN occurrences  INTEGER NOT NULL DEFAULT 0;  -- passes closes depuis l'armement
ALTER TABLE creneaux_ia ADD COLUMN somme_r      REAL    NOT NULL DEFAULT 0;  -- ΣR réalisé de ces passes
ALTER TABLE creneaux_ia ADD COLUMN verdict_test TEXT;   -- NULL = en test | 'valide' | 'refute' | 'incertain'
ALTER TABLE creneaux_ia ADD COLUMN conclut_le   INTEGER;
