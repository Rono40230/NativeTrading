# ÉTAPE 4 — Calcul des trades : SL, TP, BE automatique

> Feuille de route propriétaire — étape 4, réalisée le 29/08/2026.
> Méthode : cartographie du calcul actuel → variantes derrière drapeaux
> (`ModesEtude` : `sl_mult`, `tp1_mult`, `tp2_mult`, `be_auto`) → grid replay
> 6 assets × M1/M5/M15 (~3 100 clôtures par branche, ticks simulés) →
> retournement du gradient recherché avant décision.

---

## 1. État d'avant (référence)

- **SL** : bord opposé de l'OB + offset ATR selon `_autoSlMode` (XAU/XAG/NAS/
  SPX/DAX : 1×ATR ; BTC : 2× ; XAG historique 1.5×), **clamps**
  `[_slMin, _slMax]` = 0.5-0.8×ATR min / 1.5-2.5×ATR max par asset
  (les « maximums à ne pas dépasser » — inchangés dans toute l'étude).
- **TP** : TP1 = 1R · TP2 = 2R · TP3 = DoL plafonné 3R (Phase 2).
- **BE** : uniquement à TP1 touché (SL→entry) ; BE forcé supprimé (26/08).

## 2. Première passe (7 branches, `data/etape4_comparatif.txt`)

| Variante | Delta vs production | Verdict |
|---|---|---|
| TP1 = 0.8R | +284R* | candidat → affinage |
| SL × 0.75 | +75.5R | candidat → affinage |
| SL × 1.25 | −5.7R | rejeté |
| TP2 = 2.5R | −82.3R | rejeté (TP2 = 2R confirmé) |
| BE auto 50% / 70% | −180.5R / −82.1R | **rejeté** (voir §4) |

\* avant correction comptable (voir §3).

## 3. Le piège comptable détourné

`realized_r()` codait `Tp1 => 1.0` et `Tp2 => 2.0` **en dur** — écrit quand
les niveaux étaient fixes. Avec TP1 paramétrable, chaque verdict TP1
comptait +1.0R alors que le trade n'avait capturé que tp1_mult×R : la
première grille gonflait artificiellement les variantes (≈ +321R d'inflation
sur la branche 0.7R). **Correctif** : `Tp1/Tp2 => dist(tp)/risk0` —
comportement strictement identique en production (1R/2R), comparaison A/B
honnête ensuite. (Leçon durable ajoutée au journal : une constante de
comptabilité écrite pour des niveaux fixes devient un bug silencieux
dès qu'on paramètre ces niveaux.)

## 4. Verdict BE automatique (objectif 3) : ❌ REJETÉ

Mécanisme testé : BE armé quand la MFE atteint 50%/70% du chemin de TP1 sans
le toucher, SL→entry au retour à l'entrée. Résultats : −180.5R (50%) et
−82.1R (70%). Le mécanisme **convertit les courses gagnantes en BE** :
verdicts TP3 211→138/178, TP1+BE 632→354/492, et 1 008/486 clôtures « BE »
apparues. Plus le seuil est profond, moins il dégrade — mais même à 70%
l'effet reste négatif : sur nos marchés, un retour à l'entrée après une
avancée partielle est statistiquement une **poursuite**, pas un échec.

## 5. Grid final honnête et retournement (`data/etape4_retenue.txt`)

| Branche | R total | R moyen | max DD |
|---|---|---|---|
| production (TP1=1R, SL×1.0) | +791.1R | +0.254 | 12.0R |
| TP1=0.6R | +998.6R | +0.316 | 8.8R |
| TP1=0.5R+SL0.75 | +984.0R | +0.309 | 6.0R |
| TP1=0.4R+SL0.75 | +921.2R | +0.288 | 4.4R |
| **TP1=0.6R + SL×0.75** | **+1029.9R** | **+0.324** | **7.8R** |
| TP1=0.6R+SL0.65 | +1026.3R | +0.321 | 6.2R |

**Le gradient s'inverse sous 0.6R** (0.5R : 984R ; 0.4R : 921R) — l'optimum
est un plateau : 0.6R+SL0.75 (+238.8R, +30%) avec voisins immédiats à
−31/−46R (pas de crête knife-edge). Robustesse : 16/18 cellules positives
(seuls BTC M1 −3.2R et XAG M1 +0.8R dérogent), run reproduit à l'identique
deux fois (déterministe). Mécanisme dominant : TP1 plus proche convertit
les near-miss SL en TP1+BE (SL 927→593, TP1+BE 632→1 261).

## 6. Décision APPLIQUÉE (29/08) — TP1 = 0.6R, offset SL × 0.75

1. ✅ **Pine étalon** : TP1 = `entry ± 0.6×_r` (4 sites v11+BS) ; offsets SL
   `× 0.75` (4 sites, 3 modes). Clamps `[_slMin,_slMax]` et garde 2×slMax
   inchangés (maximums respectés). TP2 = 2R et TP3 = DoL≤3R inchangés.
2. ✅ **Rust** : défauts `SignalGenerator` (sl_mult 0.75, tp1_mult 0.6) +
   `ModesEtude::default()` — production alignée. Drapeaux conservés pour
   ré-étude.
3. ✅ **Prompt IA** (`smc_definition`) : « TP1 = 0.6R (décision étape 4 du
   29/08 : replay +239R), SL au-delà de la zone (offset ATR réduit 25 %) ».
4. ✅ Tests unitaires mis à jour aux nouveaux défauts (r/TP1/plafonds DoL).
5. Attendu en production : **+239R** sur fenêtre équivalente, R moyen
   +27 %, max DD 12→7.8R.

### Réponses aux trois objectifs

1. **SL au mieux avec maximums** : offset × 0.75 (clamps inchangés) —
   resserrer de 25 % vaut +75R seul et +31R en combinaison ; plus serré
   (×0.65) n'apporte rien de plus ; plus large (×1.25) dégrade.
2. **TP au mieux** : TP1 = 0.6R (le premier objectif sert à sécuriser et
   armer le BE, pas à viser 1R) ; TP2 = 2R confirmé ; TP3 = DoL≤3R confirmé.
3. **BE automatique à X % de TP1** : rejeté aux deux seuils testés —
   le retour à l'entrée après avancée partielle est une poursuite
   statistique, pas un signal de sortie.
