-- Normalisation source 'ib' → 'ig' pour tous les assets IG Markets.
-- Les migrations 0005 et 0006 utilisaient 'ib' (ancienne dénomination Interactive Brokers)
-- mais le validator backend accepte uniquement 'binance' | 'ig'.
UPDATE assets SET source = 'ig' WHERE source = 'ib';
