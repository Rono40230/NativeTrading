-- Migration 0045 : Flag de suivi des notifications Telegram.
-- Permet au worker centralisé de détecter les signaux non encore notifiés
-- et de les envoyer de façon robuste (retry au redémarrage, zéro perte silencieuse).

ALTER TABLE signaux ADD COLUMN telegram_envoye INTEGER NOT NULL DEFAULT 0;
ALTER TABLE rockets_signaux ADD COLUMN telegram_envoye INTEGER NOT NULL DEFAULT 0;
