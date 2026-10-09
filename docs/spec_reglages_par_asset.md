# Spécification — Réglages de stratégie PAR ASSET

**Objet** : chaque asset se comporte différemment (études : écart ~40× entre assets KDJ,
classements SMC par asset) — les réglages de sortie doivent pouvoir être différenciés par
`stratégie × asset`, avec moteurs, labo et LLM alignés sur la même source.
**Décisions propriétaire (09/10)** : ① granularité = stratégie × asset d'abord (×TF en
extension ultérieure) ; ② grilles d'événements par asset = **déjà en place** (constat
vérifié — voir § 2) ; ③ fractions par asset EXPLICITES à l'activation ; ④ séquencement en
3 phases avec checkpoint propriétaire à chaque phase.
**Statut** : spec validée 09/10 — **CHANTIER TERMINÉ, 3 phases validées par le propriétaire**.
Phase 1 (migration 0121, CRUD db + fusion, montages moteurs, empreinte hot-reload par
asset, API REST, bloc UI dans les 3 modales Paramètres). Phase 2 (essais tagués `assets`,
balayage fractions filtrable, advisory `?asset=` avec config actuelle fusionnée +
activation → SURCHARGE de l'asset, badge PAR ASSET + panels branchés + fix chips vérifié
sur capture owner). Phase 3 (`POST /api/analyses/{id}/ia/asset/{asset}` — analyste local,
cache du jour, prompt éditable `analyse_rapport_asset`, contexte 3 blocs, règle des 30
PAR ASSET, bouton 🤖 par ligne du classement des assets → modale ; chiffres clés
fabriqués par le moteur, chacun avec sa phrase d'explication — retour owner 09/10).
Rockets exclu (pas de surcharge). Déduplication au passage : fusion KDJ →
`params_kdj_fusionnes` (source unique). **Commit en attente de l'ordre du propriétaire.**
**Date** : 09/10/2026.

---

## 1. État des lieux vérifié (ce qui est / n'est pas déjà par asset)

| Mécanisme | Déjà par asset ? | Détail |
|---|---|---|
| Grille d'événements straddle | ✅ **OUI** | table `creneaux_evenements`, clé (asset, événement), armement + verdicts par case — les 81 cases = 9 assets × 9 événements |
| Calibration moteur SMC | ✅ partiellement | `AssetCalibration` par asset×TF (swing_length, sl_mode, seuils, clamps SL…) |
| Sizing / pips | ✅ | table `asset_params` |
| Périmètre straddle, assets KDJ armés | ✅ | configs `perimetre_straddle`, `kdj_assets_armes` (hot déjà) |
| **Réglages de sortie SMC** | ❌ global | clés config `smc_tp1_mult/tp2/tp3_*/trailing_r` + fractions `smc_frac_*` (prod actuelle : tp1 0,6 · tp2 2 · tp3 lointaine 3R · trailing 0,1R · fractions 1/0/0) |
| **Réglages numériques straddle** | ❌ global | `strategies_params` : sl_mult, trailing_r, placement_sec |
| **Paramètres KDJ** | ❌ global | `kdj_params` = UNE ligne (period 20, signal 7, amplitude 2, ratio_risk 2, adx_min −1) |

## 2. Principe : surcharge avec repli sur le défaut global

- Nouvelles tables **typées par stratégie** (une ligne optionnelle par asset,
  `NULL = repli sur le défaut global`) :
  - `smc_reglages_asset (asset PK, sl_max, trailing_r, tp1, tp2, tp3_mode, tp3_rfixe, frac_tp1, frac_tp2, frac_tp3)` ;
  - `straddle_reglages_asset (asset PK, sl_mult, trailing_r, placement_sec)` ;
  - `kdj_reglages_asset (asset PK, period, signal, amplitude, ratio_risk, adx_min)`.
- **Aucune ligne = comportement d'aujourd'hui exact** (non-régression par construction).
- Fonction de lecture unique par stratégie : `surcharge(asset) ⊕ défaut global` — consommée
  par les moteurs au montage, le labo, l'advisory, l'UI et le LLM (source unique de calcul).
- **Règle fractions (décision ③)** : à l'écriture d'une surcharge SMC touchant les
  fractions, les TROIS valeurs sont requises et doivent sommer à 1,00 (±0,001) — refus
  explicite sinon ; l'UI affiche la somme en direct.
- **Vécu immuable inchangé** : niveaux figés à l'émission, fractions figées à la
  clôture (le trade stocke déjà ses fractions — `fractions_json`) ; une surcharge
  n'affecte que les signaux futurs de l'asset.

## 3. Phase 1 — Socle : moteurs, hot-reload, UI

1. **Moteurs** : au montage d'un couple, le moteur reçoit les réglages FUSIONNÉS de son
   asset (SMC : `avec_tp1/tp2/tp3/trailing` par couple ; straddle : `avec_params` par
   couple ; KDJ : `avec_params` par couple — aujourd'hui globaux).
2. **Hot-reload par asset** (généralisation du mécanisme `2236035`) : l'empreinte de
   chaque couple inclut la surcharge de SON asset → changer le trailing de XAUUSD ne
   ré-arme QUE les couples XAUUSD. Garde straddle « fenêtre active » inchangée.
3. **UI Paramètres** : sélecteur « Tous les assets (défaut) / asset » sur chaque carte
   stratégie ; surcharge éditable, défaut global affiché en placeholder grisé ;
   « Réinitialiser sur défaut » ; straddle : les 3 champs numériques rejoignent la
   modale existante (la grille d'événements y est déjà par asset).
4. **API** : `GET/PUT/DELETE /api/strategies/{id}/reglages-asset/{asset}` (validation :
  fractions sommes, plages des paramètres, asset actif).

**Checkpoint 1 (propriétaire)** : surcharger un asset p.ex. trailing SMC XAUUSD 0,2 →
log « XAUUSD … réinscrit (paramètres modifiés — hot-reload) », aucun autre asset réarmé ;
trade ouvert sur un autre asset non impacté ; remise à défaut → retour au global.

## 4. Phase 2 — Labo et advisory par asset

1. **Simulation par asset** : la page 🧪 gagne le choix « asset » avec ses réglages
   virtuels propres (le re-jeu tourne déjà par couple — on lui passe les valeurs de
   l'asset) ; comparatif vécu/sim de l'asset seul.
2. **Balayage des fractions par asset** (le vécu connaît l'asset de chaque signal).
3. **Advisory 🎯 par asset** : `recommandation` renvoie le meilleur essai PAR asset,
   avec la garde d'honnêteté **règle des 30 par asset** : effectif affiché, badge
   « non jugeable » tant que n < 30 — les petits assets restent au défaut global tant
   que non prouvés (anti-overfitting, décision 24/09).

**Checkpoint 2** : simulation + advisory par asset vérifiés sur un asset fourni (p.ex.
XAUUSD M5) vs un asset maigre (badge non jugeable affiché honnêtement).

## 5. Phase 3 — LLM conseiller par asset

L'analyste IA reçoit par asset : classement du vécu (R/WR/effectif), réglage courant vs
défaut global, écart mesuré au balayage — et formule un conseil actionnable par asset
(« XAUUSD : trailing 0,2 suggéré, +1,2 R sur re-jeu, 45 clôturés — jugeable »).
L'IA propose, ne décide jamais (constitution du 24/08). Bouton par asset dans les
analyses ; mêmes deux voix (R distance / $).

**Checkpoint 3** : revue des conseils générés sur 2-3 assets par le propriétaire.

## 6. Tests et garde-fous

- Fallback : asset sans ligne = défaut (test d'égalité de construction moteur).
- Fusion : chaque champ surchargé pris isolément, les autres au défaut.
- Fractions : somme ≠ 1 → refus ; somme = 1 → écriture + relecture.
- Hot-reload : changement d'une surcharge ne réarme QUE les couples de l'asset (test
  `couples_a_retirer` étendu) ; garde fenêtre straddle inchangée.
- Non-régression : zéro ligne de surcharge ⇒ 39 suites workspace vertes à l'identique.
- Règle des 30 par asset affichée partout où un conseil chiffré par asset apparaît.

## 7. Hors périmètre (explicitement)

- Granularité ×TF des réglages (extension future si les mesures la justifient).
- Surcharge du capital/risque par asset (le vécu compose le capital global par
  stratégie — inchangé).
- Matrice d'événements : déjà par asset, rien à faire.
