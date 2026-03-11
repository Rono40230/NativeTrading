# 🧪 Plan de tests — Native Trading AI

Tests à exécuter après les corrections apportées lors de l'audit Phase 2.  
Classés par ordre d'importance décroissant.

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
