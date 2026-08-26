-- Nettoyage Dukascopy (décision propriétaire 26/08) : le datafeed public
-- n'est plus utilisé — crypto = Bybit WS, métaux/forex/indices = MT5/Axi
-- (EA + push historique). La colonne de routage datafeed_dukascopy perd
-- son dernier lecteur : suppression.
ALTER TABLE assets DROP COLUMN datafeed_dukascopy;
