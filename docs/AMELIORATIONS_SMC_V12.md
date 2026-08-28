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
| **G** | DoL comme TP3 dynamique | ✅ Retenu — **priorité maximale** | ICT enseigne que les cibles sont les pools de liquidité (EQH/EQL, PDH/PDL), pas des multiples R fixes. **Le seul module qui change les verdicts TP3.** |
| **H** | Mega-Orders (volume 3×) | ⚠️ Retenu à calibrer | Concept valable mais 3× SMA20 trop strict. **Calibrer à 2× et tester en replay.** |
| **I** | OFI (Order Flow Imbalance) | ❌ Rejeté | Approximation grossière (`buyVol = volume × (close-low)/(high-low)` suppose distribution uniforme — faux). Complexité maximale pour un signal douteux. Pas ICT authentique. |
| **J** | Alerte composite BPR | ✅ Retenu (faible) | Dépend du module A. Confort uniquement. |

---

## Corrections déjà livrées (28/08 — commit `e9141b2`)

- [x] Premium/Discount horizontal (3 lignes : limite Premium, EQ, limite Discount)
- [x] CE (Consequent Encroachment) des gaps NDOG/NWOG — ligne à 50% du gap
- [x] Breaker avec sweep préalable obligatoire (≤ 10 bars)

---

## Phase 1 — Validation préalable (1 session)

- [ ] **Analyser pourquoi le filtre de régime (MODULE 12b) est désactivé** dans le Pine. Si la raison est obsolète → réactiver avec ADX/ATR ratio. Si la raison est valable → documenter et classer le module D comme rejeté.
- [ ] **Replay comparatif dead zone** : mesurer le WR des signaux générés pendant NY Lunch (16h-18h UTC) sur 24 mois. Si WR < 30% → valider le malus de -3 pts (Module C).
- [ ] **Calibrer le seuil Mega-Orders** : comparer le WR des signaux avec volume > 2× vs 3× SMA20 sur 24 mois. Choisir le seuil optimal (Module H).

## Phase 2 — Module G : DoL comme TP3 (1 session)

**Le plus gros gain attendu** — change les verdicts, pas seulement le scoring.

- [ ] **Pine étalon** : TP3 = DoL (prochain EQH/EQL/PDH/PDL dans la direction du trade) si DoL > TP2, sinon 3R classique
  - Trade LONG → TP3 = prochain EQH ou PDH (buy-side liquidity)
  - Trade SHORT → TP3 = prochain EQL ou PDL (sell-side liquidity)
  - Garde-fou : si DoL < TP2 → repli sur 3R classique
- [ ] **Moteur Rust** : modifier le calcul TP3 dans le lifecycle (`trade.rs` / `lifecycle.rs`)
- [ ] **Frontend** : aucun changement (le TP3 vient du moteur)
- [ ] **Replay comparatif** : taux de TP3 avant/après sur 24 mois × 6 assets

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

## Phase 4 — Modules F, C, D (1 session)

- [ ] **Module F** : scoring sessions H/L — bonus +2 si prix proche du H/L de session asiatique ou londonne (données MODULE 14 déjà présentes)
- [ ] **Module C** : malus -3 pts en dead zone NY Lunch (si validé par replay Phase 1)
- [ ] **Module D** : réactivation filtre régime ADX/ATR (si Phase 1 conclut que la désactivation est obsolète)
- [ ] **Module H** : scoring volume > seuil calibré (si Phase 1 valide le seuil)
- [ ] Replay comparatif global Phase 4

## Phase 5 — Validation finale (1 session)

- [ ] **Replay comparatif intégral** : avant/après TOUTES améliorations sur 24 mois × 6 assets × 3 TF
- [ ] Ajuster les pondérations si nécessaire (règle : 30 trades minimum)
- [ ] **Synchroniser les prompts IA** (`smc_definition`) — constitution : stratégie changée = prompt changé
- [ ] Documenter les paramètres recommandés par asset
- [ ] Mettre à jour la ROADMAP.md principale

---

## Estimation totale : 5-6 sessions

| Phase | Modules | Durée | Prérequis |
|---|---|---|---|
| 1 | Audit préalable (C, D, H) | 1 session | Aucun |
| 2 | G (DoL TP3) | 1 session | Aucun |
| 3 | A (BPR) | 2 sessions | Aucun |
| 4 | F, C, D, H | 1 session | Résultats Phase 1 |
| 5 | Validation globale | 1 session | Phases 2-4 terminées |

---

## Règles transverses (rappel constitution)

- **Pine = étalon** : toute amélioration est portée d'abord dans le Pine, puis Rust
- **Chirurgie uniquement** : additive, sans régression, tests + build avant commit
- **30 trades minimum** avant tout recalibrage
- **Stratégie changée = prompt changé** : les prompts IA sont synchronisés à chaque étape
- **L'IA n'exécute jamais** : les modules bonifient le scoring, jamais de trade autonome
