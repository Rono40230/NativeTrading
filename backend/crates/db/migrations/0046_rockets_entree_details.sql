-- Migration 0046 : Ajout des champs d'entrée et d'invalidation dans rockets_signaux.
-- Ces colonnes permettent au worker Telegram centralisé d'envoyer le format détaillé complet
-- (Entrée Limite, Entrée Stop, Entrée idéale, Invalidation) sans recalcul post-insertion.

ALTER TABLE rockets_signaux ADD COLUMN entree_limite REAL;
ALTER TABLE rockets_signaux ADD COLUMN entree_stop REAL;
ALTER TABLE rockets_signaux ADD COLUMN niveau_invalidation REAL;
ALTER TABLE rockets_signaux ADD COLUMN type_entree_rec TEXT;
