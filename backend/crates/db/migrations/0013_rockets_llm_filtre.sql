-- Champs LLM sur rockets_signaux pour le filtre pré-sauvegarde (Mode 1)
ALTER TABLE rockets_signaux ADD COLUMN llm_valide INTEGER;      -- 0/1, NULL = pas filtré
ALTER TABLE rockets_signaux ADD COLUMN llm_conviction INTEGER;  -- 0-100
ALTER TABLE rockets_signaux ADD COLUMN llm_raison TEXT;
ALTER TABLE rockets_signaux ADD COLUMN llm_sl_suggere REAL;
ALTER TABLE rockets_signaux ADD COLUMN llm_tp1_suggere REAL;
