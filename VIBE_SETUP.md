# ✅ Configuration Vibe Coding - Checklist d'Implémentation

**Date:** 10 mars 2026  
**Projet:** Native Trading AI  
**Status:** 🟢 Prêt pour développement Vibe Coding

---

## 📋 Fichiers Créés/Modifiés

### 1. ✅ `.clinerules` (Racine projet)
**Fichier:** `/run/media/rono/IA/native-trading-ai/.clinerules`  
**Contenu:** 20 règles absolues métier adaptées au trading AI  
**Sections principales:**
- Objectifs projet (Trading haute fréquence)
- Conventions nommage (métier français)
- Error handling (zero panic Rust)
- File size limits (300 lignes max)
- DAG Architecture
- ML Pipeline robustesse
- Risk Management non-négociable
- Performance targets (<200ms ML)

**Utilisation:** Agent IA DOIT lire ce fichier en priorité avant toute implémentation

---

### 2. ✅ `.vibe/config.toml`
**Fichier:** `/run/media/rono/IA/native-trading-ai/.vibe/config.toml`  
**Contenu:** Configuration technique Vibe Framework  
**Paramètres clés:**
- Stack: Rust + Vue.js
- Langage: Français (métier)
- Max file lines: 300
- Forbidden patterns: `unwrap()`, `console.log()`, `panic!()`
- Custom trading section (métriques, limites risk)

**Utilisation:** Sentinelle utilise cette config pour enforcement automatique

---

### 3. ✅ `projet.md` (Racine)
**Fichier:** `/run/media/rono/IA/native-trading-ai/projet.md`  
**Contenu:** Documentation complète contexte projet  
**Sections:**
- Vision & objectifs (ROI >15%, Sharpe >1.5)
- Architecture technique détaillée
- ML Pipeline (LSTM + XGBoost)
- Stratégies (Straddle, SMC Directionnel)
- Risk Management (limites strictes)
- Indicateurs SMC (5 priorités)
- Base de données (schema SQLite)
- Data sources (Binance, MT5)
- Workflow Vibe Coding (3 phases)
- Scripts utiles
- Priorités actuelles (Roadmap)

**Utilisation:** Source de vérité pour compréhension projet global

---

### 4. ✅ `.github/copilot-instructions.md`
**Fichier:** `/run/media/rono/IA/native-trading-ai/.github/copilot-instructions.md`  
**Contenu:** Instructions GitHub Copilot enrichies Vibe Coding  
**Structure:**
- Philosophie Vibe Coding (Contrat, Règles absolues)
- Architecture (Stack, Data Flow, Crates)
- Règles critiques (Error handling, File size, DAG, Naming)
- Workflow 3 phases (Création, Validation, Commit)
- Domaine trading (Assets, Stratégies, ML, Risk)
- Règles Frontend/Backend spécifiques
- Documentation & Références
- Métriques success
- Rappels finaux

**Utilisation:** GitHub Copilot lit automatiquement ce fichier pour contexte

---

## 🚦 Workflow Vibe Coding (Rappel)

### PHASE 1 : CRÉATION
**Quoi:** Coder librement, tester localement, itérer  
**Outils:** `.vibe/bin/sentinel.sh` (formatage auto)  
**Règle:** PAS de commit

### PHASE 2 : VALIDATION
**Déclencheur:** Utilisateur dit "Valide" ou "Ready"  
**Action:** Lancer `.vibe/bin/audit.sh`  
**Résultat:**
- 🟢 VERT → Phase 3
- 🔴 ROUGE → Corriger et relancer

### PHASE 3 : COMMIT
**Condition:** Audit ✅ uniquement  
**Action:** Impact detection + commit conventionnel

---

## 🎯 Règles d'Or à Retenir

### 1. Hiérarchie Lecture
```
.clinerules > projet.md > .vibe/config.toml > docs/
```
**Agent IA doit lire `.clinerules` AVANT toute implémentation**

### 2. Zero Panic (Critique)
```rust
// ❌ INTERDIT
.unwrap()
panic!()

// ✅ REQUIS
fn ma_fonction() -> Result<T, TradingError> {
    let data = obtenir()?;
    Ok(data)
}
```

### 3. File Size (<300 lignes)
- Fichier >250 lignes → SPLIT IMMÉDIAT
- Fonction >30 lignes → Extraire
- Complexité >10 → Simplifier

### 4. DAG Architecture
```
Commands (L4) → Services (L3) → Data (L2) → Models (L1)
```
**Jamais d'imports horizontaux entre services**

### 5. Naming Convention
**Métier = Français** : `calculer_volatilite()`, `detecter_signal()`  
**APIs = Anglais OK** : `parse()`, `serialize()`

---

## 🛠️ Scripts Disponibles

### Tests
```bash
cargo test --workspace
cd frontend && npm run test
./scripts/test.sh
```

### Audit/Validation
```bash
./.vibe/bin/audit.sh                          # Audit complet
./scripts/validate-phase2.sh                  # Validation Phase 2
./scripts/impact-detection/validate-phase2.sh # Impact detection
```

### Qualité Code
```bash
cargo clippy --workspace -- -D warnings       # Linter Rust
cargo fmt --all --check                       # Formateur Rust
./scripts/check-file-size.sh                  # Vérifier tailles
```

### Application
```bash
./scripts/install.sh                          # Installation
./scripts/run.sh                              # Lancer app
./scripts/backup.sh                           # Backup données
```

---

## 📝 Prochaines Étapes Développement

### Semaine 1-2 : Fondations
- [ ] Lancer Sentinelle : `.vibe/bin/sentinel.sh` (si scripts Vibe disponibles)
- [ ] Implémenter Binance WebSocket provider
- [ ] Créer schema SQLite + migrations
- [ ] Tests acquisition données

### Semaine 3 : ML Pipeline
- [ ] Feature extraction (OHLCV + indicateurs)
- [ ] Training XGBoost classification
- [ ] Validation latence <200ms

### Semaine 4 : Stratégie
- [ ] Implémenter Straddle (volatilité)
- [ ] Risk management basic
- [ ] Backtesting simulation

### Semaine 5-6 : Dashboard
- [ ] ChartView TradingView Lightweight Charts
- [ ] Pinia stores (signals, market data)
- [ ] WebSocket temps réel
- [ ] Tests intégration

---

## 🎓 Ressources Vibe Framework

**Framework installé dans:** `Vibe-Framework/`

**Documentation Vibe:**
- `Vibe-Framework/README.md` - Guide général
- `Vibe-Framework/.vibe/rules/system_prompt.md` - Prompt système
- `Vibe-Framework/.clinerules` - Template règles (copié à racine)

**Scripts Vibe (si disponibles):**
- `.vibe/bin/sentinel.sh` - Surveillance temps réel
- `.vibe/bin/audit.sh` - Audit complet
- `.vibe/bin/utils/` - Utilitaires (check-size, etc.)

---

## ✅ Checklist Validation Configuration

- [x] `.clinerules` créé avec 20 règles métier trading
- [x] `.vibe/config.toml` configuré (stack Rust+Vue, limites)
- [x] `projet.md` documentation complète
- [x] `.github/copilot-instructions.md` enrichi Vibe Coding
- [x] Mémoire repository mise à jour
- [x] Workflow 3 phases défini
- [x] Règles critiques documentées
- [ ] Sentinelle lancée (si scripts disponibles)
- [ ] Premier cycle développement testé

---

## 🚀 Commencer à Coder

**Tu es maintenant prêt pour le développement Vibe Coding !**

### Démarrer Phase 1 (Création)
```bash
# 1. (Optionnel) Lancer sentinelle si scripts Vibe disponibles
# ./.vibe/bin/sentinel.sh

# 2. Commencer à coder
# - Focus sur métier (trading, ML, risk)
# - Sentinelle formatera automatiquement
# - Tests locaux: cargo test

# 3. Quand prêt pour validation, dire "Valide" ou "Ready"
```

### Philosophie
- **Flow** : Code fluide sans charge mentale
- **Confiance** : Système gère qualité
- **Transparence** : Audits montrent résultats vrais

**VIBE CODING = Flow + Qualité + Zero Stress** 🎯
