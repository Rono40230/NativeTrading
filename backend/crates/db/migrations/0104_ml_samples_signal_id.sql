-- §11 étape 1 (06/09) — boucle ML v2 : réalimenter ml_training_samples.
-- Colonne signal_id + index unique partiel → idempotence parfaite : le
-- rattrapage peut rejouer tout l'historique à chaque boot sans doublon, et
-- le collecteur continu (branché à fermer_signal_par_cle) ne double jamais
-- une clôture. NULL autorisé (lignes héritées éventuelles).
ALTER TABLE ml_training_samples ADD COLUMN signal_id TEXT;
CREATE UNIQUE INDEX IF NOT EXISTS idx_ml_samples_signal
    ON ml_training_samples(signal_id) WHERE signal_id IS NOT NULL;
