# AMÉLIORATIONS SMC V12 — Audit ICT et roadmap d'implémentation

> État au 28 août 2026. Source : audit complet des 27 indicateurs du Pine v12
> contre les standards SMC/ICT, croisé avec les propositions externes
> (`docs/reference/ameliorations_proposees_v12.md`).
>
> **Constitution** : le Pine est l'étalon. Toute amélioration est portée d'abord
> dans le Pine, puis clonée en Rust, puis affichée. Chaque changement de
> stratégie est immédiatement répercuté dans les prompts IA.

---

## Contexte

L'audit du 28/08 a comparé les 27 indicateurs du v12 contre les standards
SMC/ICT (LuxAlgo, InnerCircleTrader, FibAlgo, DailyPriceAction, FluxCharts).
Résultat : 22 parfaitement conformes, 4 améliorables, 1 correction visuelle
(adelivrée — Premium/Discount horizontal, CE des gaps, Breaker avec sweep).

Les 10 propositions externes ont été analysées : **5 validées, 3 invalidées,
2 avec réserves**.

---

## Décisions de validation

| ID | Module | Verdict | Justification |
|---|---|---|---|
| **A** | BPR (Balanced Price Range) | ✅ Retenu | Vrai concept ICT — chevauchement FVG bull/bear = zone d'équilibre institutionnelle. Nouveau module de scoring. |
| **B** | Scoring des Gaps NDOG/NWOG | ❌ Rejeté | **Déjà testé et retiré le 19/07/2026** (campagne FINAL, jamais positif en BT). La constitution exige 30 trades pour recalibrer. |
| **C** | Dead Zone (NY Lunch) | ⚠️ Malus, pas blocage | Concept légitime (NY Lunch = volatilité morte). Mais le blocage dur (`return`) est agressif — **préférer un malus de scoring (-3 pts)**, validé par replay. |
| **D** | Filtre de Régime (Range/Trend) | ✅ Retenu (sous conditions) | Le MODULE 12b existe dans le Pine mais est désactivé. **Comprendre pourquoi avant de réactiver.** |
| **E** | Filtre RSI | ❌ Rejeté | **RSI n'est pas un concept SMC/ICT.** Contredit la philosophie du moteur (« clone fidèle Pine SMC pur »). |
| **F** | Scoring Sessions H/L | ✅ Retenu | Cohérent avec la liquidité ICT — les H/L de session sont des cibles BSL/SSL. Données déjà présentes (MODULE 14). |
| **G** | DoL comme TP3 dynamique | ✅ Appliqué 28/08 — **DoL plafonné à 3R** | Replay 24 mois : DoL pur **coûtait 67R** (liquidité souvent > 3R, inatteignable avant expire). DoL≤3R = +61.5R vs production précédente. Pine + Rust + prompt synchronisés. |
| **H** | Mega-Orders (volume 3×) | ⚠️ Retenu à calibrer | Concept valable mais 3× SMA20 trop strict. **Calibrer à 2× et tester en replay.** |
| **I** | OFI (Order Flow Imbalance) | ❌ Rejeté | Approximation grossière (`buyVol = volume × (close-low)/(high-low)` suppose distribution uniforme — faux). Complexité maximale pour un signal douteux. Pas ICT authentique. |
| **J** | Alerte composite BPR | ✅ Retenu (faible) | Dépend du module A. Confort uniquement. |

---

## Corrections déjà livrées (28/08 — commit `e9141b2`)

- [x] Premium/Discount horizontal (3 lignes : limite Premium, EQ, limite Discount)
- [x] CE (Consequent Encroachment) des gaps NDOG/NWOG — ligne à 50% du gap
- [x] Breaker avec sweep préalable obligatoire (≤ 10 bars)

---

## Phase 1 — Validation préalable ✅ TERMINÉE (28/08)

### Module D (filtre régime) → ❌ REJETÉ
Le Pine documente (ligne 2429) : « jamais testé en BT, décidé campagne FINAL 2026-07-19 ». Le code a été entièrement retiré (pas juste désactivé). L'OTE-only voisin a été supprimé dans la même campagne pour dégradation prouvée du PF. Ne pas réactiver sans preuve.

### Module C (dead zone NY Lunch) → ❌ REJETÉ comme règle globale
Replay 24 mois (6 assets × M1/M5/M15) : le NY Lunch n'est mortel que sur BTC M1 (WR 27% vs 51%). Sur XAU M15 et NAS M15, le lunch est neutre (50% WR) voire meilleur. Une règle globale pénaliserait des trades gagnants. Échantillon lunch BTC M1 = 26 trades (sous le seuil de 30). Supprimé de la roadmap.

### Module H (mega-orders) → ⚠️ DONNÉES INSUFFISANTES
Le volume n'est pas jointif avec le journal des trades (timestamps non alignés entre `runtime_emissions` et `bougies`). À réévaluer en Phase 5 avec un replay dédié volume-corrigé. Reporter.

### Modules restants après Phase 1
- **G (DoL TP3)** — priorité maximale
- **A (BPR)** — phase 3
- **F (sessions H/L)** — phase 4
- H — en attente (Phase 5)

## Phase 2 — Module G : DoL comme TP3 ✅ TERMINÉE (28/08) — DoL≤3R APPLIQUÉ en production

### Constat initial : le module était DÉJÀ en production
Le TP3-DoL existe déjà dans le Pine étalon (`_bsDolTarget` ligne 3294 + TP3 v11
lignes 3556-3574) ET dans le moteur Rust (`nearest_liq`, commit « décisions
trading »). La proposition externe partait d'une version antérieure du Pine.

**Correctif de parité livré au passage** : le TP3 des trades BSZones incluait à
tort l'Asian High/Low côté Rust — le Pine ne l'inclut que pour les trades v11
(`_tAHH3`), pas dans `_bsDolTarget`. Aligné sur l'étalon (+ test dédié).

### Étude replay — 3 modes × 6 assets × M1/M5/M15 (≈ 14-24 mois, BE = Supprimé)
Binaire : `comparatif_tp3` · Sortie : `data/comparatif_tp3.txt`

| Mode | R total | R moyen | TP3 touchés | TP2+BE | max DD |
|---|---|---|---|---|---|
| **DoL** (production actuelle) | +704.6R | +0.254 | 95 | 316 | 12.0R |
| **Fixe3R** (contre-factuel) | **+772.0R** | **+0.278** | 186 | 226 | 12.0R |
| **DoL≤3R** (plafonné) | +766.1R | +0.276 | **195** | 217 | 12.0R |

### Lecture des chiffres — le DoL pur COÛTE ~67R
Paradoxe apparent : le DoL (cible plus « intelligente ») produit MOINS de TP3
que le 3R fixe (95 vs 186). Explication : la liquidité la plus proche (EQH/PDH/
PWH) est généralement AU-DELÀ de 3R (surtout M1/M5 où le R clampé ATR est
petit) → le prix n'atteint jamais la cible avant l'expire → le trade retombe
en TP2+BE au lieu de TP3. Les 91 bascules TP2+BE→TP3 (≈ +1R chacune) font
presque tout le delta.

Le plafonnement `DoL≤3R` (liquidité si elle est PLUS PROCHE que 3R, sinon 3R)
récupère +61.5R : il garde la logique ICT (cibler la liquidité atteignable)
sans courir après des cibles inatteignables. Fixe3R pur vs DoL≤3R :
5.9R d'écart sur 2 776 trades = bruit statistique.

### Décision VALIDÉE et APPLIQUÉE (28/08) — DoL≤3R
1. ✅ **Pine étalon** : TP3 = `min(DoL, entry±3R)` dans les 4 fonctions de création
   (v11 BUY/SELL `_tp3 := _m3` plafonné, BS BUY/SELL `_bsDolTarget` plafonné ;
   le garde-fou `si TP3 casse la monotonie → 3R` est conservé)
2. ✅ **Rust** : défaut production `ModeTp3::DolCappe3R` dans `SignalGenerator::new()`
   + test dédié (`build_levels_tp3_dol_plafonne_3r_par_defaut`)
3. ✅ **Prompt IA** : « TP3 = liquidité la plus proche plafonnée à 3R »
4. Alternative Fixe3R pur (+5.9R de plus) : statistiquement indistinguable,
   mais abandonne toute logique de liquidité — non retenue.

Échantillon de validation : 2 776 trades (règle des 30 largement respectée).
Attendu en production : +61.5R vs l'ancien DoL pur sur fenêtre équivalente.

### Constat annexe — SP500 muet
SP500 = 0 signal SMC sur toutes les fenêtres replay (18 mois). Les 5 autres
assets produisent 69-371 trades par fenêtre. À investiguer en Phase 5
(calibration ? zones jamais qualifiées ?).

## Phase 3 — Module A : BPR (2 sessions)

- [ ] **Pine étalon** :
  - Détection : chevauchement FVG bull × FVG bear (intersection géométrique)
  - Lifecycle : frais → partiel → profond → supprimé (comme OB/FVG)
  - Scoring : bonus +4 si OB chevauche un BPR frais, +3 si partiel, +1 si profond
  - Affichage : box dorée avec label « BPR »
  - Anti-doublon + FIFO 10 max + âge max 15 bars
- [ ] **Moteur Rust** : nouveau détecteur `bpr.rs` dans `smc/src/v12/`
- [ ] **Frontend** : rendu des boxes BPR dans l'overlay
- [ ] **Replay comparatif** : impact WR avec/sans BPR sur 24 mois

## Phase 4 — Module F : Sessions H/L (1 session)

- [ ] **Module F** : scoring sessions H/L — bonus +2 si prix proche du H/L de session asiatique ou londonne (données MODULE 14 déjà présentes)
- [ ] Replay comparatif

## Phase 5 — Validation finale (1 session)

- [ ] **Replay comparatif intégral** : avant/après TOUTES améliorations sur 24 mois × 6 assets × 3 TF
- [ ] **Module H** (mega-orders) : si des données volume-corrigées sont disponibles, calibrer et tester
- [ ] Ajuster les pondérations si nécessaire (règle : 30 trades minimum)
- [ ] **Synchroniser les prompts IA** (`smc_definition`) — constitution : stratégie changée = prompt changé
- [ ] Documenter les paramètres recommandés par asset
- [ ] Mettre à jour la ROADMAP.md principale

---

## Estimation totale : 4-5 sessions

| Phase | Modules | Durée | Statut |
|---|---|---|---|
| 1 | Audit préalable (C, D, H) | 1 session | ✅ Terminée |
| 2 | G (DoL TP3) | 1 session | ✅ Terminée — DoL≤3R en production |
| 3 | A (BPR) | 2 sessions | Prochaine |
| 4 | F (sessions H/L) | 1 session | |
| 5 | Validation globale + H + SP500 muet | 1 session | |

---

## Règles transverses (rappel constitution)

- **Pine = étalon** : toute amélioration est portée d'abord dans le Pine, puis Rust
- **Chirurgie uniquement** : additive, sans régression, tests + build avant commit
- **30 trades minimum** avant tout recalibrage
- **Stratégie changée = prompt changé** : les prompts IA sont synchronisés à chaque étape
- **L'IA n'exécute jamais** : les modules bonifient le scoring, jamais de trade autonome
