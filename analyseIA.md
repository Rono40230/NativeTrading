# Plan d'implémentation — Amélioration IA

> Trois remplacements de modèles, par ordre de priorité.  
> L'architecture backend (relay via Rust, clé jamais exposée au frontend) reste inchangée.

---

## Priorité 1 — Chart Import → Claude Sonnet API

**Pourquoi en premier** : C'est la fonctionnalité actuellement inutilisable. `llama3.2-vision:11b` produit des analyses non exploitables en trading. C'est le gain immédiat le plus fort.

### État actuel
- Fichier : `backend/crates/api/src/ollama/mod.rs`
- Constante : `MODELE_VISION = "llama3.2-vision:11b"`
- Fonction : `pub async fn analyser_images(...)` → POST sur `http://localhost:11434/api/chat` avec images en base64
- Handler : `ollama_handlers.rs` → `analyser_chart()` → `POST /api/ia/chart`

### Ce qui change

**1. Config** — Ajouter la clé Anthropic dans la table `config` SQLite (déjà utilisée pour IB, capital, etc.) :
- Clé : `anthropic_api_key`
- Ajout dans `SettingsView.vue` → section "IA Vision" avec champ masqué (type password)
- Sauvegarde via l'endpoint `/api/config` existant

**2. Backend** — Remplacer `analyser_images()` dans `ollama/mod.rs` :
- URL : `https://api.anthropic.com/v1/messages`
- Headers : `x-api-key`, `anthropic-version: 2023-06-01`, `content-type: application/json`
- Body : `model: "claude-sonnet-4-5"`, `max_tokens: 4000`, `system: ANALYST_PROMPT`, messages avec images base64
- La clé est lue depuis la DB via `AppState` → jamais dans une variable d'environnement ni dans le frontend

**3. Prompt** — Intégrer le `ANALYST_PROMPT` de `App.jsx` dans `ollama/prompts.rs` (ou nouveau fichier `anthropic/prompts.rs`) :
- Structure imposée : Vue d'ensemble, Structure marché, Liquidité, POI, Scénarios, Niveaux, Conclusion /10
- Format JSON en option pour les niveaux clés (exploitables par le backend)

**4. Dégradation** — Si la clé est absente ou l'API inaccessible :
- Réponse claire : `"Clé Anthropic non configurée"` avec lien vers les settings
- Pas de fallback vers LLaVA (analyse trop mauvaise pour être utile)

**5. Frontend** — Aucun changement (le composant `ChartImportPanel.vue` et `useChartImport.ts` restent identiques)

### Coût estimé
~$0.02-0.03 par analyse · ~$5-15/mois usage normal

---

## Priorité 2 — Coach SMC → DeepSeek-R1 14B (local Ollama)

**Pourquoi en deuxième** : Remplacement à coût zéro, amélioration de qualité significative. `qwen2.5:3b` est trop petit pour du raisonnement SMC pédagogique. R1 14B tient confortablement dans 24 GB VRAM.

### État actuel
- Constante : `MODELE_COACH = "qwen2.5:3b"` dans `ollama/mod.rs`
- Fonction : `interroger_chat_modele(historique, MODELE_COACH)` → POST Ollama `/api/chat`
- Handler : `ollama_handlers.rs` → `chat_coach()` → `POST /api/ia/coach`
- Le `COACH_PROMPT` est déjà bien structuré (focus SMC, émojis, diagrammes HTML)

### Ce qui change

**1. Constante uniquement** — Modifier `MODELE_COACH` :
```
"qwen2.5:3b" → "deepseek-r1:14b"
```

**2. Prompt** — Ajuster le `COACH_PROMPT` si besoin :
- DeepSeek-R1 génère des balises `<think>...</think>` (chain-of-thought interne) → les filtrer avant d'envoyer la réponse au frontend
- Ajouter instruction : ignorer les balises de raisonnement interne dans la réponse visible

**3. Download** — Commande préalable :
```bash
ollama pull deepseek-r1:14b
```
~9 GB à télécharger. À documenter dans `README.md` section "Prérequis IA".

**4. Frontend** — Aucun changement.

### Avantage vs qwen2.5:3b
- Raisonnement en chaîne (chain-of-thought) → explications plus structurées
- Meilleure compréhension des concepts SMC complexes (liquidité, CHoCH, inducement)
- Génération de code HTML pour diagrammes plus fiable (le COACH_PROMPT actuel génère des `<htmldiagram>`)

---

## Priorité 3 — Scoring LLM Signals → DeepSeek API V3.1

**Pourquoi en troisième** : Améliore la qualité des `llm_raison` dans les signaux Straddle et SMC. Coût quasi nul (~$0.003/1000 tokens). Ne bloque pas le trading si non fait, contrairement aux deux précédents.

### État actuel
- `straddle_signal_handler.rs` : `OLLAMA_MODEL = "qwen2.5:14b"` (var env) → scoring LLM du signal Straddle
- `ollama_handlers.rs` → `generer_signal()` : prompt SMC structuré → LLM Ollama local
- `ollama/smc_analyse.rs`, `ollama/rockets_analyse.rs` : analyses texte via Ollama local
- `news_traduction.rs` : traductions via `MODELE_TRADUCTION` (Ollama) → **garder en local** (volume élevé, pas critique)

### Ce qui change

**1. Nouveau module** `anthropic_ou_deepseek` (ou extension d'`ollama/mod.rs`) :
- Client HTTP vers `https://api.deepseek.com/v1/chat/completions` (compatible OpenAI)
- Header : `Authorization: Bearer DEEPSEEK_API_KEY`
- Model : `deepseek-chat` (alias de V3)
- Fallback : si API indisponible → Ollama local qwen2.5:14b (conservé comme backup)

**2. Config** — Ajouter `deepseek_api_key` dans la table `config` SQLite, éditable dans les settings.

**3. Scope ciblé** — Uniquement les appels qui génèrent `llm_raison` (visible dans `SignalAlarmeModal`) :
- `straddle_signal_handler.rs` → scoring Straddle
- `ollama_handlers.rs` → `generer_signal()` → signal SMC
- **Exclure** : traductions news (volume trop élevé), analyse rockets (déjà bonne qualité avec 14B local)

**4. Frontend** — Aucun changement. Le champ `llm_raison` est déjà affiché dans `SignalAlarmeModal`.

### Avantage vs qwen2.5:14b local
- V3.1 est entraîné sur données financières quantitatives → meilleure compréhension des confluences SMC
- Raisonnement JSON plus fiable → moins de parse errors sur la réponse structurée
- Coût : ~$0.003 par signal analysé

---

## Résumé des fichiers impactés

| Priorité | Fichiers backend | Fichiers frontend | Nouveaux fichiers |
|---|---|---|---|
| 1 — Claude Sonnet | `ollama/mod.rs`, `ollama_handlers.rs` | `SettingsView.vue` (champ clé) | `anthropic/mod.rs` (ou inline) |
| 2 — DeepSeek-R1 local | `ollama/mod.rs` (1 ligne) | Aucun | Aucun |
| 3 — DeepSeek API V3.1 | `straddle_signal_handler.rs`, `ollama_handlers.rs` | Aucun | `deepseek/mod.rs` (optionnel) |

## Contraintes à respecter

- Clés API stockées en DB SQLite (table `config`), lues via `AppState` — jamais dans `.env` ni frontend
- Dégradation silencieuse si API externe indisponible (fallback Ollama local ou message d'erreur clair)
- Aucun `unwrap()` sur les appels HTTP externes
- Timeout explicite sur tous les appels API externes (30s max)
- Mesurer la latence de chaque appel avec `Instant::now()` + log `tracing::info!`
