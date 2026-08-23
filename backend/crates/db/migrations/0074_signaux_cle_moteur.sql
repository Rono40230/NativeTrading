-- Phase 2.8 — bascule officielle : les signaux v12 du runtime deviennent les
-- signaux officiels. cle_moteur = clé stable du trade côté moteur (anti-ré-émission)
-- pour fermer la ligne (statut 'Fermé') à l'événement de clôture correspondant.
ALTER TABLE signaux ADD COLUMN cle_moteur TEXT;
