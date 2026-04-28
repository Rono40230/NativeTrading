# 🗺️ ROADMAP : Implémentation du Moteur Centralisé de Trade Management

Ce document détaille le plan d'action étape par étape pour remplacer les gestions de trades isolées par un **Moteur de Trade Management Universel** au sein du backend Rust.

## 🎯 Objectifs du Moteur
- **Standardisation stricte** : Niveau Initial ➔ TP1 (SL au BE) ➔ TP2 (SL au TP1 + Trailing sur Peak - ATR).
- **Options paramétrables** : Vente partielle (TP1%, TP2%, Trailing%) ou aucune vente partielle (100% trailing).
- **Mesure universelle** : Tout trade se clôture par un calcul natif de sa performance exprimée en **Multiple de Risque (R)**.
- **Agnosticisme** : Fonctionne de manière identique pour Straddle (Double direction), SMC (Pyramide) et Rockets (Momentum/ML).

## 🛡️ Principes d'Architecture & Précautions
- **Zéro conflit ("Delete & Replace")** : Ne pas superposer le nouveau système et les anciens. Lors de la phase de migration, tout l'ancien code de suivi (SL, BE, TP partiels) éparpillé dans les stratégies (`smc_feedback_job`, `straddle_moniteur_position`, `rockets_suivi`) sera intégralement supprimé. Les stratégies deviendront de simples relais interrogeant le Moteur Centralisé : une seule source de vérité mathématique.
- **Synchronisation absolue avec le Frontend** : Le verdict final (statut précis de la clôture et résultat exprimé en multiple de "R") calculé par le Moteur Centralisé sera inscrit de manière immuable en base de données. Le frontend Vue.js ne refera plus aucun calcul : il se contentera de lire la donnée exacte actée par le composant central, garantissant qu'il n'y ait aucun décalage d'affichage.

---

## 🟥 ÉTAPE 1 : Base de données & Paramètres (Priorité 1)
*Préparer le socle pour stocker les nouveaux paramètres choisis par l'utilisateur dans l'UI.*

1. **Migration SQLite (`backend/crates/db/migrations/`)**
   - Ajouter de nouvelles colonnes aux tables `straddle_config`, `smc_config`, `rockets_config`.
   - Colonnes à créer : 
     - `vente_partielle_active` (BOOLEAN)
     - `pct_cloture_tp1` (REAL, ex: 0.33)
     - `pct_cloture_tp2` (REAL, ex: 0.33)
2. **Mise à jour des Modèles Rust (`backend/crates/db/src/`)**
   - Mettre à jour les structs de configuration pour refléter ces tables.
   - Injecter des valeurs par défaut saines (ex: 33% / 33% / 34%).

## 🟧 ÉTAPE 2 : Refonte du Cœur Mathématique (Priorité 2)
*Transformer le fichier `backend/crates/strategies/src/position_tracking.rs` en un véritable moteur autonome.*

1. **Intégration du Concept de Risque ("R")**
   - La struct `PositionConfig` doit inclure le prix d'entrée et le SL initial pour calculer `risque_unitaire = abs(prix_entree - stop_loss)`.
2. **Refonte de l'Enum `Verdict`**
   - Modifier les retours pour inclure le résultat de l'opération en `R`.
   - `Tp1Partiel { r_encaisse: f64, pct_vendu: f64 }`
   - `ClotureTotale { label: String, r_final: f64 }`
3. **Application de la règle implacable de Trailing**
   - Coder la logique stricte demandée : 
     - *Phase 1* : Peak < TP1 ➔ SL = Initial
     - *Phase 2* : Peak ≥ TP1 ➔ SL = Break-Even
     - *Phase 3* : Peak ≥ TP2 ➔ SL = TP1 ET activation Trailing (Peak - ATR × coeff).
4. **Gestion du `flip()` natif**
   - Transférer la logique d'inversion mathématique pour les trades SHORT (actuellement isolée dans Straddle) directement dans le moteur pour qu'il soit agnostique au sens du trade.

## 🟨 ÉTAPE 3 : Migration des Stratégies (Priorité 3)
*Brancher les stratégies sur le nouveau moteur et nettoyer leur code mort.*

1. **SMC (`smc_feedback_job.rs` & `smc_boucle.rs`)**
   - **Action** : Arracher la boucle manuelle de calcul des TP. Brancher les signaux SMC au moteur `position_tracking`.
2. **Straddle (`straddle_moniteur_position.rs`)**
   - **Action** : Nettoyer l'outil de calcul de "pnl_r" manuel. Exploiter le "R" craché nativement par le `Verdict` du nouveau moteur. Maintenir la logique de Coupe-Circuit asynchrone (clôture de la jambe #2 si la jambe #1 atteint le TP1).
3. **Rockets (`rockets_position.rs` & `rockets_suivi.rs`)**
   - **Action** : Permettre un "override". Laisser le pipeline ML calculer un pourcentage dynamique (qui écrase le paramétrage UI) si le Score de l'entrée est explosif, puis l'envoyer au moteur centralisé.

## 🟩 ÉTAPE 4 : Refonte de l'interface Vue.js (Priorité 4)
*Permettre à l'utilisateur de piloter ce moteur universel.*

1. **Composant Paramétrages (`frontend/src/components/common/`)**
   - Remplacer le système "Option 1 — Partielle 1/3" désuet.
   - Ajouter un *Switch* "Activer la prise de profit partielle".
   - Si activé, afficher deux champs conditionnels : `% Vente au TP1` et `% Vente au TP2`.
2. **Gestion API Frontend (`frontend/src/services/api.service.ts`)**
   - S'assurer que les payloads envoyés aux endpoints `/api/config/*` incluent ces nouveaux paramètres.

## 🟦 ÉTAPE 5 : Apprentissage ML et Alignement LLM (Priorité 5)
*Exploiter ce nouveau rendement en "R" pour booster l'Intelligence Artificielle.*

1. **Nettoyage des Prompts LLM (`/Definitions & prompt IA`)**
   - Purger les instructions où l'on demande au LLM d'imaginer la prise de profit. 
   - Restreindre la tâche du LLM à l'analyse graphique pure (contexte fonctionnel, OB, FVG, invalidation) + fournir des niveaux de prix stricts.
2. **Feedback ML Standardisé**
   - Tous les jobs de feedback enverront en base le "R" retourné par le moteur de trade.
   - Le dataset d'entraînement local (XGBoost / LSTM) apprendra mathématiquement que tel setup graphique = `+2.5R` ou `-1.0R`, uniformisant la récompense d'apprentissage sur tout le système.

---

### Règles de sécurité Vibe Coding à maintenir
- Chaque étape doit passer l'audit de taille (`< 300 lignes`). Si le `position_tracking.rs` grossit trop, l'éclater en `position_maths.rs` et `position_verdict.rs`.
- Ne jamais utiliser `.unwrap()` lors de la manipulation des trades en direct dans le nouveau moteur.