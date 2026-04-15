-- P5 : Seuils de confiance ML dynamiques par stratégie
-- Valeur = probabilité minimum [0.0-1.0] pour qu'un signal soit publié.
-- Rockets : XGBoost fine-tuné sur trades clôturés → score TP
-- Straddle : LSTM/XGBoost général → skip si trop directionnel (seuil HAUT)
-- SMC      : LSTM/XGBoost général → rejeter si confiance insuffisante
INSERT OR IGNORE INTO configuration (cle, valeur, maj_le) VALUES
    ('seuil_confiance_rockets',  '0.60', unixepoch()),
    ('seuil_confiance_straddle', '0.75', unixepoch()),
    ('seuil_confiance_smc',      '0.60', unixepoch());
