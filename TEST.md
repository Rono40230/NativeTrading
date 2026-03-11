# 🧪 Plan de tests — Native Trading AI

Tests complets couvrant le backend Rust, le frontend Tauri/Vue 3, les modèles IA Ollama,
et les flux de données temps réel.

---

## Prérequis globaux

```bash
# 1. Démarrer l'application complète
./scripts/run.sh

# 2. Vérifier que le backend répond
curl http://localhost:8080/health
# Attendu : { "status": "ok" }

# 3. Vérifier qu'Ollama est actif
curl http://localhost:11434/api/tags
# Attendu : liste contenant qwen2.5:14b, qwen2.5:3b, llava
```

---

## T01 — Persistance des modèles ML ⭐ CRITIQUE

**Pourquoi** : Sans persistance, le modèle est perdu à chaque redémarrage. Changement structurel.

### Cas 1 — Entraînement et sauvegarde
```bash
# Lancer l'entraînement
curl -X POST "http://localhost:8080/api/ml/train?asset=BTC&timeframe=M15&limit=1000"

# Vérifier les fichiers générés
ls -lh data/modele_rf.json data/modele_lstm.json
```
**Attendu** : Les deux fichiers sont créés avec une taille > 0.

### Cas 2 — Rechargement après redémarrage
```bash
# Redémarrer le backend (Ctrl+C sur run.sh puis relance)
# Sans relancer l'entraînement :
curl http://localhost:8080/api/ml/status
```
**Attendu** : `{ "modele_pret": true, "lstm_pret": true }` — les modèles sont rechargés depuis le disque.

### Cas 3 — Démarrage sans modèle (premier lancement)
```bash
# Supprimer les modèles
rm -f data/modele_rf.json data/modele_lstm.json
# Redémarrer le backend et vérifier le statut
curl http://localhost:8080/api/ml/status
```
**Attendu** : `{ "modele_pret": false }` — le backend ne crashe pas, log approprié.

---

## T02 — Risk Management : limites de sécurité ⭐ CRITIQUE

**Pourquoi** : Vérifications financières non-négociables — 4 règles à valider.

### Cas 1 — Limite exposition par actif (25% max)
```bash
# Tenter un signal dont la taille dépasse 25% du capital configuré
curl -X GET "http://localhost:8080/api/smc/analyse?asset=BTC&timeframe=M15&limit=200"
```
**Attendu** : Le score SMC est calculé, et si un signal est émis, la taille de position
ne dépasse jamais 25% du capital dans les logs backend.

### Cas 2 — Limite max drawdown (20% → arrêt)
```bash
# Vérifier dans les logs backend qu'une alerte drawdown est levée si > 20%
# (simulable en modifiant temporairement le capital dans les données de test)
```
**Attendu** : Log `WARN risk: drawdown 20% atteint — arrêt trading`.

### Cas 3 — Risk/Reward minimum
Dans la vue **SMC Analyzer** → lancer une analyse → vérifier que le ratio R/R affiché
est calculé correctement : `(TP - Entrée) / (Entrée - SL)`.

**Attendu** : Valeur numérique cohérente, jamais `NaN` ni `Infinity`.

---

## T03 — WebSocket temps réel ⭐ CRITIQUE

**Pourquoi** : Remplacement du polling 5s par WebSocket natif Binance.

### Cas 1 — Connexion et réception de ticks
```bash
# Nécessite wscat
npm install -g wscat
wscat -c "ws://localhost:8080/api/stream?asset=BTC&timeframe=M1"
```
**Attendu** : Messages JSON reçus à chaque bougie fermée :
```json
{
  "type": "candle",
  "asset": "BTC",
  "timeframe": "M1",
  "data": {
    "timestamp": "2026-03-11T10:00:00Z",
    "open": 85000.0,
    "high": 85200.0,
    "low": 84900.0,
    "close": 85100.0,
    "volume": 123.45
  }
}
```

### Cas 2 — Mise à jour live sur la vue Charts
- Aller dans **Charts** → sélectionner `BTC / M1`
- Observer le graphique pendant 2-3 minutes

**Attendu** : La dernière bougie se met à jour en temps réel (hauteur variable) sans rechargement.
Le prix affiché en haut change de valeur.

### Cas 3 — Changement d'asset pendant le stream
- Dans **Charts**, passer de `BTC` à `ETH`

**Attendu** : L'ancien stream se déconnecte, un nouveau s'ouvre sur ETH.
Aucune erreur dans la console DevTools Tauri.

### Cas 4 — Reconnexion après coupure backend
- Couper le backend (`Ctrl+C`)
- Observer la vue Charts
- Relancer le backend

**Attendu** : Un message d'erreur s'affiche (`⚠`), et la reconnexion se fait à la prochaine
action utilisateur (changement asset/timeframe ou clic Actualiser).

---

## T04 — Backtest multi-stratégies

**Pourquoi** : Vérifier que les deux stratégies produisent des résultats distincts et cohérents.

### Cas 1 — Stratégie Straddle
```bash
curl -s -X POST http://localhost:8080/api/backtest \
  -H "Content-Type: application/json" \
  -d '{"asset":"BTC","timeframe":"M15","capital":10000,"limit":500}' | python3 -m json.tool
```
**Attendu** :
- `total_trades` > 0
- `win_rate` entre 0 et 100
- `capital_final` différent de `capital_initial`
- `max_drawdown_pct` < 50 (cohérence)

### Cas 2 — Résultats cohérents (ROI / Sharpe)
```bash
# Backtester avec plus de bougies
curl -s -X POST http://localhost:8080/api/backtest \
  -H "Content-Type: application/json" \
  -d '{"asset":"ETH","timeframe":"M5","capital":5000,"limit":1000}'
```
**Attendu** : `sharpe_ratio` et `profit_factor` non nuls, pas de `NaN` dans la réponse JSON.

### Cas 3 — Test depuis la vue PnL
- Aller dans **P&L** → cliquer **Lancer le backtest**
- Vérifier que les métriques s'affichent dans les cartes de résultat
- Vérifier que le graphique d'équité (courbe capital) est tracé

---

## T05 — Démarrage robustesse

**Pourquoi** : Les `expect()` critiques ont été supprimés — vérifier les cas limites.

### Cas 1 — Démarrage nominal
```bash
cd backend && cargo run --release 2>&1 | head -20
```
**Attendu** : `🌐 Server running on http://0.0.0.0:8080` sans panic ni erreur.

### Cas 2 — Base de données absente (création auto)
```bash
# Supprimer la base et relancer
rm -f data/trading.db
./scripts/run.sh
curl http://localhost:8080/health
```
**Attendu** : Le backend crée automatiquement la base via les migrations SQLx.
Aucun crash, `{ "status": "ok" }` retourné.

### Cas 3 — Ollama absent au démarrage de Tauri
- Couper Ollama manuellement : `pkill ollama`
- Relancer l'app Tauri

**Attendu** : L'app démarre sans bloquer, Ollama est relancé automatiquement
(délai ~1.5s), statut IA passe à `disponible: true` après quelques secondes.

---

## T06 — IA Analyse SMC narrative (`qwen2.5:14b`)

**Pourquoi** : Vérification du handler existant — non régressé après refactoring.

### Via l'API
```bash
curl -s -X POST http://localhost:8080/api/ia/analyse \
  -H "Content-Type: application/json" \
  -d '{
    "asset": "BTC",
    "timeframe": "M15",
    "direction": "LONG",
    "score_smc": 78.5,
    "prix_entree": 85000,
    "stop_loss": 84500,
    "take_profit": 86200,
    "tendance": 20,
    "order_block": 18,
    "imbalance": 15,
    "ifvg": 12,
    "fibonacci": 13.5,
    "confiance_ml": 0.72
  }'
```
**Attendu** :
- `modele` = `"qwen2.5:14b"`
- `analyse` = texte de 5-8 phrases en français
- Temps de réponse : 15-60s (modèle lourd)

### Via la vue SMC Analyzer
- Aller dans **SMC Analyzer** → sélectionner BTC/M15
- Cliquer **Analyser avec IA**

**Attendu** : Texte d'analyse affiché dans le panel dédié, badge modèle `qwen2.5:14b`.

---

## T07 — IA Coach conversationnel (`qwen2.5:3b`) ⭐ NOUVEAU

**Pourquoi** : Nouveau routage vers `qwen2.5:3b` — réponses plus rapides pour la conversation.

### Via l'API
```bash
curl -s -X POST http://localhost:8080/api/ia/chat \
  -H "Content-Type: application/json" \
  -d '{
    "messages": [
      {"role": "user", "contenu": "Explique les Order Blocks en trading SMC"}
    ]
  }'
```
**Attendu** :
- `modele` = `"qwen2.5:3b"` (vérification du routage)
- `reponse` = explication en français
- Temps de réponse : 3-10s (nettement plus rapide que 14b)

### Cas — Historique de conversation
```bash
curl -s -X POST http://localhost:8080/api/ia/chat \
  -H "Content-Type: application/json" \
  -d '{
    "messages": [
      {"role": "user",      "contenu": "Qu'est-ce qu'une imbalance ?"},
      {"role": "assistant", "contenu": "Une imbalance est un gap entre deux bougies..."},
      {"role": "user",      "contenu": "Et comment l'utiliser pour entrer en position ?"}
    ]
  }'
```
**Attendu** : Réponse contextuelle tenant compte de la conversation précédente.

### Validation limite historique
```bash
# Construire 41 messages (dépasse la limite de 40)
# Le plus simple : tester dans la vue SMC Coach en envoyant 21+ échanges
```
**Attendu** : Erreur propre `400 Bad Request` avec message `"Historique trop long (max 40 messages)"`.

### Via la vue SMC Coach
- Aller dans **SMC Coach**
- Envoyer : *"Comment identifier un Order Block valide ?"*
- Comparer la vitesse de réponse avec une analyse SMC (T06) — doit être significativement plus rapide

**Attendu** : Réponse en < 10s, badge modèle `qwen2.5:3b` visible.

---

## T08 — Analyse visuelle de graphique (`llava`) ⭐ NOUVEAU

**Pourquoi** : Nouveau endpoint `/api/ia/chart` — analyse d'image via modèle vision.

### Via l'API (test avec image de test)
```bash
# Encoder une image PNG en base64
base64 -w 0 /chemin/vers/chart_test.png > /tmp/img_b64.txt

curl -s -X POST http://localhost:8080/api/ia/chart \
  -H "Content-Type: application/json" \
  -d "{
    \"asset\": \"BTC\",
    \"timeframe\": \"M15\",
    \"image_base64\": \"$(cat /tmp/img_b64.txt)\"
  }"
```
**Attendu** :
- `modele` = `"llava"`
- `analyse` = texte mentionnant tendance + supports/résistances + recommandation
- Temps : 30-90s (1ère inférence plus lente — chargement modèle)

### Via la vue Charts (test principal)
1. Aller dans **Charts** → sélectionner `BTC / M15`
2. Attendre le chargement complet du graphique
3. Cliquer le bouton **🔍 Analyser (IA)** (violet, toolbar en haut à droite)
4. Observer l'état du bouton → passe à `🔍 Analyse...`
5. Attendre l'analyse (30-90s selon charge GPU)

**Attendu** :
- Panel violet apparaît sous les statistiques
- Texte d'analyse en français avec : tendance principale, niveaux clés, recommandation LONG/SHORT/NEUTRE
- Badge `llava` affiché
- Bouton `✕` ferme le panel

### Cas — Asset différent
- Changer pour `ETH / H1` → cliquer **🔍 Analyser (IA)**

**Attendu** : L'analyse mentionne `ETH` dans le contexte (le timeframe H1 est transmis).

### Cas — Erreur si Ollama absent
- Couper Ollama : `pkill ollama`
- Cliquer **🔍 Analyser (IA)**

**Attendu** : Message d'erreur dans le panel violet : `Échec analyse: Ollama injoignable`.
Le bouton revient à l'état normal (non bloqué).

---

## T09 — Statut IA (endpoint de monitoring)

```bash
curl -s http://localhost:8080/api/ia/status | python3 -m json.tool
```
**Attendu** :
```json
{
  "ollama_disponible": true,
  "modele": "qwen2.5:14b",
  "url": "http://localhost:11434/api/chat"
}
```
- `ollama_disponible` = `true` si Ollama répond
- Cohérence avec le badge dans le Dashboard (indicateur IA en ligne)

---

## T10 — Smoke test flux complet (régression)

**Pourquoi** : Vérifier qu'aucun endpoint existant n'a régressé.

```bash
# Santé
curl -s http://localhost:8080/health

# Bougies BTC
curl -s "http://localhost:8080/api/candles?asset=BTC&timeframe=M15&limit=5" | python3 -m json.tool

# Analyse SMC
curl -s "http://localhost:8080/api/smc/analyse?asset=BTC&timeframe=M15&limit=200" | python3 -m json.tool

# Prédiction ML
curl -s "http://localhost:8080/api/ml/predict?asset=BTC&timeframe=M15" | python3 -m json.tool

# Signaux
curl -s "http://localhost:8080/api/signaux?limit=5" | python3 -m json.tool

# Export CSV (vérifier que le header Content-Type est text/csv)
curl -sI "http://localhost:8080/api/signaux/export?limit=10"
```
**Attendu** : Tous les endpoints répondent avec HTTP 200, JSON valide (ou CSV pour l'export).

---

## T11 — Interface Tauri (vérifications visuelles)

### Dashboard
- [ ] Métriques affichées : Win Rate, ROI, Total Trades, Max Drawdown
- [ ] Indicateur Ollama (vert si en ligne, rouge si hors ligne)
- [ ] Signaux récents listés avec badge direction (LONG/SHORT)
- [ ] Aucune erreur `[object Object]` dans les alertes

### Vue Charts
- [ ] Graphique chandelier charge avec les 200 dernières bougies
- [ ] Sélecteurs asset (BTC/ETH) et timeframe (M1→W1) fonctionnels
- [ ] Dernier prix + variation % affichés en haut
- [ ] Stats (Bougies, Volume moy, Plus haut, Plus bas) cohérentes
- [ ] Bouton **🔍 Analyser (IA)** présent et cliquable
- [ ] Panel violet d'analyse se ferme avec ✕

### Vue SMC Analyzer
- [ ] Score de confluence affiché (0-100)
- [ ] Détail des 5 composantes (Tendance, OB, Imbalance, IFVG, Fibonacci)
- [ ] Niveaux TP1/TP2/TP3 affichés si signal ≥ 70/100
- [ ] Dropdown asset/timeframe avec texte lisible (fond sombre, texte noir)

### Vue SMC Coach
- [ ] Zone de saisie message fonctionnelle
- [ ] Historique de conversation affiché avec rôles user/assistant distincts
- [ ] Badge modèle `qwen2.5:3b` visible
- [ ] Bouton d'envoi désactivé pendant la génération

### Vue P&L
- [ ] Backtest se lance et retourne des métriques
- [ ] Dropdown timeframe avec texte lisible
- [ ] Métriques : ROI, Win Rate, Sharpe Ratio, Max Drawdown, Profit Factor

### Vue Historique
- [ ] Tableau des trades passés affiché
- [ ] Filtres par asset et timeframe fonctionnels
- [ ] Export CSV téléchargeable
- [ ] Dropdown avec texte lisible

### Vue Paramètres
- [ ] Paramètres sauvegardés persistent après fermeture/réouverture de l'app
- [ ] Asset et timeframe par défaut bien appliqués à l'ouverture

### Vue Heatmap
- [ ] Données affichées sans erreur

---

## T12 — Performance (seuils critiques)

| Métrique | Seuil requis | Commande de mesure |
|---|---|---|
| Réponse `/health` | < 50 ms | `time curl http://localhost:8080/health` |
| Chargement 200 bougies | < 2 s | `time curl "...api/candles?limit=200"` |
| Analyse SMC (calculs) | < 3 s | `time curl "...api/smc/analyse"` |
| Prédiction ML | < 10 s | `time curl "...api/ml/predict"` |
| Backtest 500 bougies | < 10 s | `time curl -X POST ...api/backtest` |
| Chat IA Coach (3b) | < 15 s | `time curl -X POST ...api/ia/chat` |
| Analyse narrative (14b) | < 60 s | `time curl -X POST ...api/ia/analyse` |
| Analyse visuelle (llava) | < 120 s | `time curl -X POST ...api/ia/chart` |

```bash
# Exemple mesure prédiction ML
time curl -s "http://localhost:8080/api/ml/predict?asset=BTC&timeframe=M15" > /dev/null

# Exemple mesure chat IA Coach
time curl -s -X POST http://localhost:8080/api/ia/chat \
  -H "Content-Type: application/json" \
  -d '{"messages":[{"role":"user","contenu":"Définir une imbalance"}]}' > /dev/null
```

---

## Récapitulatif

| # | Test | Priorité | Statut |
|---|---|---|---|
| T01 | Persistance modèles ML | 🔴 Critique | À tester |
| T02 | Risk Management (limites) | 🔴 Critique | À tester |
| T03 | WebSocket temps réel | 🔴 Critique | À tester |
| T04 | Backtest multi-stratégies | 🟠 Important | À tester |
| T05 | Démarrage robustesse | 🟠 Important | À tester |
| T06 | IA Analyse SMC (qwen2.5:14b) | 🟠 Important | À tester |
| T07 | IA Coach conversationnel (qwen2.5:3b) | 🟠 Important | **NOUVEAU** |
| T08 | Analyse visuelle Charts (llava) | 🟠 Important | **NOUVEAU** |
| T09 | Statut IA monitoring | 🟡 Standard | À tester |
| T10 | Smoke test flux complet | 🟡 Standard | Régression |
| T11 | Interface Tauri visuels | 🟡 Standard | Régression |
| T12 | Seuils de performance | 🟢 Optionnel | Perf |


---

## T01 — Persistance des modèles ML ⭐ CRITIQUE

**Pourquoi** : Changement structurel — sans persistance, le modèle est perdu à chaque redémarrage.

### Prérequis
```bash
# Démarrer le backend
cd backend && cargo run --release
```

### Étapes
1. S'assurer que `data/modele_rf.json` et `data/modele_lstm.json` n'existent pas encore
2. Lancer un entraînement :
   ```bash
   curl -X POST http://localhost:8080/api/ml/train \
     -H "Content-Type: application/json" \
     -d '{"asset":"BTCUSDT","timeframe":"M5"}'
   ```
3. Vérifier que les fichiers ont été créés sur disque :
   ```bash
   ls -lh data/modele_rf.json data/modele_lstm.json
   ```
4. **Redémarrer le backend** (Ctrl+C puis relance)
5. Vérifier l'état du modèle sans ré-entraînement :
   ```bash
   curl http://localhost:8080/api/ml/status
   ```

### Résultat attendu
- ✅ `data/modele_rf.json` et `data/modele_lstm.json` créés après entraînement
- ✅ Après redémarrage : statut `"entraine": true` sans relancer l'entraînement
- ✅ Logs au démarrage : `Modèle RF chargé depuis disque` / `Modèle LSTM chargé depuis disque`

### Scénario d'échec à tester
1. Supprimer les fichiers de modèle
2. Relancer le backend
3. Vérifier que le backend **ne crashe pas** et log `Aucun modèle trouvé — entraînement requis`

---

## T02 — Risk Management : limite d'exposition par actif ⭐ CRITIQUE

**Pourquoi** : La règle `MAX_EXPOSITION_ACTIF = 25%` n'était pas appliquée — correction de sécurité financière.

### Prérequis
- Capital simulé : 10 000 €
- Limite : 25% = 2 500 € max par actif

### Cas 1 — Signal refusé (exposition dépassée)
```bash
# Ouvrir une première position BTC (20% = 2000€ — OK)
curl -X POST http://localhost:8080/api/signal \
  -H "Content-Type: application/json" \
  -d '{"asset":"BTCUSDT","timeframe":"M5","taille_position":2000}'

# Tenter une deuxième position BTC (10% = 1000€ — dépasserait 25%)
curl -X POST http://localhost:8080/api/signal \
  -H "Content-Type: application/json" \
  -d '{"asset":"BTCUSDT","timeframe":"M5","taille_position":1000}'
```

**Résultat attendu** : 2ème signal → `{ "accepte": false, "raison": "Exposition max par actif dépassée" }`

### Cas 2 — Signal accepté (actif différent)
```bash
# Même capital, mais sur ETH (actif différent)
curl -X POST http://localhost:8080/api/signal \
  -H "Content-Type: application/json" \
  -d '{"asset":"ETHUSDT","timeframe":"M5","taille_position":1000}'
```

**Résultat attendu** : Signal accepté ✅

---

## T03 — WebSocket temps réel ⭐ CRITIQUE

**Pourquoi** : Nouvelle fonctionnalité `/api/stream` — non testée en production.

### Test connexion WebSocket
```bash
# Nécessite wscat : npm install -g wscat
wscat -c "ws://localhost:8080/api/stream?asset=BTCUSDT&timeframe=M5"
```

**Résultat attendu** (message reçu toutes les ~5s) :
```json
{
  "type": "candle",
  "asset": "BTCUSDT",
  "timeframe": "M5",
  "data": {
    "timestamp": 1741694400,
    "open": 85000.0,
    "high": 85200.0,
    "low": 84900.0,
    "close": 85100.0,
    "volume": 123.45
  }
}
```

### Test depuis l'app Tauri
- Ouvrir la vue **Charts**
- Vérifier que les bougies se mettent à jour sans recharger la page
- Vérifier qu'il n'y a pas d'erreur dans la console Tauri (DevTools)

### Cas de déconnexion
- Couper le backend pendant que le WebSocket est connecté
- Relancer le backend
- Vérifier que le frontend se reconnecte automatiquement (ou affiche une erreur claire)

---

## T04 — Backtest multi-stratégie

**Pourquoi** : Avant la correction, seule la stratégie Straddle était accessible via l'API backtest.

### Stratégie SMC Directionnel
```bash
curl -X POST http://localhost:8080/api/backtest \
  -H "Content-Type: application/json" \
  -d '{
    "asset": "BTCUSDT",
    "timeframe": "M5",
    "capital": 10000.0,
    "strategie": "smc"
  }'
```

**Résultat attendu** : Objet de résultat avec trades utilisant la logique SMC (scoring ≥70/100)

### Stratégie Straddle (régression)
```bash
curl -X POST http://localhost:8080/api/backtest \
  -H "Content-Type: application/json" \
  -d '{
    "asset": "BTCUSDT",
    "timeframe": "M5",
    "capital": 10000.0,
    "strategie": "straddle"
  }'
```

**Résultat attendu** : Résultat avec trades Straddle (positions LONG+SHORT simultanées)

### Sans champ strategie (défaut = Straddle)
```bash
curl -X POST http://localhost:8080/api/backtest \
  -H "Content-Type: application/json" \
  -d '{"asset":"BTCUSDT","timeframe":"M5","capital":10000.0}'
```

**Résultat attendu** : Même résultat que Straddle ✅

---

## T05 — Démarrage du backend (robustesse)

**Pourquoi** : Le `expect()` au démarrage a été remplacé — vérifier que les erreurs sont bien gérées.

### Cas nominal
```bash
cd backend && cargo run --release
```
**Résultat attendu** : `🌐 Server running on http://0.0.0.0:8080`

### Cas base de données inaccessible
1. Changer temporairement le chemin DB dans la config pour pointer vers un dossier inexistant
2. Lancer le backend

**Résultat attendu** : Message d'erreur explicite dans les logs + exit propre (pas de panic)

---

## T06 — Flux nominal complet (smoke test)

Vérification end-to-end que rien n'est cassé par les refactorisations.

```bash
# 1. Santé du backend
curl http://localhost:8080/api/health

# 2. Récupération des bougies
curl "http://localhost:8080/api/candles?asset=BTCUSDT&timeframe=M5&limit=10"

# 3. Analyse SMC
curl -X POST http://localhost:8080/api/smc/analyze \
  -H "Content-Type: application/json" \
  -d '{"asset":"BTCUSDT","timeframe":"M5"}'

# 4. Génération de signal
curl -X POST http://localhost:8080/api/signal \
  -H "Content-Type: application/json" \
  -d '{"asset":"BTCUSDT","timeframe":"M5"}'

# 5. Historique des trades
curl "http://localhost:8080/api/history?asset=BTCUSDT&limit=20"
```

**Résultat attendu** : Tous les endpoints répondent sans erreur 500.

---

## T07 — Interface Tauri (vérifications visuelles)

### Dashboard
- [ ] Les métriques s'affichent (P&L, Win Rate, Sharpe)
- [ ] Les alertes d'erreur affichent un message lisible (pas `[object Object]`)

### Vue Charts
- [ ] TradingView charge les bougies correctement
- [ ] Le sélecteur d'asset/timeframe fonctionne
- [ ] Aucune erreur TypeScript dans la console DevTools Tauri

### Vue SMC Analyzer
- [ ] Les indicateurs SMC s'affichent (Order Blocks, Imbalance, IFVG, Fibonacci)
- [ ] Le score de confluence est calculé et affiché

### Vue Historique
- [ ] La liste des trades passés est visible
- [ ] Les filtres par asset et timeframe fonctionnent

### Vue Paramètres
- [ ] La configuration se sauvegarde et se recharge après redémarrage de l'app

---

## T08 — Performance (seuils critiques)

| Métrique | Seuil | Mesure |
|---|---|---|
| Inférence ML complète | < 200 ms | Logs backend : `Inference: Xs` |
| Génération signal | < 10 s | Mesurer avec `time curl ...` |
| Réponse backtest (100 bougies) | < 5 s | Mesurer avec `time curl ...` |

```bash
# Mesurer la latence de signal
time curl -X POST http://localhost:8080/api/signal \
  -H "Content-Type: application/json" \
  -d '{"asset":"BTCUSDT","timeframe":"M5"}' -s > /dev/null
```

---

## Récapitulatif

| # | Test | Priorité | Couverture |
|---|---|---|---|
| T01 | Persistance modèles ML | 🔴 Critique | Nouveau |
| T02 | Exposition par actif 25% | 🔴 Critique | Corrigé |
| T03 | WebSocket `/api/stream` | 🔴 Critique | Nouveau |
| T04 | Backtest multi-stratégie | 🟠 Important | Corrigé |
| T05 | Démarrage robuste | 🟠 Important | Corrigé |
| T06 | Smoke test flux complet | 🟡 Standard | Régression |
| T07 | UI Tauri visuelle | 🟡 Standard | Régression |
| T08 | Seuils de performance | 🟢 Optionnel | Perf |
