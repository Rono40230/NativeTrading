-- Abaisse le seuil de volume minimum pour le pipeline LLM Rockets.
-- Avant : 1.5 (bloquait quasi tous les candidats breakout/prelancement)
-- Après : 1.0 (laisse passer les setups avec momentum modéré)
UPDATE rockets_config
SET    ratio_volume_min = 1.0
WHERE  ratio_volume_min > 1.0;
