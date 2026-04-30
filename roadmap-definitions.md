# Roadmap — Corrections pages "Définitions & Prompt IA"

## Objectifs globaux

1. **Exactitude** : chaque valeur affichée reflète le code réel (barèmes, seuils, TP/SL)
2. **Dynamisme** : toutes les métriques/paramètres se mettent à jour automatiquement si la config change en DB
3. **Design** : présentation claire, responsive, cohérente entre les trois stratégies

---

## BLOC A — Corrections de fond (exactitude)

### A1 — Straddle (`VolatiliteDefinitionView.vue`)

#### A1.1 — Seuil conviction LLM dynamique
- **Problème** : le barème "≥ 75 → Entre ✅" est hardcodé dans le template
- **Réalité** : le seuil vient de `charger_seuils(asset, categorie)` en DB, évolue avec les feedbacks. En warm start (<5 feedbacks) = 5.5/10, en mode calibré = valeur apprise
- **Correction** : afficher "Seuil calibré (dynamique par asset)" + lire la valeur via un appel API `/straddle/monitoring` ou endpoint dédié au lieu d'une constante

#### A1.2 — TP/SL réels vs paramètres UI
- **Problème** : les paramCards affichent TP1/TP2/TP3/SL de la config UI mais ce ne sont pas ces valeurs qui sont utilisées dans le calcul réel
- **Réalité code** (`straddle_signal_ollama.rs`) :
  - SL Long = `prix − 0.5 × ATR`
  - TP1 = `prix + 2.0 × ATR`
  - TP2 = `prix + 3.5 × ATR`
  - TP3 = `prix + 5.0 × ATR`
- **Correction** : remplacer les paramCards TP/SL par les formules réelles (affichage statique des formules), ou aligner le code pour utiliser les params configurés

#### A1.3 — Mécanismes absents de la description
Les éléments suivants existent dans le code mais ne sont pas documentés :
- Anti-doublon 30 min filtré par stratégie (aucun nouveau signal si un Straddle est déjà actif sur cet asset dans les 30 dernières minutes)
- Catégorisation des pics : `Calendrier`, `SessionOuverture`, `AtrPur`, `Whipsaw` — chaque catégorie a ses propres seuils calibrés
- Corrélation soft : si un asset corrélé est déjà en position → score règles −10 pts + seuil LLM +0.7 (pas de blocage dur)
- Warm start : pour un asset avec <5 feedbacks, seuils abaissés (score_règles=50, ratio_atr=1.3) pour permettre l'apprentissage

#### A1.4 — Description ML incorrecte
- **Problème** : "IA indécise" non défini dans la vue
- **Réalité** : pipeline LSTM+XGBoost hybride. Si confiance ML > seuil configurable (défaut 0.75) = directionnel → skip. Si indécis → Straddle éligible. Si ML non disponible → score règles prend le relais.

---

### A2 — SMC (`SmcDefinitionView.vue`)

#### A2.1 — Barèmes incorrects (critique)

| Confluence | Affiché dans la vue | Code réel (`smc/src/lib.rs`) |
|---|---|---|
| Tendance | +20 pts | **+25 pts** |
| Order Block | +25 pts | +25 pts ✅ |
| Imbalance/FVG | +20 pts | **+15 pts** |
| IFVG | +20 pts | +20 pts ✅ |
| Fibonacci | +15 pts | +15 pts ✅ |

- **Correction** : lire les constantes `SCORE_MAX_*` depuis l'API (exposer un endpoint `/smc/baremes` ou les inclure dans les paramètres) et ne plus hardcoder ces valeurs dans le template

#### A2.2 — Kill Zone et Sweep absents
- **Réalité** : `kill_zone_active` et `sweep_detecte` sont des **prérequis ICT** calculés dans le scorer et transmis au LLM comme contexte. Ils conditionnent la conviction finale.
- **Correction** : les ajouter comme 6e et 7e éléments (prérequis, non scorés en points mais bloquants si absents selon catégorie)

#### A2.3 — Seuil conviction LLM dynamique
- Même problème que Straddle A1.1 : le seuil vient de `charger_seuils_smc(asset, tf, categorie)`, pas d'une constante

#### A2.4 — TP pyramidal non documenté
- Code réel :
  - TP2 = `prix + ATR14 × 2.5`
  - TP3 = `prix + ATR14 × 4.0`
  Ces valeurs ne correspondent pas aux `atr_tp2` / `atr_tp3` affichés dans les paramCards si ceux-ci sont différents

#### A2.5 — "POST /ia/signal" incorrect
- La carte "Génération signal JSON" indique `POST /ia/signal` — ce endpoint n'existe pas. Le signal est généré par la boucle automatique, pas par un appel REST externe.
- **Correction** : remplacer par "Boucle auto 15 min" ou supprimer la mention du endpoint

#### A2.6 — Graduations IFVG et Imbalance absentes
- IFVG est gradué (0 IFVG aligné = 0 pts, 1 = 10 pts, 2+ = 20 pts), non mentionné
- Imbalance est gradué (0 zone = 0 pts, 1 = 8 pts, 2+ = 15 pts), non mentionné

---

### A3 — Rockets (`RocketsDefinitionView.vue`)

#### A3.1 — Seuil conviction non fixe
- Le seuil affiché (65) vient de `charger_seuils(pool, phase, session)` — calibré par phase et session de marché. Peut être différent selon que c'est `prelancement`, `breakout` ou `momentum`.
- **Correction** : afficher "Seuil calibré par phase/session" + valeur en temps réel si disponible

#### A3.2 — Trailing coeff non documenté
- Le LLM peut ajuster le `trailing_coeff` (clampé entre `trailing_coeff_min` et `trailing_coeff_max` de la config). C'est un paramètre actif visible en logs, absent de la définition.

#### A3.3 — Sélection feedbacks par similarité 3D non mentionnée
- Le contexte few-shot du LLM est construit en sélectionnant les feedbacks les plus similaires selon 3 dimensions : ratio_volume, atr_ratio, score. C'est une feature avancée qui améliore la pertinence du LLM, à mentionner dans "Rôle de l'IA".

---

## BLOC B — Dynamisme automatique des valeurs

### B1 — Principe
Toute valeur numérique affichée dans une page de définition doit provenir d'un appel API au démarrage du composant, jamais hardcodée dans le template ou le `<script setup>`.

### B2 — Checklist par stratégie

#### Straddle
- [x] `atr_seuil`, `atr_periode`, `horizon_bougies` → déjà chargés via `strategyParamsStore` ✅
- [ ] `score_seuil` LLM → à charger via un endpoint (ex: `/straddle/seuils-effectifs/{asset}`)
- [ ] Formules SL/TP → à aligner avec les params ou afficher dynamiquement la formule avec les multiplicateurs réels du code
- [ ] Nombre de feedbacks actifs (warm start actif ?) → optionnel, informatif

#### SMC
- [x] `score_min`, `atr_periode`, `atr_tp1/2/3`, `atr_sl`, `horizon_bougies` → déjà chargés via `strategyParamsStore` ✅
- [ ] Barèmes individuels (SCORE_MAX_TENDANCE, etc.) → à exposer via un endpoint `/smc/baremes` et charger au montage
- [ ] Seuil conviction LLM effectif → à charger

#### Rockets
- [x] `score_min`, `rsi_min/max`, `ratio_volume_min`, `vol_marche_min`, `phases_actives` → déjà chargés ✅
- [ ] Seuil conviction calibré par phase → à charger via `/rockets/seuils/{phase}`
- [ ] `trailing_coeff_min/max` → à afficher dans les paramCards

### B3 — Pattern recommandé
```
onMounted(async () => {
  await store.charger()           // params config
  seuilsEffectifs.value = await api.getSeuilsEffectifs(asset) // seuils calibrés
})
```
Toute modification de config dans le panel de paramètres doit émettre un événement qui rafraîchit les pages de définition ouvertes (store réactif Pinia).

---

## BLOC C — Refonte visuelle

### C1 — Problèmes actuels
- Layout `grid-cols-3` fixe → pas responsive sur écrans < 1400px
- Certaines colonnes débordent ou sont inutilement scrollables
- Pas de hiérarchie visuelle claire entre "ce qui est constant" et "ce qui est calibré/dynamique"
- Les valeurs hardcodées ne se distinguent pas visuellement des valeurs dynamiques
- Absence de tooltip ou légende pour les termes techniques (IFVG, BOS, Kill Zone…)

### C2 — Objectifs de la refonte

#### Layout responsive
- **Desktop (>1400px)** : 3 colonnes actuelles, conservées
- **Laptop (1024–1400px)** : 2 colonnes (Concept+Params | Détails) + colonne IA en dessous plein-largeur
- **Compact (<1024px)** : 1 colonne, accordéons pliables par section

#### Hiérarchie visuelle
- Badge "dynamique" (vert) sur toute valeur chargée depuis l'API → indique qu'elle se met à jour
- Badge "formule" (bleu) sur les valeurs calculées (TP/SL par ATR)
- Section "Mécanique" visuellement distincte de "Rôle de l'IA"

#### Indicateurs de santé
- Petit indicateur en temps réel : nombre de signaux actifs, dernier signal il y a X min, warm start actif ou non

#### Tooltips
- Chaque terme technique (Order Block, IFVG, Kill Zone, Straddle, ATR, etc.) doit avoir un tooltip au survol avec une définition en 1 phrase

### C3 — Cohérence inter-stratégies
Les trois pages doivent partager :
- Le même composant de paramCards (`<ParamCard label value badge />`)
- Le même composant de scoring row (`<ScoringRow label detail max_pts />`)
- Le même composant de règle LLM (`<LlmRegle icon label couleur />`)
- Une charte couleur commune : vert pour validé, rouge pour rejeté, jaune pour warning, bleu pour info

---

## Ordre d'exécution recommandé

| # | Tâche | Priorité | Effort |
|---|---|---|---|
| 1 | Corriger barèmes SMC dans le template (A2.1) | 🔴 Urgent | 30 min |
| 2 | Corriger seuil conviction Straddle (A1.1) | 🔴 Urgent | 30 min |
| 3 | Supprimer "POST /ia/signal" SMC (A2.5) | 🔴 Urgent | 5 min |
| 4 | Ajouter Kill Zone + Sweep dans SMC (A2.2) | 🟡 Important | 1h |
| 5 | Corriger TP/SL Straddle (A1.2) | 🟡 Important | 1h |
| 6 | Ajouter mécanismes Straddle manquants (A1.3) | 🟡 Important | 1h |
| 7 | Exposer barèmes SMC via API + chargement dynamique (B2 SMC) | 🟡 Important | 2h |
| 8 | Charger seuils calibrés dynamiquement pour les 3 stratégies (B2) | 🟡 Important | 3h |
| 9 | Créer composants partagés ParamCard, ScoringRow, LlmRegle (C3) | 🟢 Amélioration | 3h |
| 10 | Refonte layout responsive (C2) | 🟢 Amélioration | 4h |
| 11 | Ajout tooltips termes techniques (C2) | 🟢 Amélioration | 2h |
| 12 | Indicateurs de santé temps réel (C2) | 🟢 Amélioration | 2h |
