-- Purge du worker Bybit WS (bascule MT5/Axi totale, owner 24-25/10) : le
-- module et son interrupteur disparaissent — la config morte avec lui.
DELETE FROM configuration WHERE cle = 'worker_actif_bybit';
