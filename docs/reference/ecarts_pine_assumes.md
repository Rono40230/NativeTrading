# Écarts du Pine étalon assumés par décision propriétaire

Le Pine `smc_indicateur_v12.pine` (md5 `9f231ccc0889df9f2fa1849af74beb23`) reste
l'étalon : toute différence Rust/Pine non listée ici est un bug. Ce document
centralise les seules déviations votées par le propriétaire.

## 14/09 — Classe alt-crypto partage le profil BTC (Phase 4.1)

**Règle Pine d'origine** (`_assetReconnu`, ligne 49 + ligne 2540) : seuls
XAU/XAG/NAS/BTC/DAX/SPX sont reconnus ; tout autre actif reçoit un scoring
nul (`sc = 0`) → force plafonnée ~2, aucune OB affichable.

**Décision propriétaire (chantier B du 14/09)** : les 10 alt-cryptos suivies
en 7.F — ETH, SOL, ADA, AVAX, BNB, DOGE, DOT, LINK, LTC, XRP — partagent le
**profil BTC intégral** : `asset_reconnu = true`, seuils de force 8/99/99/15,
pondérations (3,1,2,2,2), `seuil_ib` 2.0, `atr_score` 3.5, `SlMode::Atr2x`,
SL min/max ×ATR et durées de trade de la famille crypto.
Implémentation : `smc/src/v12/calibration.rs` (const `ALT_CRYPTOS`).

**Justification mesurée** (outil : `etude_calibration_smc --cal BTC`,
rejeu 24 mois M15 par actif, amorce MTF complète) : les distributions des
scores maximaux par OB sont superposables à celle de BTC —
p50 3 · p90 6-7 · p95 7-8 · p99 9-10 · max 15-19, taux de zones force ≥ 5
de 7,3 à 10,0 % (BTC réel : 8,2 % ; gamme des 6 actifs calibrés : 3-14 %).
La rareté des zones fortes — un choix de design du Pine — est préservée.

**Restent NON reconnus** (scoring nul, fidèle au Pine) : les forex Axi
(EURUSD, GBPUSD, USDJPY, …) et les métaux XPTUSD/XPDUSD — aucun historique
suffisant pour calibrer par la mesure ; règle « mesure avant décision ».
Le calibrage de ces classes passera par ce même protocole quand l'EA aura
poussé l'historique : rejeu → distribution → vote du profil.
