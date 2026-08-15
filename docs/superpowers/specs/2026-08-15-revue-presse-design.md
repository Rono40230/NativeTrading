# Revue de presse — Design

> Phase 4.1 de la ROADMAP. Brainstormé et validé par le propriétaire le 2026-08-15.
> L'implémentation attendra les gates 2-3 (phases 2 et 3 prioritaires) — ce document
> est la référence figée d'ici là.

## Décisions propriétaire (2026-08-15)

| Sujet | Décision |
|---|---|
| Objectif (ordre de préférence) | 1. **Bibliothèque consultable** · 2. Brief quotidien synthétique · 3. Matière pour le trading · 4. Flux temps réel |
| Rôles de l'IA locale (Ollama) | Les quatre : traduire en FR, classifier (thème/assets/impact), générer le brief, analyser le sentiment |
| Sources RSS | **Pilotables par l'utilisateur** (DB + UI) — la liste actuelle des 9 flux devient le point de départ modifiable |
| Architecture | **Process séparé dédié** (`news-collector`), crash-isolé (gate 4) |
| Cadence de collecte | 30 minutes |
| Brief | **À la demande seulement** (bouton « Générer »), pas de génération planifiée |
| Rétention | **12 mois** par défaut, clé `retention_presse_mois` configurable |
| UI | **Vue dédiée** 📰 + extrait dashboard (5 dernières) |
| Approche IA | **C — Hybride** : collecteur sans IA (mots-clés), IA à la demande (lecture + brief) |
| Articles intraduisibles | Traduction = porte d'entrée : **échec ×2 → suppression** de l'article |

## Architecture

```
┌────────────────────────────┐        ┌─────────────────────────────┐
│ news-collector             │        │ Backend API (existant)      │
│ (process séparé, run.sh)   │        │                             │
│ cycle 30 min :             │ écrit  │ À LA DEMANDE :              │
│  sources (DB) → fetch RSS  ├───────►│  traduction FR + sentiment  │
│  → dédoublonnage (jaccard) │  DB    │  → cache (tables existantes)│
│  → scoring mots-clés       │        │  bouton « Générer le brief »│
│  → insertion articles      │        │  → Ollama qwen3             │
└────────────────────────────┘        └──────────┬──────────────────┘
                                                 │ GET /api/presse/*
┌─────────────────────────────────────┐          ▼
│ UI : vue dédiée 📰 + extrait dash   │◄─────────┘
└─────────────────────────────────────┘
```

**Pourquoi l'approche C (hybride)** : le scoring mots-clés existant classe dès la
collecte (filtres opérationnels immédiatement, sans IA) ; l'IA ne travaille que
sur ce qui est lu + le brief (coût minimal) ; le collecteur ne dépend jamais
d'Ollama (Ollama éteint = collecte intacte). L'approche B (pré-calcul IA de
tout) a été écartée : ~50 articles/jour × qwen3:32b ≈ 25 min de calcul
quotidien pour des articles parfois jamais lus.

**Nuance sur la classification** (rôle IA retenu au brainstorming) : en v1, la
classification thème/assets/impact est faite par **mots-clés** au collecteur
(gratuit, immédiat, filtrable dès l'insertion). L'IA n'y touche pas — un
affinage LLM de la classification (reclassement à la consultation, thèmes plus
fins) reste une évolution possible post-v1 si les mots-clés montrent leurs
limites.

## Schéma DB (migration unique)

| Table | Statut | Rôle |
|---|---|---|
| `presse_sources` | nouvelle | sources pilotables : `id, url_rss, nom, poids_score, actif, categorie, cree_le`. Pré-remplie avec les 9 flux actuels (Reuters 40, CNBC 35, MarketWatch 35, CoinTelegraph 30, Yahoo 28, CryptoNews 28, Decrypt 30, FXStreet 38, Kitco 38) |
| `presse_articles` | nouvelle | bibliothèque : `hash_titre (UNIQUE), titre, url, source_id, publie_le, resume_source, score, theme, assets_concernes (JSON), impact, statut_traduction ('non_tente'/'ok'/'echec'), ajoute_le` |
| `news_traductions` | existante, réutilisée | cache traductions (`hash_titre → titre_fr`) |
| `news_sentiment` | existante, réutilisée | cache sentiment |
| `news_lus` | existante, réutilisée | lu/non-lu |
| `presse_briefs` | nouvelle | briefs générés : `id, genere_le, fenetre_de, fenetre_a, contenu (markdown), nb_articles` |

**Rétention** : articles et briefs purgés au-delà de 12 mois (clé
`retention_presse_mois`), intégrée au job rétention existant (même mécanique
que bougies/observations/émissions).

## Le collecteur (binaire `news-collector`, cycle 30 min)

1. Lit `presse_sources` actives en DB
2. Fetch + parse RSS (réutilise `news_rss.rs` tel quel)
3. Dédoublonnage : jaccard bigrammes (existant) + `hash_titre` UNIQUE
4. Scoring mots-clés (existant) **+ attribution thème/assets** par les mêmes
   mots-clés (Fed→macro+USD, bitcoin→crypto+BTC, gold→métaux+XAU, Dax→indice…)
5. `INSERT OR IGNORE`. Fin — **aucune dépendance Ollama, aucun appel réseau
   autre que les flux RSS**

Implantation : `backend/crates/news/src/bin/collector.rs` (le crate news a déjà
sqlx) — pas de nouveau crate. Lancé par `run.sh` à côté du backend.

## IA à la demande

### Consultation d'un article (lazy, avec cache)

```
clic → POST /articles/{hash}/ouvrir
  traduction absente du cache ?
    Ollama : titre + résumé → FR (~5-10 s)
    succès   → cache, statut 'ok', affichage
    échec ×2 → statut 'echec' → article SUPPRIMÉ
  sentiment absent du cache ?
    Ollama : sentiment par asset cité (haussier/baissier/neutre)
    succès → cache (badge) ; échec → pas de badge (non bloquant, réessai au
    prochain ouverture)
```

**Règle de suppression** : la traduction est la porte d'entrée de la
bibliothèque — un titre intraduisible signale paywall/anti-robot/bruit, aucune
valeur pour une bibliothèque francophone. Deux tentatives, puis suppression.

### Le brief (bouton « Générer », à la demande)

1. Sélection des articles des **dernières 24 h** triés par score (top 15),
   traduits à la volée si besoin (seuls ceux du brief)
2. Un appel Ollama → **markdown structuré** : 3 lignes de contexte marché
   (thèmes dominants) + 5-10 articles marquants (2-3 lignes chacun, impact,
   assets)
3. Stockage `presse_briefs` — chaque clic crée une entrée horodatée
   (historique consultable, régénérable sans écrasement)

## API (préfixe `/api/presse`)

| Endpoint | Rôle |
|---|---|
| `GET /articles` | bibliothèque — `?theme=&asset=&source=&q=&lu=` + pagination (50/page) ; exclut les articles supprimés pour échec de traduction |
| `POST /articles/{hash}/ouvrir` | consultation → traduction + sentiment lazy + marquage lu |
| `POST /brief` | génère le brief 24 h (Ollama) |
| `GET /briefs` / `GET /briefs/{id}` | dernier brief + archives |
| `GET/POST/DELETE /sources` | pilotage des sources (comme les assets) |

## UI

- **Vue dédiée « 📰 Revue de presse »** (sidebar, groupe Outils) : filtres
  (thème · asset · source · recherche · lu/non-lu), liste (titre FR, source,
  ancienneté, badges impact/sentiment/assets), modal article (traduction
  complète, lien VO), panneau brief (dernier + bouton Générer + archives)
- **Dashboard** : panneau news existant → extrait (5 dernières) avec lien vers
  la vue

## Isolation & dégradation (gate 4)

- `kill -9` du collector → **l'app ne s'aperçoit de rien** (la bibliothèque
  cesse de se remplir, c'est tout) — le collector est hors watchdog de run.sh
- Un cycle qui panique = loggé et sauté (catch par cycle) ; un flux RSS down =
  skip + log (existant)
- Ollama down = traductions en attente, bouton brief répond « indisponible »
  proprement — jamais bloquant pour la bibliothèque (titres VO + filtres)

## Tests

- Attribution thème/assets par mots-clés (unitaires — la seule logique neuve du collecteur)
- Insertion + dédoublonnage presse (db, base mémoire)
- Rétention presse (db)
- Sélection top-24 h du brief (unitaire, déterministe, sans réseau)
- Machine à états `statut_traduction` + suppression après 2 échecs (unitaire)
- Les chemins IA sont court-circuités par le cache — aucun test ne dépend d'Ollama

## Hors périmètre (noté, pas construit)

- **Filtres de trading consommant la presse** (3e préférence) : post-gates 2-3,
  via les plugins stratégies — la table `presse_articles` sera prête
- **Alertes temps réel** (4e préférence) : non construit
- Scheduling du brief : volontairement absent (décision : à la demande)
