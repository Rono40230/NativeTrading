-- Phase 5.2 — purge des bougies MT5 décalées : l'horloge du serveur Axi est
-- en GMT+3 (heure de New York alignée) et l'EA poussait les estampilles
-- serveur telles quelles → toutes les bougies MT5 sont 3 h dans le futur
-- (constat 25/08 12:51 locales : dernières bougies « 15:45 »). Les moteurs,
-- les fraîcheurs et les passes straddle (annonces UTC) exigeaient la
-- vérité UTC. L'EA convertit désormais serveur→UTC ; l'historique est
-- repoussé par l'EA au cycle suivant (~14 min, preuve faite).
DELETE FROM bougies WHERE source = 'mt5';
