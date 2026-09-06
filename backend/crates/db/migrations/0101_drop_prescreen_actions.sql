-- §7-3a (décision 05/09) : DROP de l'entonnoir prescreen_actions — journal
-- write-only depuis la suppression de l'endpoint GET /api/rockets/actions/
-- prescreen (aucun lecteur restant, le scanner n'écrit plus dedans).
DROP TABLE IF EXISTS prescreen_actions;
