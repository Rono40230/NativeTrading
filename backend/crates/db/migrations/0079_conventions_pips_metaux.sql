-- Fix conventions métaux : taille_pip incohérente avec valeur_pips.
--
-- XAGUSD : valeur 50 $/pip/lot correspond à 1 lot = 5 000 oz × 0,01 $ —
--   mais taille_pip valait 1.0 : un stop < 0,50 $ s'arrondissait à 0 pip
--   → lot = 0 dans les messages Telegram (signalement 24/08).
-- XAUUSD : valeur 10 $/pip/lot correspond à 1 lot = 100 oz × 0,1 $ —
--   taille_pip valait 1.0 (lots non nuls mais convention décalée ×10).
-- Le calcul du lot passe par ailleurs sur la distance EXACTE (plus
-- d'arrondi intermédiaire) — voir signaux_officiels.formater_message.
UPDATE asset_params SET taille_pip = 0.1  WHERE asset = 'XAUUSD';
UPDATE asset_params SET taille_pip = 0.01 WHERE asset = 'XAGUSD';
