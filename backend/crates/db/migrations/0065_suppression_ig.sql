-- 0065 — Suppression d'IG Markets et Twelve Data comme sources de données.
--
-- IG et TwelveData sont abandonnés (remplacés à terme par Dukascopy).
-- Les colonnes restent en base (pas de destruction de schéma), seul le
-- contenu lié à ces providers est nettoyé.

-- Retirer les clés IG et TwelveData de la configuration
DELETE FROM configuration WHERE cle IN (
    'ig_api_key', 'ig_username', 'ig_password', 'ig_env', 'twelvedata_api_key'
);
-- Interrupteur du worker IG REST (supprimé du code)
DELETE FROM configuration WHERE cle = 'worker_actif_ig';

-- Retirer epic_ig des assets (la colonne reste mais vidée)
UPDATE assets SET epic_ig = NULL;

-- Marquer les assets source='ig' comme inactifs (ils n'ont plus de provider)
UPDATE assets SET actif = 0 WHERE source = 'ig';
