# 📋 Améliorations Stratégie Straddle — Plan de travail

> Document de travail — à compléter au fil des itérations

---

## État actuel (17 avril 2026)

### Ce qui fonctionne ✅

| Composant | Fichier | Détail |
|---|---|---|
| Surveillance ATR | `straddle_boucle.rs` | Toutes les 15min, tous assets actifs (actif=1 en DB) |
| Scan pics | `straddle_scan_pics.rs` | Toutes les 5min, seuil détection 1.3×, stockage en DB |
| Catégorisation | `straddle_categorisation.rs` | 8 catégories : annonce_high, annonce_medium, overlap_lnd_ny, ny_open, london_open, tokyo_open, creneau_recurrent, choc_isole |
| Filtrage annonces | `straddle_boucle.rs` l.120–143 | Fenêtre ±90min, High Impact uniquement, depuis le cache calendrier DB |
| Calibration seuils | `straddle_calibration.rs` | Seuil ATR + score LLM calibrés par (asset, categorie) depuis les feedbacks |
| Invalidation categories | `straddle_calibration.rs` | Catégorie WR < 50% → flag `invalide` → skip automatique |
| Contexte LLM | `straddle_boucle.rs` + `straddle_signal_handler.rs` | Prix, ATR, session, kill zone, annonces, créneaux historiques, feedbacks few-shot |
| Calcul SL/TP | `straddle_signal_handler.rs` l.232–242 | SL=0.5×ATR, TP1=2×, TP2=3.5×, TP3=5× (fixes) |
| Snapshot features | `straddle_features.rs` + migration 0053 | Sauvegarde features au moment du signal pour fine-tuning |
| Fine-tuning ML | `straddle_trainer.rs` + `ml_retrain_job.rs` | XGBoost Straddle, guard 50 samples, split 80/20 |
| Gate ML | `straddle_boucle.rs` | Si ML très confiant directionnel → skip Straddle |
| Anti-doublon | Boucle + Scan | 90min boucle, 30min scan |
| Corrélation whipsaw | `straddle_heatmap` | Champ `whipsaw_minutes` calculé en backtest |

### Ce qui manque ❌

- Modèle `modele_xgboost_straddle.json` non généré (0 trades clôturés à ce stade)
- SL/TP non calibrés par catégorie (valeurs fixes)
- Pas de clôture automatique de la jambe opposée au TP touché
- `whipsaw_minutes` non utilisé en temps réel
- Pas de filtre de corrélation inter-assets (BTC + ETH peuvent émettre simultanément)

---

## 🔴 P1 — Gestion dynamique de position (SL progressif + trailing) — Module commun

**Priorité : HAUTE — Impact direct sur le risk/reward, s'applique aussi à SMC**

### Problème
La gestion de position Straddle est actuellement statique : SL/TP fixes calculés à l'émission du signal, jamais mis à jour. Il n'y a pas de :
- Remontée du SL vers le BE quand TP1 est touché
- Remontée du SL vers TP1 quand TP2 est touché
- Trailing stop actif après TP2 (peak − ATR × coeff)
- Vente partielle par palier (TP1 = sortie partielle, pas fermeture totale)

La stratégie Rockets dispose déjà de cette logique (`rockets_niveaux.rs`, `rockets_position.rs`) mais elle ne peut pas être réutilisée directement depuis Straddle : violation du DAG (pas d'import horizontal entre stratégies) et logique métier distincte (2 jambes, paramètres différents). La même mécanique sera également nécessaire pour **SMC**.

### Décision d'architecture
Extraire un **module commun de suivi de position** dans le crate `strategies` :

```
backend/crates/strategies/src/position_tracking.rs
```

Ce module expose une logique générique paramétrée :
- `PeakTracker` : suivi du prix maximum atteint sur la jambe
- `calculer_verdict(prix, peak, config)` → `Verdict` : `TP1` (partiel) | `TP2` (partiel) | `TrailingTouche` (clôture) | `SL` / `BE` / `TP1` / `TP2` (clôture progressive)
- `PositionTrackingConfig` : `sl_ratio`, `tp1_ratio`, `tp2_ratio`, `trailing_coeff_min/max`, **`vente_partielle: bool`**, `split_tp1/tp2/trailing`

Chaque stratégie passe sa propre config :
- **Rockets** → refactorisé pour déléguer à `position_tracking` (pas de régression fonctionnelle)
- **Straddle** → config spécifique par catégorie (annonce_high ≠ london_open)
- **SMC** → config spécifique (à définir lors de P13 SMC)

### Vente partielle : option configurable (toutes stratégies)

Le flag `vente_partielle: bool` dans `PositionTrackingConfig` contrôle le comportement aux paliers TP :

| Mode | TP1 touché | TP2 touché | TP3/Trailing |
|---|---|---|---|
| `false` (pas de vente partielle) | SL → BE uniquement | SL → TP1 uniquement | Clôture totale |
| `true` (vente partielle) | Vente X% + SL → BE | Vente Y% + SL → TP1 + trailing actif | Clôture du solde |

Ce flag est configurable dans **Configuration → Paramètres des stratégies** pour chaque stratégie indépendamment.

> **Note Rockets** : le flag `vente_partielle` existe déjà dans `RocketsConfig` (`rockets_config.rs` l.15) mais **n'est pas vérifié** dans `rockets_suivi.rs` — la vente partielle s'applique toujours. C'est un bug latent à corriger lors du refactoring (Option A).

### Logique de la jambe Straddle survivante
1. Signal Straddle émis → 2 jambes (LONG + SHORT) avec leurs entrées/SL/TP initiaux
2. Prix touche TP1 jambe gagnante → (si `vente_partielle`) vente partielle, sinon rien ; SL remonte au BE ; **jambe perdante clôturée** (cf. P2)
3. Prix touche TP2 → (si `vente_partielle`) vente partielle, sinon rien ; SL → TP1 ; trailing stop activé sur le reste
4. Trailing touché → clôture le solde avec gain maximal
5. SL touché à tout instant → clôture la jambe

### Ratios initiaux par catégorie (à calibrer depuis feedbacks)
Les ratios de départ au moment du signal restent dans `straddle_calibration` :

```
sl_ratio        REAL DEFAULT 0.5
tp1_ratio       REAL DEFAULT 2.0
tp2_ratio       REAL DEFAULT 3.5
trailing_coeff  REAL DEFAULT 2.0
```

Un pic `annonce_high` (amplitude forte, courte durée) aura des ratios différents d'un `london_open` (move progressif, plus long). Ces valeurs sont calculées depuis les feedbacks clôturés, comme `atr_seuil` l'est déjà.

### Fichiers impactés
- **Nouveau** : `backend/crates/strategies/src/position_tracking.rs` → module commun
- `backend/crates/strategies/src/rockets_niveaux.rs` → refactorisé pour déléguer (sans régression)
- `backend/crates/strategies/src/rockets_position.rs` → idem
- `backend/crates/db/migrations/` → migration ALTER TABLE `straddle_calibration` (colonnes ratios)
- `backend/crates/db/src/straddle_calibration.rs` → `SeuilsEffectifs` + `CalibrationRow` étendus
- `backend/crates/api/src/straddle_calibration.rs` → calcul ratios depuis feedbacks
- `backend/crates/api/src/straddle_signal_handler.rs` → utiliser ratios calibrés à l'émission
- `backend/crates/api/src/straddle_boucle.rs` → idem pour signaux auto
- **Futur SMC** : `backend/crates/api/src/smc_boucle.rs` → utilisera le même module

### Stratégie de refactoring Rockets : Option A (API inchangée)

Deux approches possibles :

- **Option A** : `rockets_niveaux.rs` délègue **en interne** à `position_tracking.rs` mais conserve la même signature publique `calculer_verdict_rocket(...)`. `rockets_suivi.rs` **ne change pas une ligne**. Les tests existants (`rockets_suivi_tests.rs`) continuent de valider exactement les mêmes comportements.
- **Option B** : `rockets_suivi.rs` appelle directement `position_tracking` avec une `PositionTrackingConfig` Rockets. Plus propre à terme mais risque de régression élevé.

**Décision : Option A**. Le refactoring est interne au crate `strategies`. Straddle et SMC appellent directement `position_tracking` sans passer par `rockets_niveaux`. On unifie la logique sans perturber Rockets.

### Notes / Décisions
- Refactoring Rockets = Option A : API publique inchangée, délégation interne uniquement
- Corriger le bug `vente_partielle` non vérifié dans `rockets_suivi.rs` lors du refactoring
- Ne pas toucher à `rockets_niveaux.rs` avant validation des tests existants (`rockets_suivi_tests.rs`)
- Le refactoring Rockets doit être transparent : mêmes verdicts, mêmes comportements garantis par les tests

### ✅ Décisions actées (sprint P1+P2)

**Ordre d'implémentation (sprint séquentiel, pas de dette technique) :**
1. Créer `position_tracking.rs` + tests unitaires complets
2. Refactoring Rockets (Option A) → valide le module sur du code en production
3. Intégrer le module dans le moniteur Straddle (P2)
4. Calibration des ratios par catégorie

**Scope du sprint :** P1 et P2 dans le même sprint. Le refactoring Rockets (step 2) sert de validation réelle du module avant de l'utiliser dans Straddle — plus sûr qu'une validation théorique.

---

## 🔴 P2 — Moniteur de suivi de position en temps réel (peak tracking + clôtures)

**Priorité : HAUTE — Nécessaire pour que P1 soit effectif**

### Problème
P1 définit la logique de gestion dynamique (SL progressif, trailing, vente partielle). Mais cette logique ne peut s'exécuter que si un processus surveille en continu le prix des jambes actives pour détecter les franchissements (TP1, TP2, trailing touché, SL remonté touché).

Aujourd'hui, une fois le signal Straddle émis, personne ne surveille le prix. Les jambes restent `Actif` indéfiniment.

### Solution envisagée
Un moniteur de suivi (tâche tokio, cycle ~1 min) qui, pour chaque signal Straddle `statut=Actif` :

1. Récupère le prix actuel de l'asset
2. Charge `peak` depuis la DB (initialisé à `prix_entree` à l'émission, mis à jour à chaque cycle)
3. Appelle `position_tracking::calculer_verdict(prix, peak, config)` pour chaque jambe
4. Selon le verdict :
   - `TP1` → enregistre la vente partielle, remonte SL au BE, **clôture immédiatement la jambe opposée**
   - `TP2` → vente partielle, SL → TP1, trailing activé
   - `TrailingTouche` → clôture le solde de la jambe
   - `SL` / `BE` → clôture la jambe avec le label de résultat approprié
5. Met à jour `peak` en DB si `prix_actuel > peak`

### Stockage du peak
Une colonne `peak` (ou `prix_max_atteint`) doit être ajoutée sur la table `signaux` ou sur une table de suivi dédiée. La table `signaux` étant partagée entre stratégies, préférer une table `straddle_suivi_position` :

```sql
signal_id    INTEGER PK REFERENCES signaux(id)
jambe        TEXT  -- 'LONG' | 'SHORT'
peak         REAL
sl_effectif  REAL  -- mis à jour dynamiquement
statut_jambe TEXT  -- 'actif' | 'tp1_touche' | 'tp2_touche' | 'cloturee'
```

### Source du prix en temps réel

**✅ Résolu — aucun travail supplémentaire.**

`backend/crates/api/src/prix_utils.rs` expose `fetch_prix_asset()` : dispatcher unifié qui route automatiquement vers la bonne source selon le type d'asset :

| Asset | Source | Mécanisme |
|---|---|---|
| BTC, ETH | Binance | `fetch_binance()` |
| XAUUSD, XAGUSD, Forex, Indices | IG Markets | `fetch_ig()` → `GET /markets/{epic}` → **bid/ask snapshot temps réel** |

Le moniteur Straddle appelle simplement `fetch_prix_asset(asset, client, db)`. Aucune implémentation supplémentaire.

### P&L de la vente partielle → feedback ML

**Décision : un seul feedback agrégé à la clôture complète du trade.**

`straddle_suivi_position` trace chaque événement (TP1 touché à prix X, % vendu ; TP2 à prix Y, % vendu ; trailing à prix Z, % vendu). À la fermeture totale :

```
pnl_final = (pct_tp1 × r_tp1) + (pct_tp2 × r_tp2) + (pct_trailing × r_trailing)
```

→ **une seule entrée `straddle_feedback`** avec ce P&L pondéré. Cohérent avec Rockets (un verdict final par signal). Évite que le trainer ML voie 2–3 entrées pour le même signal avec des labels contradictoires.

`straddle_suivi_position` étendue :
```sql
signal_id       INTEGER PK REFERENCES signaux(id)
jambe           TEXT    -- 'LONG' | 'SHORT'
peak            REAL
sl_effectif     REAL    -- mis à jour dynamiquement
statut_jambe    TEXT    -- 'actif' | 'tp1_touche' | 'tp2_touche' | 'cloturee'
prix_tp1        REAL    -- prix au moment où TP1 a été touché
prix_tp2        REAL    -- prix au moment où TP2 a été touché
prix_cloture    REAL    -- prix final de clôture
pnl_r_final     REAL    -- P&L pondéré calculé à la clôture
```

### Fichiers impactés
- Source prix : `fetch_prix_asset()` dans `prix_utils.rs` — dispatcher unifié Binance/IG déjà implémenté ✅
- P&L : suivi palier par palier dans `straddle_suivi_position`, feedback agrégé unique dans `straddle_feedback` à la clôture totale

---

## 🟠 P3 — Délai d'entrée basé sur le whipsaw

**Priorité : MOYENNE — Améliore la qualité d'entrée**

### Problème
La table `straddle_heatmap` stocke `whipsaw_minutes` : la durée médiane pendant laquelle le marché fait un faux mouvement avant la vraie direction, calculée en backtest. Ce champ est ignoré en temps réel : la boucle émet le signal immédiatement dès que l'ATR pique.

### Solution envisagée
Pour les catégories avec `whipsaw_minutes > 0` (ex: `annonce_high` → souvent 2–5min de spike puis retour) :
- Ne pas émettre le signal immédiatement
- Mettre en file d'attente l'opportunité avec un timer `whipsaw_minutes` 
- Réévaluer l'ATR après ce délai → si encore > seuil, émettre le signal

### Fichiers impactés
- `backend/crates/api/src/straddle_boucle.rs` → délai conditionnel
- `backend/crates/db/src/straddle.rs` → accès `whipsaw_minutes` depuis heatmap
- Potentiellement un `PendingStraddle` en mémoire (HashMap<asset, tokio::JoinHandle>)

### Notes / Décisions
<!-- À compléter -->

---

## 🟠 P4 — Fallback structuré quand ML indisponible (< 50 samples)

**Priorité : MOYENNE — Robustesse hors-phase démarrage**

### Problème
Tant que `modele_xgboost_straddle.json` n'existe pas (< 50 trades clôturés), la décision repose à 100% sur Ollama. Le LLM seul est sujet à des hallucinations ou à des réponses inconstantes selon le modèle chargé.

Actuellement la gate ML est bypassée silencieusement (`xgb_straddle` = `None`).

### Solution envisagée
Score de confiance structuré sans ML basé sur des règles métier :
```
score = 0
+ 30 pts si categorie IN (annonce_high, overlap_lnd_ny)
+ 20 pts si créneau historique validé correspond à l'heure actuelle
+ 20 pts si kill zone active (SMC)
+ 15 pts si ratio_atr > 2.0× (pic fort)
+ 15 pts si session London ou NY
→ seuil : 60/100 pour autoriser l'appel Ollama
```

Ce fallback remplace la gate ML absente et réduit les appels Ollama sur des contextes faibles.

### Fichiers impactés
- `backend/crates/api/src/straddle_boucle.rs` → fallback avant appel Ollama
- Potentiellement un nouveau `straddle_score_regle.rs`

### Notes / Décisions
<!-- À compléter -->

---

## 🟡 P5 — Filtre de corrélation inter-assets

**Priorité : BASSE — Évite exposition doublée en crypto**

### Problème
BTC et ETH sont fortement corrélés (0.85+ en général). Lors d'une annonce macro (ex: CPI, FOMC), les deux piquent simultanément. La boucle Straddle peut émettre 2 signaux en même temps, doublant l'exposition réelle alors que la règle de risk limit est 2% par direction.

### Solution envisagée
Définir des groupes de corrélation (configurables) :
```toml
[correlation_groups]
crypto = ["BTC", "ETH"]
metaux = ["XAUUSD", "XAGUSD"]
```

Règle : si un signal Straddle est déjà `Actif` pour un asset du même groupe → skip ce cycle.

### Fichiers impactés
- `backend/crates/api/src/straddle_boucle.rs` → vérification avant analyse
- Config via paramètres DB ou fichier TOML

### Notes / Décisions
<!-- À compléter -->

---

## 📊 Récapitulatif

| # | Amélioration | Priorité | Complexité | Statut |
|---|---|---|---|---|
| P1 | Calibration SL/TP par catégorie | 🔴 Haute | Moyenne | ❌ À faire |
| P2 | Clôture automatique jambe opposée | 🔴 Haute | Haute | ❌ À faire |
| P3 | Délai entrée whipsaw | 🟠 Moyenne | Moyenne | ❌ À faire |
| P4 | Fallback ML structuré | 🟠 Moyenne | Faible | ❌ À faire |
| P5 | Filtre corrélation inter-assets | 🟡 Basse | Faible | ❌ À faire |
