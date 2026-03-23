# Intégration LLM → Rockets

> **Statut** : ✅ IMPLÉMENTÉ ET PUSHÉ (commit `6215dd3`, 23 mars 2026)  
> Mode 1 + Mode 2 : tous deux opérationnels en production.

**Objectif** : Faire du LLM un co-pilote actif de la stratégie Rockets pour améliorer le winrate, affiner les entrées et optimiser les réglages — sans bloquer le pipeline temps réel.

---

## Architecture retenue : 2 modes complémentaires

```
Mode 1 — Filtre pré-sauvegarde (temps réel)
  Worker scan → détecte breakout/prelancement
    → interroge LLM avec contexte ticker
    → LLM valide ou rejette
    → seulement si validé : sauvegarder() en DB

Mode 2 — Analyse stratégique périodique (asynchrone, hebdo)
  Job hebdomadaire → agrège métriques 30 derniers Rockets
    → envoie tout au LLM
    → LLM produit rapport de recommandations
    → stocké en DB, affiché dans RocketsAnalyseModal
```

---

## Mode 1 — Filtre LLM pré-sauvegarde ✅ TERMINÉ

### Fichiers créés / modifiés

| Fichier | Statut | Rôle |
|---|---|---|
| `backend/crates/db/migrations/0013_rockets_llm_filtre.sql` | ✅ Créé | 5 colonnes `llm_*` dans `rockets_signaux` |
| `backend/crates/db/src/rockets.rs` | ✅ Modifié | `NouveauRocket` étendu + `historique_ticker()` |
| `backend/crates/api/src/ollama/rockets_filtre.rs` | ✅ Créé | Prompt + parser + `filtrer_signal()` |
| `backend/crates/api/src/ollama/mod.rs` | ✅ Modifié | `pub mod rockets_filtre` ajouté |
| `backend/crates/api/src/rockets_scan.rs` | ✅ Modifié | Boucle auto-save remplacée avec filtre LLM |
| `backend/crates/api/src/rockets_handlers.rs` | ✅ Modifié | `NouveauRocket` manuel avec `llm_*=None` |
| `frontend/src/services/api.types.ts` | ✅ Modifié | `RocketSignalHistorique` + champs `llm_*` |
| `frontend/src/views/HistoryView.vue` | ✅ Modifié | Colonne "IA" badge conviction + tooltip raison |

### Comportement réel implémenté

Pour chaque candidat Rocket avant auto-save, le LLM reçoit :

**Données du signal détecté :**
- `ticker`, `phase` (breakout / prelancement)
- `score` (0–100)
- `prix_entree`, `stop_loss`, `atr14`, `atr_ratio`
- `ratio_volume`, `rsi`, `change1h`
- `support`, `target20`

**Historique DB du ticker (5 derniers signaux clôturés) :**
- verdict (tp1/tp2/tp3/sl/invalide)
- R réalisé (calculé depuis prix_entree / prix_verdict / stop_loss)
- phase du signal historique
- score du signal historique
- durée en heures entre entrée et clôture

**Métriques agrégées du ticker :**
- winrate sur les 20 derniers trades
- R moyen réalisé
- phase la plus rentable historiquement
- plage de score des trades gagnants

### 1.2 Nouveau prompt `PROMPT_FILTRE_ROCKET`

Le LLM répond en JSON strict :

```json
{
  "valide": true | false,
  "conviction": 0-100,
  "raison": "explication courte",
  "ajustements": {
    "sl_suggere": 1.23456,
    "tp1_suggere": 1.45678,
    "trailing_coef": 1.2
  }
}
```

- `valide: false` → le signal n'est PAS sauvegardé en DB
- `conviction < 60` → sauvegardé mais flaggé `qualite: "faible"`
- `ajustements` → permet d'affiner SL/TP avant insertion

### 1.3 Modifications backend ✅ RÉALISÉES

**`rockets_scan.rs` — `executer_scan()`** → boucle remplacée avec filtre LLM (voir comportement ci-dessus)

**`db/rockets.rs`** → `NouveauRocket` étendu, `historique_ticker()` ajoutée

Champs réellement implémentés dans `rockets_signaux` (migration `0013`) :
- `llm_valide INTEGER` (1=validé, null si pas filtré)
- `llm_conviction INTEGER` (0-100)
- `llm_raison TEXT`
- `llm_sl_suggere REAL`
- `llm_tp1_suggere REAL`

> Note : `llm_trailing_coef` non implémenté (non nécessaire en MVP)

**`ollama/rockets_filtre.rs`** (≠ nom initial `rockets.rs`) contient :
- `PROMPT_FILTRE_ROCKET`
- `fn formater_contexte()` (signal + historique ticker)
- `async fn filtrer_signal()` avec timeout 5s + fallback
- `struct SignalCandidat`, `struct FiltreReponse`, `struct AjustementsSl`

### 1.4 Gestion du cas Ollama indisponible ✅

Si Ollama ne répond pas dans 5s → **fallback : sauvegarder sans filtre** (`llm_*=NULL`). Le LLM est un bonus, pas un bloquant. Loggé en `warn`.

---

## Mode 2 — Analyse stratégique hebdomadaire ✅ TERMINÉ (commit `f133052` + fix `c960941`)

### Fichiers créés / modifiés

| Fichier | Statut | Rôle |
|---|---|---|
| `backend/crates/db/migrations/0012_rockets_analyses_llm.sql` | ✅ Créé | Table `rockets_analyses_llm` |
| `backend/crates/db/src/rockets.rs` | ✅ Modifié | `AnalyseLlm` + `sauvegarder_analyse()` + `derniere_analyse()` + `signaux_pour_analyse()` |
| `backend/crates/api/src/ollama/rockets_analyse.rs` | ✅ Créé (261L) | Prompt + agrégation par phase/tranche + `analyser_strategie()` |
| `backend/crates/api/src/rockets_analyse_handler.rs` | ✅ Créé (92L) | `lancer_analyse()` + `get_derniere_analyse()` + `demarrer_worker_analyse()` |
| `backend/crates/api/src/main.rs` | ✅ Modifié | `web::resource()` GET+POST sur `/api/rockets/analyse-llm` |
| `frontend/src/services/api.types.ts` | ✅ Modifié | `RocketRecommandation` + `RocketAnalyseLlm` |
| `frontend/src/services/api.service.ts` | ✅ Modifié | `lancerAnalyseLlmRockets()` + `getDerniereAnalyseLlmRockets()` |
| `frontend/src/components/common/RocketsAnalyseLlm.vue` | ✅ Créé (159L) | Sous-composant synthèse + recommandations |
| `frontend/src/components/RocketsAnalyseModal.vue` | ✅ Modifié | Onglet "🤖 Recommandations IA" |

### 2.1 Déclenchement ✅

- Worker séparé dans `main.rs`, interval = 7 jours
- Endpoint manuel `POST /api/rockets/analyse-llm` (bouton "Relancer" dans l'UI)

### 2.2 Données injectées ✅ RÉALISÉ

Agrégation via `signaux_pour_analyse()` dans `db/rockets.rs` — **toutes les tranches disponibles** (pas de limite à 30, configurable) :

```
Par phase :
  breakout    : N trades, winrate%, R moyen, score moyen des gagnants
  prelancement: N trades, winrate%, R moyen, score moyen des gagnants

Par tranche de score :
  <40, 40-60, 60-80, >80 : winrate% + R moyen par tranche

Par verdict :
  tp1/tp2/tp3 : N, % du total
  invalide/expire : N, % du total

Métriques globales :
  Ratio volume moyen, RSI moyen, ATR ratio moyen
  Durée moyenne entre entrée et clôture
```

### 2.3 Prompt `PROMPT_ANALYSE_ROCKETS` ✅ RÉALISÉ

Implémenté dans `ollama/rockets_analyse.rs`. Le LLM répond en JSON :

```json
{
  "synthese": "résumé de la performance globale",
  "recommandations": [
    {
      "type": "seuil_score | filtre_phase | trailing_stop | filtre_rsi | ...",
      "description": "explication détaillée",
      "impact_estime": "+X% winrate ou +Y R",
      "priorite": "haute | moyenne | faible"
    }
  ],
  "meilleur_setup": "description du setup le plus rentable",
  "pire_setup": "description du setup à éviter"
}
```

Paramètres Ollama : `temperature: 0.3`, `num_predict: 1024`, timeout **90s**.

### 2.4 Stockage et affichage ✅ RÉALISÉ

**DB** : table `rockets_analyses_llm` (migration `0012`)
- `id`, `cree_le`, `synthese TEXT`, `recommandations JSON`, `nb_trades_analyses`, `periode`

**Frontend** : `RocketsAnalyseLlm.vue` + onglet "🤖 Recommandations IA" dans `RocketsAnalyseModal.vue`
- Date de la dernière analyse
- Synthèse en haut (texte libre)
- Liste des recommandations triées par priorité (badge haute/moyenne/faible)
- Bouton "Relancer l'analyse" → `POST /api/rockets/analyse-llm`

---

## Statut d'implémentation global

| Étape | Description | Statut |
|---|---|---|
| 1 | Migration SQL colonnes `llm_*` dans `rockets_signaux` | ✅ `0013` |
| 2 | Migration SQL table `rockets_analyses_llm` | ✅ `0012` |
| 3 | CRUD `db/rockets.rs` (tout) | ✅ |
| 4 | `ollama/rockets_filtre.rs` (Mode 1) | ✅ |
| 5 | `ollama/rockets_analyse.rs` (Mode 2) | ✅ |
| 6 | `executer_scan()` avec filtre LLM | ✅ |
| 7 | Worker hebdo + endpoint analyse | ✅ |
| 8 | Frontend colonne IA (`HistoryView.vue`) | ✅ |
| 9 | Frontend onglet Recommandations IA | ✅ |

---

## Constantes techniques implémentées

| Paramètre | Valeur |
|---|---|
| Timeout filtre Mode 1 | 5s |
| Timeout analyse Mode 2 | 90s |
| Fallback si Ollama KO | Sauvegarde sans filtre (`llm_*=NULL`) |
| Fréquence worker analyse | 7 jours |
| Historique ticker | 10 derniers signaux clôturés |
| Trades minimum pour Mode 2 | Non bloquant (analyse quand même, module agrégeant ce qui existe) |

---

## Ce que le LLM ne peut PAS faire (limites à accepter)

- **Prédire l'avenir** : il raisonne sur des patterns historiques, pas sur le marché futur
- **Remplacer le backtest** : les recommandations sont des hypothèses à valider sur données réelles
- **Auto-modifier le code** : ses recommandations sont textuelles, l'humain décide d'appliquer ou non

---

## Métriques de succès

Après 30 jours d'utilisation du Mode 1 :
- Winrate visé : +10 points par rapport au baseline sans filtre
- R moyen visé : +0.2R
- Volume signaux : peut baisser (filtrage), c'est normal et souhaitable

Après 3 cycles d'analyse (Mode 2) :
- Au moins 1 recommandation appliquée (ex: seuil score ajusté)
- Évolution mesurable des métriques dans `RocketsAnalyseModal`

---

## Notes opérationnelles

- Les badges IA n'apparaissent que sur les **signaux créés après le déploiement** (23 mars 2026)
- Les anciens signaux affichent `—` dans la colonne IA (champs `NULL` en DB)
- Si Ollama est éteint au moment du scan → fallback automatique, aucune perte de signal
- Le badge IA est coloré : vert ≥70, orange 50-69, rouge <50
