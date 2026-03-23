# Intégration LLM → Rockets

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

## Mode 1 — Filtre LLM pré-sauvegarde

### 1.1 Données injectées dans le prompt

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

### 1.3 Modifications backend

**`rockets_scan.rs` — `executer_scan()`**

Dans la boucle `for r in resultats.iter().filter(...)` :
1. Requêter DB : historique des 5 derniers Rockets clôturés sur `r.ticker`
2. Appeler `ollama::filtrer_rocket(signal, historique)` (nouveau module)
3. Si `reponse.valide == false` → `continue` (pas de sauvegarde)
4. Si `reponse.valide == true` → utiliser `reponse.ajustements.sl_suggere` etc. pour le `NouveauRocket`
5. Stocker `reponse.conviction` et `reponse.raison` dans la table `rockets_signaux` (nouveaux champs)

**`db/rockets.rs`**

Nouveaux champs dans `rockets_signaux` :
- `llm_valide INTEGER` (0/1, null si pas encore interrogé)
- `llm_conviction INTEGER` (0-100)
- `llm_raison TEXT`
- `llm_sl_suggere REAL`
- `llm_tp1_suggere REAL`
- `llm_trailing_coef REAL`

Migration SQL à créer dans `db/migrations/`.

**`ollama/rockets.rs`** — nouveau fichier

Contient :
- `PROMPT_FILTRE_ROCKET` (prompt système)
- `fn formater_contexte_rocket(signal, historique) -> String`
- `async fn filtrer_rocket(signal, historique) -> Result<FiltreReponse>`
- `struct FiltreReponse { valide, conviction, raison, ajustements }`

### 1.4 Gestion du cas Ollama indisponible

Si Ollama ne répond pas dans 5s → **fallback : sauvegarder sans filtre** (comportement actuel). Le LLM est un bonus, pas un bloquant. Logger le timeout en `warn`.

---

## Mode 2 — Analyse stratégique hebdomadaire

### 2.1 Déclenchement

- Worker séparé dans `main.rs`, interval = 7 jours
- OU endpoint manuel `POST /api/rockets/analyse-llm` déclenché depuis l'UI

### 2.2 Données injectées

Agrégation des **30 derniers Rockets clôturés** :

```
Par phase :
  breakout   : N trades, winrate%, R moyen, score moyen des gagnants
  prelancement: N trades, winrate%, R moyen, score moyen des gagnants

Par verdict :
  tp1_atteint: N, % du total, durée moyenne
  tp2_atteint: N, % du total, durée moyenne
  tp3_atteint: N, % du total, durée moyenne
  sl_touche  : N, % du total, durée moyenne avant SL
  invalide   : N, % du total

Corrélations observées :
  Meilleur winrate quand ratio_volume > X
  Meilleur R moyen quand rsi entre Y et Z
  Phases les plus profitables
  Coefficient ATR actuel (SL=1×ATR, TP1=1×ATR, TP2=2×ATR, TP3=20×ATR)
```

### 2.3 Prompt `PROMPT_ANALYSE_ROCKET`

Le LLM reçoit les métriques et répond en JSON :

```json
{
  "synthese": "résumé en 3 phrases de la performance",
  "recommandations": [
    {
      "type": "seuil_score",
      "description": "Passer le score minimum de 40 à 65 pour les breakouts",
      "impact_estime": "+12% winrate",
      "priorite": "haute"
    },
    {
      "type": "filtre_phase",
      "description": "Désactiver la phase prelancement (winrate 28% < seuil 40%)",
      "impact_estime": "réduction volume, +8% winrate global",
      "priorite": "moyenne"
    },
    {
      "type": "trailing_stop",
      "description": "Coefficient ATR stop trop serré : suggère 1.5×ATR au lieu de 1×ATR",
      "impact_estime": "+0.3R moyen",
      "priorite": "haute"
    },
    {
      "type": "filtre_rsi",
      "description": "RSI>85 sur breakout corrèle avec 73% de SL touché, filtrer",
      "impact_estime": "+15% winrate sur breakouts",
      "priorite": "haute"
    }
  ],
  "meilleur_setup": "breakout + ratio_volume>2.5 + RSI 60-75 + score>70",
  "pire_setup": "prelancement + rsi>80 + score<50"
}
```

### 2.4 Stockage et affichage

**DB** : nouvelle table `rockets_analyses_llm`
- `id`, `cree_le`, `synthese TEXT`, `recommandations JSON`, `nb_trades_analyses`

**Frontend** : nouveau bloc dans `RocketsAnalyseModal.vue`
- Onglet "Recommandations IA" (à côté des stats existantes)
- Date de la dernière analyse
- Synthèse en haut (texte libre)
- Liste des recommandations triées par priorité (badge haute/moyenne/faible)
- Bouton "Relancer l'analyse"

---

## Ordre d'implémentation recommandé

### Étape 1 — Infrastructure DB
1. Migration SQL : ajout colonnes `llm_*` dans `rockets_signaux`
2. Création table `rockets_analyses_llm`
3. Fonctions CRUD dans `db/rockets.rs`

### Étape 2 — Module ollama/rockets.rs
1. `PROMPT_FILTRE_ROCKET` + parser JSON réponse
2. `formater_contexte_rocket()` (signal + historique ticker)
3. `filtrer_rocket()` avec timeout 5s + fallback
4. `PROMPT_ANALYSE_ROCKET` + parser JSON
5. `analyser_strategie_rockets()` (agrégation + appel LLM)

### Étape 3 — Filtre temps réel (Mode 1)
1. Modifier `executer_scan()` : appel LLM + logique valide/invalide
2. Récupérer historique ticker depuis DB avant chaque save
3. Utiliser `llm_sl_suggere` / `llm_tp1_suggere` si présents

### Étape 4 — Analyse périodique (Mode 2)
1. Worker hebdomadaire dans `main.rs`
2. Endpoint `POST /api/rockets/analyse-llm`
3. Agrégation des métriques dans `db/rockets.rs`

### Étape 5 — Frontend
1. Affichage `llm_conviction` + `llm_raison` dans `HistoryView.vue` (colonne ou tooltip)
2. Onglet "Recommandations IA" dans `RocketsAnalyseModal.vue`
3. Bouton "Relancer analyse" → appel `POST /api/rockets/analyse-llm`

---

## Contraintes techniques à respecter

| Contrainte | Détail |
|---|---|
| Timeout LLM | 5s en mode filtre, 60s en mode analyse |
| Fallback | Si Ollama KO → sauvegarder sans filtre (pas de blocage) |
| Fréquence filtre | 1 appel LLM par signal détecté, max ~10 par scan |
| Fréquence analyse | 1 fois par semaine (ou manuelle) |
| Taille fichiers | Respecter <300 lignes : `ollama/rockets.rs` + `ollama/rockets_analyse.rs` séparés |
| DAG | `rockets_scan.rs` peut appeler `ollama::` (Layer 3) |
| Zero panic | Tous les appels LLM wrappés en `Result`, erreurs loguées |
| JSON parsing | Utiliser `serde_json`, rejeter silencieusement si JSON invalide + fallback |

---

## Ce que le LLM ne peut PAS faire (limites à accepter)

- **Prédire l'avenir** : il raisonne sur des patterns historiques, pas sur le marché futur
- **Remplacer le backtest** : les recommandations sont des hypothèses à valider sur données réelles
- **Auto-modifier le code** : ses recommandations sont textuelles, l'humain décide d'appliquer ou non
- **Fonctionner sans historique** : avec moins de 10 Rockets clôturés, l'analyse n'est pas fiable → désactiver Mode 2 sous ce seuil

---

## Métriques de succès

Après 30 jours d'utilisation du Mode 1 :
- Winrate visé : +10 points par rapport au baseline sans filtre
- R moyen visé : +0.2R
- Volume signaux : peut baisser (filtrage), c'est normal et souhaitable

Après 3 cycles d'analyse (Mode 2) :
- Au moins 1 recommandation appliquée (ex: seuil score ajusté)
- Évolution mesurable des métriques dans `RocketsAnalyseModal`
