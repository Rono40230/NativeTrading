-- Activation de SOL par défaut (Solana sur Binance — entièrement supporté)
UPDATE assets SET actif = 1 WHERE id = 'SOL' AND actif = 0;
