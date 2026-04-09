# Diagramme des Communications Extérieures — Native Trading AI

> Inventaire exhaustif de toutes les APIs, WebSockets et fonctions qui font communiquer l'application avec l'extérieur (services tiers, protocoles réseau, LLMs, brokers).

---

## 1. Services Externes Consommés par le Backend (Rust)

### 1.1 IB Gateway (Interactive Brokers)
| Élément | Détail |
|---|---|
| **Protocole** | TCP propriétaire IBapi (bibliothèque `ibapi`) |
| **Adresse** | `127.0.0.1:4002` (paper) ou `127.0.0.1:4001` (live) — local machine |
| **Fichiers** | `data/src/providers/ibgateway.rs`, `api/src/ws_handlers/ib.rs`, `api/src/handlers.rs` |

**Fonctions qui se connectent à IB :**

- **`IbGatewayProvider::fetch_candles()`** — Télécharge les bougies historiques (OHLCV) pour métaux, forex et indices. Crée une connexion TCP IB par appel. Utilisé dans `get_candles`, `straddle_handlers`, `tendance_handlers`, `data_handlers`, `backtest_handlers`.
- **`ib_status()` handler** — Vérifie si IB Gateway est joignable via `ibapi::Client::connect()`. Résultat mis en cache 90 secondes pour éviter la saturation (cf. bug CLOSE_WAIT).
- **`stream_ib()` dans `ws_handlers/ib.rs`** — Connexion IB persistante pour le streaming temps réel des bougies chart (`historical_data_streaming`) et du prix bid/ask (`tick_by_tick_bid_ask`). Utilisé quand le frontend ouvre un graphique sur un asset non-crypto. Compteur atomique `IB_OFFSET_COUNTER` pour éviter les conflits de `client_id`.

---

### 1.2 Binance REST API
| Élément | Détail |
|---|---|
| **Protocole** | HTTPS REST |
| **Base URL** | `https://api.binance.com` |
| **Authentification** | Aucune (endpoints publics) |
| **Fichiers** | `data/src/providers/binance.rs`, `api/src/sentiment_handlers.rs`, `api/src/rockets_scan.rs`, `api/src/rockets_suivi.rs`, `api/src/rockets_analyse.rs`, `api/src/handlers.rs`, `api/src/prix_utils.rs` |

**Endpoints utilisés :**

| Endpoint | Utilisé dans | Rôle |
|---|---|---|
| `GET /api/v3/klines` | `BinanceProvider::fetch_candles()`, `rockets_analyse.rs` | Bougies historiques BTC/ETH et crypto Rockets (1h OHLCV) |
| `GET /api/v3/ticker/price?symbol=X` | `handlers.rs` (`get_prix_actuel`), `rockets_suivi.rs`, `prix_utils.rs` | Prix actuel d'un asset crypto |
| `GET /api/v3/ticker/24hr` | `sentiment_handlers.rs`, `rockets_scan.rs` | Variation 24h, volume, nb trades — sentiment marché + scan momentum Rockets |

---

### 1.3 Yahoo Finance REST API
| Élément | Détail |
|---|---|
| **Protocole** | HTTPS REST (non-officiel, pas de clé API) |
| **Base URL** | `https://query2.finance.yahoo.com` |
| **Fichiers** | `api/src/sentiment_handlers.rs`, `api/src/prix_utils.rs` |

**Endpoints utilisés :**

| Endpoint | Utilisé dans | Rôle |
|---|---|---|
| `GET /v8/finance/chart/{ticker}?interval=1d&range=2d` | `sentiment_handlers.rs`, `prix_utils.rs` | Prix spot des indices (S&P500, Nasdaq, DAX, CAC40, etc.), métaux (Or, Argent, Pétrole) et VIX |

**Tickers interrogés :** `^GSPC`, `^IXIC`, `^DJI`, `^N100`, `^GDAXI`, `^FCHI`, `GC=F`, `SI=F`, `CL=F`, `ZC=F`, `^VIX`

---

### 1.4 Ollama (LLM Local)
| Élément | Détail |
|---|---|
| **Protocole** | HTTP REST |
| **URL** | `http://localhost:11434/api/chat` (env `OLLAMA_URL`) |
| **Modèle par défaut** | `qwen2.5vl:7b` (env `OLLAMA_MODEL`) ; `qwen2.5:14b` pour ajustements straddle |
| **Authentification** | Aucune (service local) |
| **Fichiers** | `api/src/ollama/types.rs`, `api/src/ollama/mod.rs`, `api/src/straddle_signal_ollama.rs`, `api/src/smc_signal_ollama.rs`, `api/src/ollama_ajustements_handler.rs`, `api/src/straddle_signal_handler.rs`, `api/src/news_traduction.rs`, `api/src/ollama_chat_handler.rs` |

**Fonctions qui appellent Ollama :**

| Fonction / Handler | Modèle | Rôle |
|---|---|---|
| `ollama::interroger()` | `OLLAMA_MODEL` | Interface générique — prompt texte → réponse texte |
| `ollama::interroger_chat_modele()` | Spécifié par caller | Chat multi-tours avec historique |
| `smc_confirm::enrichir_signal_avec_ollama()` | `OLLAMA_MODEL` | Enrichissement narratif des signaux SMC (analyse LLM de confluence) |
| `straddle_signal_ollama::appeler_ollama_et_publier()` | `OLLAMA_MODEL` | Analyse LLM avant publication signal Straddle — juge si signal est viable |
| `smc_signal_ollama::appeler_smc_et_publier()` | `OLLAMA_MODEL` | Idem pour signaux SMC |
| `ollama_ajustements_handler::ajustements()` | `OLLAMA_MODEL` ou `qwen2.5:14b` | Optimisation paramètres Straddle par LLM à partir des résultats backtest |
| `ollama_handlers::analyser()` | `OLLAMA_MODEL` | Analyse IA générale d'un setup trading (POST `/api/ia/analyse`) |
| `ollama_handlers::chat()` | Choisi dynamiquement | Chat interactif Coach IA (POST `/api/ia/chat`) |
| `ollama_handlers::generer_diagram()` | `qwen2.5-coder:14b` | Génération de diagrammes SMC SVG (POST `/api/ia/diagram`) |
| `ollama_handlers::analyser_chart()` | Vision | Analyse visuelle d'un screenshot de chart via LLM vision |
| `ollama_handlers::generer_signal()` | `OLLAMA_MODEL` | Génération signal IA pur (POST `/api/ia/signal`) |
| `news_traduction::traduire()` | `OLLAMA_MODEL` | Traduction FR des titres/contenus news en anglais |
| `ollama_chat_handler` | `qwen2.5:14b` | Chat interactif avec contexte backtest injecté |

**Endpoint de contrôle :**
- `GET http://localhost:11434/api/tags` — Vérifie si Ollama est actif et liste les modèles disponibles (appelé dans `ollama_chat_handler`)

---

### 1.5 Anthropic Claude (API Cloud)
| Élément | Détail |
|---|---|
| **Protocole** | HTTPS REST |
| **Base URL** | `https://api.anthropic.com/v1/messages` |
| **Modèle** | `claude-sonnet-4-5` (vision + texte) |
| **Authentification** | Clé API `ANTHROPIC_API_KEY` — stockée en DB (`anthropic_api_key`) |
| **Fichiers** | `api/src/anthropic.rs`, `api/src/ollama_chart_handler.rs` |

**Fonctions :**

| Fonction | Rôle |
|---|---|
| `anthropic::analyser_images_claude()` | Analyse visuelle d'un ou plusieurs screenshots de chart (POST `/api/ia/analyser-chart-local`) — fallback ou alternative à Ollama vision |
| `anthropic::chat_claude()` | Chat interactif avec Claude via l'API cloud (POST `/api/ia/chat` avec `forcer_ollama: false`) |

---

### 1.6 Telegram Bot API
| Élément | Détail |
|---|---|
| **Protocole** | HTTPS REST |
| **URL** | `https://api.telegram.org/bot{TOKEN}/sendMessage` |
| **Authentification** | `TELEGRAM_BOT_TOKEN` + `TELEGRAM_CHAT_ID` — DB ou variables d'environnement (`telegram.env`) |
| **Fichiers** | `api/src/telegram.rs`, `api/src/straddle_signal_ollama.rs`, `api/src/smc_signal_ollama.rs`, `api/src/rockets_sauvegarder.rs`, `api/src/ollama_signal_ia_handler.rs` |

**Fonction :**

- **`telegram::notifier_telegram()`** — Envoie une notification HTML formatée à chaque nouveau signal validé (Straddle, SMC, Rockets, IA). Spawn Tokio détaché — n'impacte jamais la latence du signal engine. Si token vide → silencieux.

**Déclencheurs :**
- Signal Straddle confirmé par LLM → `straddle_signal_ollama.rs`
- Signal SMC confirmé par LLM → `smc_signal_ollama.rs`
- Signal Rockets sauvegardé → `rockets_sauvegarder.rs`
- Signal IA généré → `ollama_signal_ia_handler.rs`

---

### 1.7 Alternative.me — Fear & Greed Index
| Élément | Détail |
|---|---|
| **Protocole** | HTTPS REST |
| **Endpoint** | `GET https://api.alternative.me/fng/?limit=1` |
| **Authentification** | Aucune (API publique) |
| **Cache** | TTL 1 heure (`fear_greed_cache` dans AppState) |
| **Fichier** | `api/src/news_fear_greed.rs` |

**Rôle :** Retourne le score Fear & Greed Index Bitcoin (0–100) pour pondérer les signaux SMC. Score < 20 (peur extrême) → pénalité de confiance sur signaux haussiers.

---

### 1.8 Forex Factory — Calendrier Économique
| Élément | Détail |
|---|---|
| **Protocole** | HTTPS REST (JSON) |
| **Endpoints** | `GET https://nfs.faireconomy.media/ff_calendar_thisweek.json` <br> `GET https://nfs.faireconomy.media/ff_calendar_nextweek.json` |
| **Authentification** | Aucune (API publique) |
| **Fichier** | `api/src/calendar_handlers.rs` |

**Rôle :** Calendrier des événements économiques de la semaine (NFP, CPI, BCE, Fed...). Affiché dans la vue calendrier du frontend.

---

### 1.9 Flux RSS — Sources Actualités Financières
| Élément | Détail |
|---|---|
| **Protocole** | HTTPS (RSS/XML) |
| **Fichier** | `api/src/news_handlers.rs` |

**Sources configurées :**

| URL RSS | Source | Score pertinence |
|---|---|---|
| `https://feeds.reuters.com/reuters/businessNews` | Reuters | 42 |
| `https://search.cnbc.com/rs/search/combinedcms/view.xml?...` | CNBC | 40 |
| `https://feeds.marketwatch.com/marketwatch/marketpulse/` | MarketWatch | 38 |
| `https://cointelegraph.com/rss` | CoinTelegraph | 30 |
| `https://finance.yahoo.com/news/rssindex` | Yahoo Finance News | 35 |
| `https://cryptonews.com/news/feed/` | CryptoNews | 28 |
| `https://decrypt.co/feed` | Decrypt | 30 |
| `https://www.fxstreet.com/rss/news` | FXStreet | 38 |
| `https://www.kitco.com/news/rss/metals-news.xml` | Kitco (métaux) | 36 |

**Rôle :** Agrégation news financières, scoring de pertinence par asset, traduction FR via Ollama, affichage dans le panneau news du dashboard.

---

## 2. WebSockets — Streaming Temps Réel

### 2.1 Backend → Binance WebSocket (upstream)
| Élément | Détail |
|---|---|
| **URL** | `wss://stream.binance.com:9443/ws/{symbol}@kline_{interval}` |
| **Direction** | Backend → Binance (client) |
| **Fichier** | `api/src/ws_handlers/binance.rs` |

**Rôle :** Le backend se connecte au stream Binance pour recevoir les bougies temps réel d'un asset crypto (BTC, ETH, SOL, etc.) et les retransmet au frontend via le WebSocket interne `/api/stream`. Déclenché quand un onglet graphique est ouvert sur un asset crypto.

---

### 2.2 Backend → Frontend WebSocket : Stream Graphique
| Élément | Détail |
|---|---|
| **Route** | `GET ws://localhost:8080/api/stream?asset=X&timeframe=Y` |
| **Direction** | Frontend → Backend (client WS), Backend → Binance ou IB (upstream) |
| **Fichier** | `api/src/ws_handlers/mod.rs` |

**Rôle :** Proxy WebSocket qui dispatche vers `stream_binance` (crypto) ou `stream_ib` (métaux/forex/indices) selon l'asset. Le frontend `market.store.ts` s'y connecte quand une vue graphique est ouverte. Envoie des `CandleEvent` JSON au client.

**Variante multi-assets :** `market.store.ts` ouvre un WS par asset dans `connecterPrixLiveAssets()` sur le timeframe M1 pour alimenter les mini-graphiques du dashboard.

---

### 2.3 Backend → Frontend WebSocket : Stream Signaux
| Élément | Détail |
|---|---|
| **Route** | `GET ws://localhost:8080/api/signal-engine/stream` |
| **Direction** | Frontend → Backend (abonnement) |
| **Fichiers** | `api/src/engine_handlers.rs`, `frontend/src/App.vue`, `frontend/src/composables/useSignalEngine.ts` |

**Rôle :** Stream temps réel des signaux générés par le Signal Engine. Le backend publie via un canal `broadcast::Sender<Signal>`. **Deux connexions WS ouvertes simultanément depuis le frontend :**
- `App.vue` → connexion globale, alimente `signal-alarme.store` pour les notifications portables
- `useSignalEngine.ts` → connexion du composable dashboard, alimente `signal.store` pour le tableau

---

### 2.4 Frontend → Binance WebSocket (direct)
| Élément | Détail |
|---|---|
| **URL** | `wss://stream.binance.com:9443/ws/!ticker@arr` |
| **Direction** | Frontend → Binance (direct, sans passer par le backend) |
| **Fichier** | `frontend/src/stores/prix.store.ts` |

**Rôle :** Stream global de tous les tickers Binance 24h pour alimenter la Heatmap et le tableau de prix des crypto. Reconnexion automatique (5s). Si bloqué → fallback HTTP polling (`GET https://api.binance.com/api/v3/ticker/24hr`) toutes les 30s.

---

## 3. Appels Extérieurs Directs du Frontend (sans passer par le backend)

### 3.1 Binance REST — Opportunités Crypto
| Élément | Détail |
|---|---|
| **URL** | `GET https://api.binance.com/api/v3/ticker?symbols=[...]&windowSize=1h` |
| **Fichier** | `frontend/src/composables/useCryptosAlert.ts` |

**Rôle :** Chargement des prix et variations 1h d'un ensemble de symboles crypto pour détecter les opportunités de momentum (composant CryptosAlert). Appel HTTP direct sans passer par le backend Rust.

### 3.2 Binance REST — Bootstrap prix 24h
| Élément | Détail |
|---|---|
| **URL** | `GET https://api.binance.com/api/v3/ticker/24hr` |
| **Fichier** | `frontend/src/stores/prix.store.ts` |

**Rôle :** Chargement initial de tous les tickers avant que le WebSocket ne soit établi, et fallback polling si le WS est coupé.

---

## 4. REST API Interne (Backend → Frontend)

> Le backend Actix-Web expose les routes suivantes sur `http://localhost:8080`. Le frontend les consomme via Axios (`api.service.ts` et fichiers complémentaires).

### 4.1 Routes Système & Santé
| Méthode | Route | Rôle |
|---|---|---|
| GET | `/health` | Health check backend |
| GET | `/api/ib/status` | Statut IB Gateway (cache 90s, 1 connexion ibapi max/90s) |
| GET | `/api/ia/status` | Statut Ollama (disponibilité + modèles) |
| GET | `/api/config?cle=X` | Lire une valeur de configuration en DB |
| POST | `/api/config` | Écrire une valeur de configuration en DB |

### 4.2 Routes Assets & Marché
| Méthode | Route | Rôle |
|---|---|---|
| GET | `/api/assets` | Liste des assets gérés |
| POST | `/api/assets` | Ajouter un asset |
| DELETE | `/api/assets/{id}` | Supprimer un asset |
| GET | `/api/assets/params` | Paramètres par asset (spread, pip, value_pip) |
| PUT | `/api/assets/params` | Mettre à jour les paramètres par asset |
| GET | `/api/candles?asset=X&timeframe=Y&limit=N` | Bougies OHLCV (cache DB → IB/Binance si absent) |
| GET | `/api/prix?assets[]=X` | Prix agrégés multi-assets |
| GET | `/api/prix-actuel?ticker=X` | Prix spot unique (Binance ou Yahoo Finance) |
| GET | `/api/stream?asset=X&timeframe=Y` *(WS upgrade)* | Stream bougies temps réel (voir section 2.2) |

### 4.3 Routes Signaux & Stratégies
| Méthode | Route | Rôle |
|---|---|---|
| GET | `/api/signaux?limit=N` | Historique des signaux |
| POST | `/api/signaux/export` | Export CSV des signaux filtrés |
| GET | `/api/smc/analyse?asset=X&timeframe=Y` | Analyse SMC complète (5 indicateurs) |
| GET | `/api/smc/debug-score` | Score SMC détaillé pour debug |
| GET | `/api/signal-engine/status` | Statut du Signal Engine automatique |
| POST | `/api/signal-engine/start` | Démarrer le Signal Engine |
| POST | `/api/signal-engine/stop` | Stopper le Signal Engine |
| GET | `/api/signal-engine/stream` *(WS upgrade)* | Stream signaux temps réel (voir section 2.3) |

### 4.4 Routes ML
| Méthode | Route | Rôle |
|---|---|---|
| GET | `/api/ml/predict?asset=X&timeframe=Y` | Inférence ML hybride (LSTM+XGBoost) |
| POST | `/api/ml/train` | Entraînement ML manuel |
| GET | `/api/ml/status` | Statut pipeline ML |
| GET | `/api/ml/history` | Historique des entraînements |
| POST | `/api/backtest/raffiner-ml` | Backtest + raffinement ML (requiert ≥62 bougies IB) |
| GET | `/api/ml/feedback/stats` | Statistiques feedback ML |
| GET | `/api/ml/suggestions` | Suggestions d'optimisation ML |
| POST | `/api/ml/suggestions/appliquer` | Appliquer une suggestion ML |
| POST | `/api/ml/retrain` | Déclencher un ré-entraînement incrémental |
| GET | `/api/ml/retrain` | Dernier statut de ré-entraînement |
| GET | `/api/ml/retrain/{job_id}` | Statut d'un job de ré-entraînement |

### 4.5 Routes IA / Ollama
| Méthode | Route | Rôle |
|---|---|---|
| POST | `/api/ia/analyse` | Analyse IA d'un setup (prompt + contexte backtest) |
| POST | `/api/ia/chat` | Chat Coach IA (Ollama ou Claude selon `forcer_ollama`) |
| POST | `/api/ia/diagram` | Génération diagramme SVG SMC par LLM |
| POST | `/api/ia/analyser-chart` | Analyse visuelle chart (Ollama vision) |
| POST | `/api/ia/analyser-chart-local` | Analyse visuelle chart (Claude Anthropic) |
| POST | `/api/ia/signal` | Génération signal IA pur |
| POST | `/api/ia/ajustements` | Optimisation paramètres Straddle par LLM |
| GET | `/api/ia/ab-test` | Statistiques A/B test stratégies |

### 4.6 Routes Backtest & Straddle
| Méthode | Route | Rôle |
|---|---|---|
| POST | `/api/backtest` | Backtest Straddle (capital, asset, TF, paramètres) |
| POST | `/api/straddle/analyser` | Analyse créneaux horaires de volatilité (LLM) |
| GET | `/api/straddle/creneaux` | Liste des créneaux de volatilité détectés |
| PATCH | `/api/straddle/creneaux/{id}` | Mettre à jour un créneau (winrate, statut) |
| POST | `/api/straddle/creneaux/{id}/precision` | Analyse précision timing d'un créneau |
| POST | `/api/straddle/backtest` | Backtest ciblé sur un créneau |
| GET | `/api/straddle/params` | Paramètres Straddle |
| PUT | `/api/straddle/params` | Sauvegarder paramètres Straddle |
| GET | `/api/straddle/volatilite-live` | Volatilité actuelle vs histórique |
| GET | `/api/straddle/monitoring-ml` | Monitoring ML Straddle adaptatif |
| GET | `/api/straddle/calibration` | Calibration seuils Straddle |
| GET | `/api/straddle/pics` | Pics ATR détectés |
| GET | `/api/straddle/feedback` | Feedback signaux Straddle ouverts |
| POST | `/api/straddle/feedback/{id}/cloturer` | Clôturer un signal Straddle |
| POST | `/api/straddle/signal` | Générer signal Straddle manuellement |

### 4.7 Routes SMC
| Méthode | Route | Rôle |
|---|---|---|
| GET | `/api/smc/params` | Paramètres SMC |
| PUT | `/api/smc/params` | Sauvegarder paramètres SMC |
| GET | `/api/smc/monitoring-ml` | Monitoring ML SMC |
| GET | `/api/smc/calibration` | Calibration seuils SMC |
| GET | `/api/smc/feedback` | Feedback signaux SMC ouverts |
| GET | `/api/smc-analyse` | Dernière analyse SMC LLM |
| POST | `/api/smc-analyse` | Lancer une analyse SMC LLM |

### 4.8 Routes Rockets
| Méthode | Route | Rôle |
|---|---|---|
| POST | `/api/rockets/signal` | Sauvegarder un signal Rockets |
| GET | `/api/rockets/scan` | Scan momentum crypto (Binance 24h) |
| GET | `/api/rockets/scan-debug` | Scan momentum debug |
| GET | `/api/rockets/historique` | Historique signaux Rockets |
| POST | `/api/rockets/sync` | Synchroniser verdicts Rockets (prix actuels Binance) |
| GET | `/api/rockets/config` | Config seuils Rockets |
| PUT | `/api/rockets/config` | Sauvegarder config Rockets |
| GET | `/api/rockets/monitoring-ml` | Monitoring ML Rockets |
| GET | `/api/rockets/calibration` | Calibration seuils Rockets |
| GET | `/api/rockets/feedback` | Feedback Rockets |
| POST | `/api/rockets/analyse-llm` | Lancer analyse LLM Rockets |
| GET | `/api/rockets/analyse-llm` | Récupérer dernière analyse LLM Rockets |

### 4.9 Routes Informations Marché
| Méthode | Route | Rôle |
|---|---|---|
| GET | `/api/tendance/multi-tf` | Tendance multi-timeframe (3 TF) |
| GET | `/api/indicators?asset=X&timeframe=Y` | Indicateurs techniques (ATR, RSI, MACD, Bollinger) |
| GET | `/api/volatility/patterns` | Patterns de volatilité horaire historiques |
| GET | `/api/calendrier` | Calendrier économique (Forex Factory) |
| GET | `/api/sentiment` | Sentiment marché (Yahoo Finance + Binance) |
| GET | `/api/news` | Alertes news filtrées (RSS agrégé) |
| GET | `/api/news/content?url=X` | Contenu scraped d'un article (SSRF protégé) |
| GET | `/api/news/traduire?url=X` | Traduction FR d'un article via Ollama |
| GET | `/api/news/fear-greed` | Fear & Greed Index (cache 1h) |
| POST | `/api/news/lus` | Marquer un article comme lu |
| GET | `/api/news/lus` | Liste des articles lus |
| GET | `/api/news/contexte` | Contexte marché synthétisé (news + sentiment) |

### 4.10 Routes Data Management
| Méthode | Route | Rôle |
|---|---|---|
| GET | `/api/data/coverage` | Couverture données en base (asset × TF) |
| POST | `/api/data/collect` | Collecte bulk données IB/Binance en base |
| POST | `/api/data/import-mt5` | Import fichier CSV MetaTrader 5 |

### 4.11 Routes Prompts & Config Avancée
| Méthode | Route | Rôle |
|---|---|---|
| GET | `/api/prompts` | Lister les prompts LLM personnalisables |
| PUT | `/api/prompts/{id}` | Modifier un prompt |
| DELETE | `/api/prompts/{id}` | Restaurer un prompt à la valeur par défaut |

---

## 5. Résumé Architectural

```
┌─────────────────────────────────────────────────────────────────────┐
│                        MONDE EXTÉRIEUR                              │
│                                                                     │
│  Binance REST API          IB Gateway (local)    Forex Factory      │
│  (crypto OHLCV, prix)      (métaux, forex, idx)  (calendrier éco)   │
│                                                                     │
│  Yahoo Finance REST        alternative.me         Reuters/CNBC RSS  │
│  (indices, VIX, métaux)    (Fear&Greed Index)     (news financières) │
│                                                                     │
│  Ollama (localhost:11434)  Anthropic Claude API   Telegram Bot API   │
│  (LLM local, 9 modèles)    (vision + chat cloud)  (notifications)   │
│                                                                     │
│  wss://stream.binance.com  (direct frontend)                        │
└──────────────────────────────┬──────────────────────────────────────┘
                               │
              ┌────────────────┴────────────────┐
              │       BACKEND RUST              │
              │       Actix-Web :8080           │
              │       (60+ routes REST)         │
              │       (2 endpoints WS serveur)  │
              └────────────────┬────────────────┘
                               │
              ┌────────────────┴────────────────┐
              │       FRONTEND VUE 3            │
              │       (Tauri natif)             │
              │  WS stream graphiques           │
              │  WS stream signaux (×2)         │
              │  REST via Axios (8 fichiers)    │
              │  WS Binance direct (prix)       │
              └─────────────────────────────────┘
```

### Points d'attention
- **Anthropic Claude** : payant, nécessite `anthropic_api_key` en DB — silencieux si absent
- **Telegram** : optionnel, silencieux si tokens absents
- **IB Gateway** : local machine uniquement, pas de clé API — authentification par `client_id`
- **Ollama** : local machine, aucune donnée envoyée à l'extérieur
- **Binance** : endpoints publics, aucune clé requise
- **Yahoo Finance** : endpoint non-officiel, peut être bloqué sans préavis
- **Forex Factory** : endpoint JSON public, pas de garantie SLA
- **SSRF protection** : `news_scraper.rs::est_url_externe_sure()` bloque localhost, RFC 1918 et toute URL non-HTTPS pour le scraping d'articles
