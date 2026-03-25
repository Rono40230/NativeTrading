-- Champs LLM sur signaux pour le filtre pré-sauvegarde SMC Directionnel
ALTER TABLE signaux ADD COLUMN llm_valide     INTEGER;  -- 1=validé | 0=rejeté | NULL=LLM indispo
ALTER TABLE signaux ADD COLUMN llm_conviction INTEGER;  -- 0–100
ALTER TABLE signaux ADD COLUMN llm_raison     TEXT;
ALTER TABLE signaux ADD COLUMN llm_sl_suggere  REAL;
ALTER TABLE signaux ADD COLUMN llm_tp1_suggere REAL;
