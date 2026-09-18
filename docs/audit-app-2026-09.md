# Audit complet de l'app — Septembre 2026

> Règle absolue : une étape vérifiée avant de démarrer la suivante.
> État : étape 1 (inventaire front + croisement) — TERMINÉE.

## Références de départ (étape 0 — FAITE)

- SMC : 427 clôtures · −10,02 R · 896,50 $
- Straddle : 54 · +23,50 R · 1 257,50 $
- Rockets : 2 · +1,25 R · 1 012,54 $
- KDJ : 0 · 0 R · 1 000 $
- 495 signaux · 17 546 282 bougies · DB 2,15 Go
- Tests : 61 api + 35 db + 23 front verts

---

## Étape 1 — Inventaire front + croisement code (FAITE)

### 1.1 Pages inventoriées et validées par le propriétaire

| # | Page | Route | Statut | Remarques |
|---|---|---|---|---|
| 1 | Dashboard | / | ✅ validée | — |
| 2 | SMC | /smc | ✅ validée | Boutons d'action à vérifier (pas sur la page) |
| 3 | SMC Scanner | /smc/scanner | ✅ validée | — |
| 4 | SMC Définition | /smc/definition | ✅ validée | — |
| 5 | SMC Analyse | /smc/analyse | ✅ validée | Refonte demandée : conseils actionnables |
| 6 | Straddle | /straddle | ✅ validée | Armement auto demandé (paper trading) |
| 7 | Straddle Définition | /straddle/definition | ✅ validée | — |
| 8 | Straddle Analyse | /straddle/analyse | ✅ validée | Même refonte que SMC |
| 9 | Rockets | /rockets | ✅ validée | — |
| 10-12 | Rockets Def/Scanner/Analyse | — | ✅ validées | — |
| 13-15 | KDJ Page/Def/Scanner | — | ✅ validées | Bouton scanner à supprimer de la page |
| 16 | Graphiques | /smc/graphiques | ✅ validée | — |
| 17 | Heatmap | /heatmap | ⚠️ orpheline | Aucun lien n'y mène — décision à prendre |
| 18 | Simulation | /simulation | ✅ validée | — |
| 19 | Analyses | /analyses | ✅ validée | Ergonomie + fusion avec analyses dédiées |
| 20 | IA | /ia | ✅ validée | Évaluer utilité ML/LLM |
| 21 | Presse | /presse | ✅ validée | 4 corrections demandées |
| 22 | Données | /donnees | ✅ validée | — |

### Décisions propriétaire gravées pendant l'inventaire

1. **Toutes pages Analyse** : doivent fournir des conseils actionnables (choix assets/TF/événements, réglages) — pas juste des chiffres
2. **Source unique de calcul** : tout vient du moteur backend, le front affiche sans recalculer
3. **Straddle paper trading** : armement automatique de TOUS les assets (supprimer plafond 8 pendant la phase)
4. **KDJ** : armer le moteur dès que possible
5. **Boutons** : pas de duplication entre pages stratégie et cartes dashboard

### 1.2 Croisement endpoints front ↔ backend

**Résultat global** : 124 routes backend, 103 appelées par le front, 0 bug 404.

#### Endpoints backend jamais appelés par le front (21)

**Code mort confirmé — à supprimer (P2)** :
- GET /api/strategies/{id}/performance
- GET /api/smc/rejeu
- GET /api/straddle/rejeu
- GET/POST /api/rockets/unlocks + DELETE /api/rockets/unlocks/{symbole}/{date}
- GET /api/sentiment/composite
- GET /api/news/traduire
- POST /api/ia/signal/straddle

**API outillée EA — à GARDER** :
- GET /api/mt5/symboles
- POST /api/mt5/kline
- POST /api/mt5/heartbeat
- POST /api/mt5/historique
- GET /api/mt5/historique/etat

**API outillée runtime — à évaluer** :
- GET /api/runtime/concordance
- POST/GET /api/runtime/replay (×3 variantes)
- GET /api/runtime/emissions

**Divers** :
- PUT /api/worker/config — anomalie : commentaire front sans implémentation
- GET /api/creneaux-volatilite — redondant avec volatility/patterns-jour

#### Paramètres suspects

| Endpoint | Problème | Priorité |
|---|---|---|
| smc/v12/analyse?limit=50000 | Pas de plafond haut backend | P1 |
| signaux?limit=N | Pas de plafond serveur | P2 |
| armer-file sans timeout | Timeout axios 15 s trop court | P1 |

### 1.3 Catch silencieux (~50+ occurrences)

**Les 5 plus dangereuses** :

1. `SimulationSmcPanel.vue` `appliquer()` — écriture réglages réels échoue en silence après confirmation
2. `PositionsARisqueTable.vue` — clôture de position non exécutée
3. `DashboardStrategiesBlocs.vue` `basculerTelegram()` — icône ment sur l'état
4. `JournalBordModal.vue` — notes ajoutées/supprimées sans effet
5. `StraddleAgendaPanel.vue` (×4) — armer/ignorer/calculer/seuils tous silencieux

**Pattern correct existant** : `alerteStore.afficherErreur` utilisé par ~35 catch — à généraliser.

**Catégories** :
- Cat 1 (vide) : 4 occurrences (bénines — nettoyage chart)
- Cat 2 (commentaire) : ~25 occurrences (impact réel — actions métier)
- Cat 3 (remise à zéro) : ~15 occurrences (données vides sans distinction erreur/vide)
- Cat 4 (retour silencieux) : ~10 occurrences (null/false sans message)
- Cat 5 (attrape sans rien faire) : 4 occurrences

### 1.4 Recalculs front violant la source unique

| Fichier | Ce qui est recalculé | Correctif |
|---|---|---|
| SignauxTableau.vue:168 | risque = abs(entry - SL) | Servir `risque` depuis backend |
| HistoryTable.vue:138 | Idem | Idem |
| useEnCoursStrategies.ts:56 | Idem | Idem |

**Bon pattern de référence** : `useLatentsSignaux` — backend fournit `r_fixe + coef_prix`, le front compose avec le prix live WS.

---

## Plan de vérification — Étape 1

1. Ouvrir chaque page et confirmer chargement sans erreur console (F12)
2. Tester un bouton d'action par catégorie (armement, sauvegarde, clôture) et vérifier retour visuel
3. Couper le backend → vérifier que les pages affichent une erreur visible au lieu de rester vides

---

## Bugs et améliorations catalogués

### P0 — Bugs cassants (à corriger en premier)

| # | Description | Fichier |
|---|---|---|
| P0-1 | Presse : clic sur article traduit repasse en version originale | PresseView.vue |
| P0-2 | Simulation : appliquer réglages peut échouer en silence après confirmation | SimulationSmcPanel.vue |

### P1 — Corrections importantes

| # | Description |
|---|---|
| P1-1 | Généraliser `alerteStore.afficherErreur` sur les ~50 catch silencieux |
| P1-2 | Plafonner `smc/v12/analyse` côté backend (limite haute) |
| P1-3 | Timeout dédié pour `armer-file` (actuellement 15 s défaut) |
| P1-4 | KDJ : supprimer bouton « Scanner tendance » de la page |
| P1-5 | Heatmap : décider garder (ajouter lien) ou supprimer |
| P1-6 | Servir `risque` depuis backend (fin des recalculs front) |

### P2 — Dette technique

| # | Description |
|---|---|
| P2-1 | Supprimer 8 endpoints morts confirmés |
| P2-2 | Supprimer code mort front (boutons SMO, commentaires worker/config) |
| P2-3 | Plafonner `signaux?limit` côté serveur |

### P3 — Améliorations

| # | Description |
|---|---|
| P3-1 | Refonte pages Analyse : conseils actionnables (verdict 5 s → classements → détail) |
| P3-2 | Fusion Rapport d'activité + pages Analyse dédiées |
| P3-3 | Straddle : armement automatique pendant paper trading |
| P3-4 | KDJ : armer moteur |
| P3-5 | Presse : tri par score, filtrer faibles, lien source dynamique |
| P3-6 | IA : évaluer utilité ML/LLM |
