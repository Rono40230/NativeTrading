-- 0072 — Revue de presse : résumé RSS stocké à la collecte (Piste A).
--
-- La modal d'article scrape le contenu via GET /api/news/contenu, mais les
-- sites modernes (Yahoo, CoinTelegraph…) rendent en JavaScript : le scraper
-- récupère la page de cookies, ou rien. Les flux RSS ont TOUJOURS une
-- <description> : on la stocke désormais à la collecte — elle devient le
-- socle d'affichage, le scraper n'est plus qu'un enrichissement.
-- '' pour les lignes antérieures (dégradation, pas de rétro-collecte).

ALTER TABLE presse_articles ADD COLUMN resume_source TEXT NOT NULL DEFAULT '';
