# Objectifs & Roadmap — Native Trading AI

> Document de fondation : **ce que l'on cherche à atteindre** (le « quoi »), la
> **séquence de phases** (le « quand »), et les **règles de fonctionnement**.
> Le « comment » détaillé de chaque phase fera l'objet d'un plan d'exécution séparé.
>
> Date : 2026-08-11 · Statut : brouillon à valider. · Auteur : propriétaire + assistant.
> Sources : vision du propriétaire + audits code (stratégies/ML, architecture,
> cohérence frontend) + analyse du projet SMC de référence + Vibe Framework.

---

## 0. Règles de fonctionnement *(non-négociables)*

Ces règles s'imposent à toute phase, correction ou ajout.

| Règle | Pratique |
|-------|----------|
| **Vibe-coding assisté** | Le propriétaire n'est pas codeur. L'assistant est **responsable** de vérifier qu'aucune correction ne casse ou régresse le code existant. |
| **Test avant validation** | Aucune évolution n'est validée sans que les tests passent (`cargo test --workspace`, `npm run test`, `.vibe/bin/audit.sh`). C'est le standard Vibe Phase 2 (VALIDATION). |
| **Push contrôlé par le propriétaire** | L'assistant **ne pousse jamais** sur GitHub. Seul le propriétaire décide du push (Vibe Phase 3 COMMIT). |
| **Adoption des règles Vibe** | `.clinerules` + `.vibe/config.toml` font loi : naming métier français, **zéro panic** (interdiction `unwrap()`/`panic!()`/`console.log()`), architecture DAG. |
| **Hiérarchie de lecture** | `.clinerules` > `projet.md` > `.vibe/config.toml` > `docs/`. À lire avant toute implémentation. |
| **Honnêteté** | Si l'assistant n'est pas sûr d'un résultat, il le dit. Aucune approximation présentée comme validée. |

> ✅ **D0 tranché** : la règle Vibe « 300 lignes max/fichier » est **assouplie à ~600 lignes** sur la logique métier cohérente (dur conservé pour handlers/routes). L'audit avait prouvé que 300 lignes causait une fragmentation illisible (16 fichiers pour le pipeline Straddle). Voir §3.

---

## 1. Vision (objectif maître)

> **Une plateforme personnelle à triple mission :**

1. **Veille + signaux + alertes** — surveiller forex/cryptos/actions, faire tourner
   3 moteurs de signaux, alerter (mail + Telegram) sur les setups pertinents.
2. **Revue de presse ciblée** — synthèse d'actualité livrée à l'utilisateur.
3. **Système auto-apprenant** — les **résultats des signaux alimentent l'IA** (LLM/ML)
   pour qu'elle apprenne de ses erreurs et **adapte les paramètres des stratégies**
   afin d'en augmenter l'efficacité.

L'utilisateur **reçoit** l'information et **décide/exécute** lui-même sur son broker.
L'app n'exécute aucun trade.

> La mission 3 est **fondamentale** : l'app n'est pas qu'un système de veille, c'est
> un système qui s'améliore en boucle. Le **journal de trading** (§5, phase 7) en
> est la source de vérité (résultats réels des trades manuels).

---

## 2. Cœur de métier

| Brique | Description |
|--------|-------------|
| **Veille multi-actifs** | Forex + cryptos + actions, surveillance continue |
| **3 moteurs de signaux** | ① SMC/ICT · ② Rockets (VCP/Minervini) · ③ Volatilité (Straddle) |
| **Alertes** | Mail + Telegram, avec anti-bruit (sélection des signaux dignes d'alerte) + type/contenu à définir (D5) |
| **Revue de presse** | Synthèse ciblée (ciblage à définir — D4) |
| **Boucle d'apprentissage** | Résultats → feedback → LLM/ML adapte les paramètres stratégies |
| **Journal de trading** | Saisie des trades manuels + résultats = données d'entraînement de l'IA |

---

## 3. Décisions fondatrices *(tranchées le 2026-08-11)*

Ces décisions conditionnent le contenu de la phase 1. **Toutes tranchées par le
propriétaire.**

### ✅ D0 — Règle « 300 lignes » (Vibe) → **assouplie à 600**
La limite Vibe passe à **~600 lignes** sur la logique métier cohérente. Le dur est
conservé pour handlers/routes. *Raison : l'audit a prouvé que 300 lignes causait une
fragmentation illisible (16 fichiers pour le pipeline Straddle).*

### ✅ D1 — Backtesting → **supprimé, modèle « backtest externe »**
**Toute trace de backtest est retirée de l'app** en phase 1 (section UI + crate
`backtest` + routes + adapter). L'app = veille + signaux + alertes + apprentissage.
Une stratégie se backteste **ailleurs** (TradingView, MT5, Python) **avant** son
intégration. Un backtest honnête in-app pourrait revenir en phase 9, sur décision.

### ✅ D2 — Provider de données → **swap direct vers CPG en phase 1**
**IBKR Client Portal Gateway devient le provider unique.** Faisabilité confirmée :

| Classe | Couverture CPG |
|--------|----------------|
| Forex / méta | ✅ IBKR = broker forex majeur (IdealPro), données REST + streaming |
| Actions | ✅ Cœur de métier IBKR, 170+ marchés, global |
| Cryptos | ✅ **BTC + ETH (+ 9 autres) disponibles en EEA depuis le 31/03/2026** (IBKR Ireland + Zero Hash). France = éligible. |

**Caveats opérationnels à gérer en phase 1** (le propriétaire a déjà intégré CPG par le passé) :
- **Abonnements market data** : les flux temps-réel IBKR sont **payants par bourse/feed**. Sans abonnement → données différées (15 min). À vérifier sur le compte IBKR.
- **Disponibilité gateway 24/7** : la session CPG expire (~quotidien) → il faut un gateway qui se relance/re-auth automatiquement pour la veille continue.
- **Permissions crypto** : activer la permission « Cryptocurrencies » dans le Client Portal IBKR + l'entité compte doit être IBKR Ireland (EEA).
- **Mapping symboles** : IBKR crypto = `BTC/USD` (pas `BTCUSDT` comme Binance) ; prévoir la table de mapping par actif.

**Sécurisation du swap (gestion du risque régression)** — phase 1 en deux sous-étapes :
- **1a** : abstraction `DataProvider` propre + connecteur CPG **ajouté en parallèle** des sources existantes. Vérifier que CPG délivre des données correctes pour les 3 classes + que les 3 stratégies consomment correctement.
- **1b** : une fois CPG prouvé correct → **supprimer** Binance + IG + MT5 (et la duplication `data/`↔`api/`).

### ✅ D3 — Moteur SMC → **BSZones**
Calquer le moteur SMC sur **BSZones** (ICT 2022 canonique : Sweep→Displacement→OB,
score /10 étoilé) en récupérant la **calibration par actif** de v11 (tables de
seuils/pondérations). Référence : `BSZones.pine` du projet
`/home/rono/Applis Nono/Indicateur Scalp à Nono/`.

---

## 4. État des lieux (diagnostic condensé)

### Solide — conserver
ML réel (LSTM cuDNN + XGBoost, fallback GPU→CPU, ML gate actif). Logique de marché
réelle (SMC 5 composantes, Rockets VCP). Discipline d'erreur saine (quasi zéro
`unwrap`/`panic` en chemin critique). Frontend : aucune vue vide, aucun mock.

### Bloquant pour la confiance — corriger
1. Module risk fantôme (jamais importé). 2. Backtest sans frais + ≠ live
(cf. D1). 3. 3 implémentations par stratégie qui dérivent. 4. Démarrage double des
boucles SMC/Straddle. 5. Fuite mémoire `String::leak()`. 6. Diagnostic ML trompeur
(`est_pret()` ignore LSTM). 7. Navigation cassée. 8. ts-rs partiel + drift.

### Dette structurelle
Monolithe `api` (131 fichiers). LLM réimplémenté 4-5× + 27 `reqwest::Client` +
sémaphore Ollama contourné. Tests quasi absents (`api`, `common`, `data`).

### Lacunes vs vision
❌ Alerte mail. ❌ Revue de presse/digest. ❌ Actions. ❌ Journal de trading.
⚠️ Fuseaux horaires incohérents. ⚠️ Boucle d'apprentissage partielle (infra
feedback/calibration existe mais non exploitée comme objectif).

---

## 5. Phases de développement

L'ordre 1-6 est celui du propriétaire. Les phases 7-10 sont les ajouts rendus
nécessaires par la vision enrichie et l'audit.

### Phase 1 — Assainissement & bases solides
**Objectif : repartir d'un code honnête, sans mort ni bug critique.**
- Supprimer le code mort (`NavBar.vue`, ~9 routes backend mortes, fonctions dev mortes).
- Corriger les bugs critiques : double-start boucles, `String::leak()`, params SMC ignorés.
- Démonolithe `api` : extraire `llm`, `news`, `notifications` dans des crates.
- Mutualiser : 1 `reqwest::Client`, types Ollama uniques, sémaphore respecté partout.
- Unifier couche HTTP frontend (plus de `localhost` hardcodé) + corriger drift ts-rs.
- **[D1]** Supprimer toute la section backtest (UI + crate `backtest` + routes + adapter).
- **[D2]** Swap vers CPG en deux sous-étapes : (1a) abstraction `DataProvider` + connecteur CPG en parallèle, vérification données 3 classes ; (1b) suppression Binance + IG + MT5 et de la duplication `data/`↔`api/`.
- Poser des tests minimaux sur `api`/`common`/`data`.
- **[D0]** Appliquer la nouvelle limite ~600 lignes (regroupement par responsabilité).
**Critère « terminé » :** 0 lien mort, 0 route morte, 0 bug critique connu, 1 client
HTTP, sémaphore respecté, métier testable, tests verts, CPG opérationnel (3 classes).

### Phase 2 — Alignement des stratégies *(refondu selon la vraie définition)*
**Objectif : les 3 moteurs alignés sur leurs références et affinés par classe d'actif.**
- **SMC/ICT** [D3] : calquer sur **BSZones** (ICT 2022 canonique : Sweep→Displacement→OB, score /10 étoilé) du projet de référence `/home/rono/Applis Nono/Indicateur Scalp à Nono/`, en récupérant la **calibration par actif** de v11, Kill Zones UTC, lifecycle SL/TP, modèle d'exécution par retest. *(Enrichissement majeur : le SMC Rust passe de 5 composantes au moteur canonique.)*
- **Rockets** : **rechercher** la méthode VCP/Minervini pour **affiner les critères par classe d'actif** (crypto vs indices vs forex).
- **Volatilité (Straddle)** : **rechercher** pour **affiner les critères par classe d'actif**.
- Réunifier les implémentations (live = une seule source de vérité par stratégie).
- Wirer le module `risk`.
- ML honnête (`est_pret()` + walk-forward sur LSTM).
**Critère « terminé » :** chaque stratégie alignée sur sa référence/critères affinés
par actif ; risk actif ; ML diagnostiqué honnêtement.

### Phase 3 — Prompts IA
**Objectif : chaque prompt a un but clair et une qualité mesurable.**
- Inventaire exhaustif (`/api/prompts` + `PromptsIAView`).
- Audit + réécriture par prompt (Straddle, SMC, Rockets, Coach, analyse chart).
- A/B testing (`ab_test_handlers` existe).
- Routing via le client/sémaphore mutualisé (phase 1).
**Critère :** chaque prompt documenté + A/B sur les prompts à fort impact.

### Phase 4 — Revue de presse
**Objectif : une synthèse d'actualité ciblée et livrée.**
- **Définir le ciblage (D4)** : sujets, sources, langues, fréquence, format.
- Construire le digest sur l'infra news existante (scraping, RSS, scoring, traduction, sentiment, fear-greed).
- Livraison via alertes (phase 6).
**Critère :** revue générée + livrée selon ciblage.

### Phase 5 — Fuseaux horaires
**Objectif : cohérence TZ bout-en-bout.**
- Unifier : stockage UTC, affichage heure de Paris (TZ utilisateur de référence), tests DST.
- Cas spécifique : indicateur range Asie affiché en heure de Paris (cf. `A_faire.txt`).
**Critère :** une seule convention TZ + tests couvrant les transitions DST.

### Phase 6 — Sélection des signaux & alertes
**Objectif : alerter seulement sur ce qui le mérite, via mail + Telegram.**
- **Système de sélection** : scoring/filtrage anti-bruit, déduplication multi-TF.
- **Définir (D5)** : type d'alerte (signal neuf, approche seuil, cloture TP/SL, revue de presse…) et **contenu** (actif, TF, score, niveaux SL/TP, contexte, screenshot chart…).
- **Alerte mail** (NOUVEAU — choix transport SMTP, D6).
- **Alerte Telegram** (existant — améliorer).
**Critère :** mail + Telegram fonctionnels ; précision d'alerte mesurée ; anti-spam effectif.

### Phase 7 — Journal de trading + boucle d'apprentissage  ***[ajout — mission fondamentale]***
**Objectif : l'app apprend des résultats réels pour améliorer les stratégies.**
- **Journal de trading** in-app : saisie des trades manuels (entrées, SL/TP réels, sortie, notes), liés aux signaux qui les ont générés.
- **Fermer la boucle** : résultats du journal → labels de vérité → feedback → LLM/ML **adapte les paramètres** des stratégies (exploiter `*_calibration_job`, `*_feedback_job` existants).
- Tableaux de bord : taux de réussite par stratégie/actif/TF, drift de performance.
**Critère :** journal fonctionnel ; boucle résultat→ajustement de paramètres active et mesurable.

### Phase 8 — Extension des actifs  ***[ajout]***
**Objectif : couvrir forex + cryptos + actions.**
- **Selon D2** : basculer vers CPG (ou provider retenu) comme provider unifié.
- Forex complet, cryptos, **actions**.
- Tableau de veille multi-classes.
**Critère :** forex + cryptos + actions surveillés et générant des signaux.

### Phase 9 — Validation & edge  ***[ajout — conditionnel après phase 2]***
**Objectif : mesurer honnêtement la rentabilité, décider.**
- Backtest forward honnête (walk-forward OOS, avec coûts) — **rapport externe** (MT5/TradingView/Python), le backtest ayant été retiré de l'app (D1).
- Décision documentée par moteur : garder / optimiser / abandonner.
**Critère :** fiche de décision argumentée par stratégie.

### Phase 10 — Constructeur de stratégies  ***[ajout — phase finale, exploratoire]***
**Objectif : permettre de construire de nouvelles stratégies génératrices de signaux.**
- Système permettant d'ajouter une stratégie sans tout réécrire (plugins ? DSL de
  scoring ? template de moteur ?).
- **Inconnue méthodologique** : le propriétaire ne sait pas comment aborder cela.
  Cette phase fera l'objet d'un **brainstorming dédié** quand elle arrivera.
**Critère :** une stratégie « exemple » créée via le système, du signal à l'alerte.

---

## 6. Réorganisation de l'interface (sidebar + pages)

À réaliser en phase 1 (assainissement nav) puis enrichie au fil des phases.

**Constat (audit frontend) :** 1 lien sidebar cassé (`/smc/analyser`), `NavBar.vue`
mort, 6 routes orphelines (3 vues Définition + Prompts IA + ML insights + signaux
Straddle), 1 lien interne cassé (`/pnl`).

**Pages manquantes (vs vision) :**
- 📓 **Journal de trading** (phase 7) — saisie + historique + lien signaux.
- 📰 **Revue de presse** (phase 4).
- 🔔 **Centre d'alertes** (phase 6) — config + journal d'alertes envoyées.

**Pages à supprimer/merged :**
- `NavBar.vue` (mort). Les 3 vues « Définition » → intégrer comme sous-onglets de
  leur stratégie (plus de routes orphelines).

*Proposition de sidebar restructurée à finaliser en phase 1.*

---

## 7. Principes directeurs (transverses)

1. **Confiance avant edge.** On fiabilise (phases 1-2) avant de mesurer (phase 9).
2. **Vrai = simple.** Réunifier plutôt qu'aligner des implémentations divergentes.
3. **Honnêteté de l'IA.** Si le LSTM n'est prêt, l'app le dit. Si l'app apprend, elle
   le montre.
4. **Ne pas régresser la discipline** d'erreur existante (zéro panic).
5. **Le bruit tue la veille.** Une alerte n'a de valeur que rare et juste.
6. **L'apprend du réel.** Le journal de trading est la source de vérité de l'IA.

---

## 8. Non-objectifs

- ❌ Exécution automatique sur broker réel.
- ❌ Distribution multi-utilisateur / SaaS.
- ❌ Nouvelles stratégies **avant** que les 3 actuelles ne soient alignées (phase 10 les enable).
- ❌ Refonte graphique complète (on corrige la cohérence, on ne redesign pas).
- ❌ Promesse de rentabilité (l'edge est mesuré honnêtement, jamais garanti).

---

## 9. Arbitrages ouverts

| # | Décision | Statut | Phase |
|---|----------|--------|-------|
| D0 | Règle « 300 lignes » | ✅ **Assouplie à 600** | 1 |
| D1 | Backtest | ✅ **Supprimé (modèle externe)** | 1 |
| D2 | Provider de données | ✅ **Swap direct CPG en phase 1** (2 sous-étapes) | 1 |
| D3 | Moteur SMC | ✅ **BSZones** (+ calibration v11) | 2 |
| D4 | Ciblage revue de presse (sujets/sources/langues/fréquence/format) | ⬜ Ouvert | 4 |
| D5 | Type + contenu des alertes | ⬜ Ouvert | 6 |
| D6 | Transport mail (SMTP perso Gmail / service tiers) | ⬜ Ouvert | 6 |
| ~~D7~~ | ~~Provider d'actions si CPG écarté~~ | ⛔ *Sans objet* (CPG adopté en D2) | — |
| D8 | Garder les 3 stratégies ? | ⬜ Ouvert | 9 |

---

## 10. Prochaines étapes

1. **D0-D3 tranchées** (2026-08-11). Document de fondation finalisé.
2. **Committer** ce document (décision appartenant au propriétaire).
3. **Détailler la phase 1** en plan d'exécution concret (skill `writing-plans`).
4. Exécuter phase par phase, revue + tests entre phases.
