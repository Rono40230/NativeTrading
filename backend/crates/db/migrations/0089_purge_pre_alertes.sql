-- Nettoyage Niveau 2 (décision propriétaire 25/08) : l'ancien système de
-- pré-alertes (scorer SMC + ATR Straddle sur bougies clôturées, cycle 5 min)
-- est mort — worker supprimé (il alimentait Telegram en double des signaux
-- officiels), endpoint de lecture /api/pre_alertes supprimé (plus aucun
-- consommateur frontend). Les seules notifications sont les signaux v12
-- VALIDÉS émis par le writer. La table n'est plus écrite ni lue : purge.
-- La table `positions` (historique du monolithe) est conservée : données.
DROP TABLE IF EXISTS pre_alertes;
