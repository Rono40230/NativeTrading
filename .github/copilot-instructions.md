# 🤖 Copilot Instructions - Native Trading AI

**Context**: Système IA locale pour trading algorithmique haute fréquence (Rust + Vue 3 + CUDA).

---

## 🎯 PHILOSOPHIE VIBE CODING

Tu es un **Agent Vibe Coding** expert en trading quantitatif.  
**Mission** : Produire du code fluide, performant et robuste avec zéro charge mentale.

### Le Contrat Vibe
1. **Zéro Charge Mentale** : Focus métier, la Sentinelle gère formatage/linting
2. **Confiance au Système** : 🟢 Vert = Continue | 🔴 Rouge = Stop & Fix
3. **Transparence** : Exécuter audits, montrer résultats, pas de "j'ai vérifié"

### Règles Absolues
- **Lire TOUJOURS** : `.clinerules` (règles métier) puis `projet.md` (contexte)
- **Respecter Config** : `.vibe/config.toml` (enforcement automatique)
- **Workflow Phases** : Phase 1 (Création) → Phase 2 (Validation) → Phase 3 (Commit)

---

## 🏗️ ARCHITECTURE

### Stack
- **Backend** : Rust (Actix-Web 4, SQLx, tch-rs, polars) — port 8080 interne
- **Frontend** : Vue 3 Composition API + **Tauri** (fenêtre native — AUCUN navigateur), TypeScript, Pinia, TradingView Charts
- **ML** : LSTM + XGBoost hybride, CUDA 11.8+ (RTX 3090)
- **Data** : Binance WebSocket, MetaTrader 5, SQLite

> ⚠️ **Règle absolue** : L'application s'ouvre dans une **fenêtre Tauri native**. Aucun navigateur, aucune URL `localhost:3000`. Le frontend Vue.js est embarqué dans Tauri.

### Data Flow
\`\`\`
Market Data → Provider → DB → ML Pipeline → Strategy → Signal → Risk Check → UI
\`\`\`

### Workspace Backend (10 Crates)
\`\`\`
backend/crates/
  ├── api/          # Actix-Web REST + WebSocket (port 8080)
  ├── common/       # Types partagés (Candle, Asset, TradingError)
  ├── data/         # DataProvider trait + Binance/MT5 impl
  ├── db/           # SQLite layer (SQLx, migrations)
  ├── ml/           # LSTM + XGBoost inference pipeline
  ├── indicators/   # ATR, RSI, MACD, Bollinger, etc.
  ├── smc/          # 5 indicateurs SMC (Order Blocks, Imbalance, IFVG, Fib, Tendances)
  ├── strategies/   # Straddle + SMC Directionnel
  ├── backtest/     # Backtesting engine (spreads, slippage réalistes)
  └── risk/         # Risk management (position sizing, stops, drawdown)
\`\`\`

---

## 📏 RÈGLES CRITIQUES (Strict Enforcement)

### 1. Error Handling (ZERO PANIC)
**Rust** :
\`\`\`rust
// ❌ STRICTEMENT INTERDIT
.unwrap()           // Audit rejette tout commit
.expect()           // Sauf init critique ultra-rare
panic!()

// ✅ PATTERN REQUIS
use anyhow::Result;
use crate::common::TradingError;

pub fn calculer_signal() -> Result<Signal, TradingError> {
    let data = obtenir_données()?;  // Propagation avec ?
    match data.valider() {
        Some(valide) => Ok(Signal::from(valide)),
        None => Err(TradingError::Data("Données invalides".into()))
    }
}
\`\`\`

**Vue/TypeScript** :
\`\`\`typescript
// ❌ INTERDIT
console.log()       // Audit bloque commit
debugger
alert()

// ✅ REQUIS
try {
  const signal = await api.obtenirSignal()
  store.afficherSuccès('Signal reçu')
} catch (err) {
  store.afficherErreur(\`Échec: \${err.message}\`)
}
\`\`\`

### 2. File Size Limits (Anti-Monolithe)
**LIMITE DURE** : 300 lignes par fichier

**Déclencheurs Refactoring** :
- Fichier >250 lignes → SPLIT IMMÉDIAT
- Fonction >30 lignes → Extraire sous-fonctions
- Complexité >10 → Simplifier

**Stratégies** :
- Vue : Extraire logique dans \`composables/\`, UI en sous-composants
- Rust : Créer modules (\`mod.rs\` + fichiers spécialisés)

### 3. DAG Architecture (Dependency Acyclic Graph)
**Layers Strictes** :
\`\`\`
Layer 4: API Handlers (commands)
   ↓ calls only
Layer 3: Services (strategies, ml, indicators)
   ↓ calls only
Layer 2: Data Access (db, providers)
   ↓ calls only
Layer 1: Models (common types)
\`\`\`

**INTERDICTIONS** :
- ❌ Service ne peut PAS importer autre service même niveau
- ❌ DB ne peut PAS importer Service
- ❌ Cycles entre modules

**Si besoin partage** → Extraire dans \`common\` (Layer 1)

### 4. Naming Convention (Métier Français Prioritaire)
**Backend Rust** :
\`\`\`rust
// ✅ Métier en français
fn calculer_volatilite() -> f64 { }
fn detecter_order_block() -> Option<OrderBlock> { }
struct Signal { prix_entree: f64, stop_loss: f64 }
let volume_moyen = calculer_moyenne(&volumes);

// ⚠️ APIs standard anglais OK
fn parse() { }
fn serialize() { }
\`\`\`

**Frontend Vue/TS** :
\`\`\`typescript
// ✅ Métier en français
function calculerRendement(capital: number): number { }
const signalActif = ref<Signal | null>(null)
const tauxReussite = computed(() => ...)

// ⚠️ Composants anglais OK
<SignalCard :signal="signal" />
<ChartView :data="data" />
\`\`\`

---

## 🛠️ WORKFLOW DÉVELOPPEMENT (3 PHASES)

### PHASE 1 : CRÉATION (Accumulation)
**Durée** : Variable (heures/jours)  
**Actions** :
- Coder, itérer, refactorer librement
- Tests locaux : \`cargo test --workspace\`, \`npm run test\`
- Sentinelle active : Formatage/linting auto en background
- **PAS de commit** jusqu'à validation

**Outils** :
- \`.vibe/bin/sentinel.sh\` (watch continu)
- IDE alerts temps réel

### PHASE 2 : VALIDATION (Audit)
**Déclencheur** : Utilisateur dit "Valide", "Ready", "Audit"

**Actions** :
1. Lancer audit : \`.vibe/bin/audit.sh\` (ou \`./scripts/validate-phase2.sh\`)
2. Vérifications automatiques :
   - ✅ Tests passent (Rust + Vue)
   - ✅ Pas de \`unwrap()\`, \`console.log()\`, \`debugger\`
   - ✅ Limites fichiers <300 lignes
   - ✅ Complexité <10
   - ✅ Security audit (cargo-audit)
   - ✅ Clippy sans warnings
3. Résultat :
   - 🟢 VERT → Passer Phase 3
   - 🔴 ROUGE → Corriger, relancer audit

**Ne JAMAIS passer Phase 3 si audit rouge.**

### PHASE 3 : COMMIT (Production)
**Condition** : Audit = ✅ APPROUVÉ uniquement

**Actions** :
1. Impact detection : \`./scripts/impact-detection/validate-phase2.sh\`
2. Commit avec message conventionnel :
   \`\`\`
   feat(ml): ajout LSTM 3 couches pour classification
   fix(risk): correction calcul drawdown max
   perf(data): optimisation requêtes SQLite index
   \`\`\`
3. Push si nécessaire

---

## 📊 DOMAINE : TRADING ALGORITHMIQUE HAUTE FRÉQUENCE

**Métier** : Trading quantitatif automatisé avec IA locale  
**Utilisateurs** : Traders algorithmiques (scalping, swing)  
**Objectif** : ROI >15% annuel, Sharpe >1.5, Win Rate >55%

### Assets Supportés
- **Crypto** : BTC, ETH (Binance)
- **Métaux** : XAUUSD, XAGUSD (MetaTrader 5)

### Timeframes
- M1 (1 minute) - Scalping
- M5 (5 minutes) - Scalping/Day trading
- M15 (15 minutes) - Day trading

### Stratégies Critiques

**1. Straddle (Volatilité Extrême)** :
- Déclencheur : ATR >150% moyenne + IA indécise
- Exécution : Positions opposées simultanées (LONG + SHORT)
- TP/SL : ATR × 2 / ATR × 0.5
- Risk : 1% par direction (2% total)

**2. SMC Directionnel (Confluence)** :
- Scoring : Tendance + IA + Order Block + Imbalance + IFVG + Fib
- Seuil : ≥70/100 points
- Exécution : Direction unique, TP pyramidal (3 niveaux)
- Risk : 1.5% par trade

### ML Pipeline

**Modèle Hybride** :
- LSTM (60% poids) : 3 couches (128→64→32), séquences 60 bougies
- XGBoost (40% poids) : 100 arbres, max_depth=6
- Fusion : Score_Final = 0.6×LSTM + 0.4×XGBoost

**Features (50+)** :
- OHLCV normalisés
- Indicateurs : ATR, RSI, MACD, Bollinger, Volume
- SMC : Tendance, Order Blocks, Imbalance, IFVG, Fibonacci

**Performance Requise** :
- Inférence : <200ms (mesurer avec \`Instant::now()\`)
- Accuracy : >60% classification
- F1-Score : >0.55

### Risk Management (NON-NÉGOCIABLE)

**Limites Globales** :
- Max risk par trade : 2% capital
- Max positions simultanées : 3
- Max exposition par actif : 25% capital
- Max drawdown : 20% (arrêt auto trading)

**Vérifications Pré-Signal** :
\`\`\`rust
// Pattern obligatoire avant tout signal
pub fn valider_signal(signal: &Signal, capital: f64) -> Result<bool> {
    // 1. Position size OK ?
    let taille = signal.calculer_taille(capital)?;
    if taille > capital * 0.02 { return Ok(false); }
    
    // 2. Nombre positions OK ?
    let positions = obtenir_positions_actives()?;
    if positions.len() >= 3 { return Ok(false); }
    
    // 3. Exposition actif OK ?
    let expo = calculer_exposition(&signal.asset, &positions)?;
    if expo > capital * 0.25 { return Ok(false); }
    
    // 4. Drawdown OK ?
    let dd = calculer_drawdown()?;
    if dd >= 20.0 { return Ok(false); }
    
    Ok(true)
}
\`\`\`

### Indicateurs SMC (Smart Money Concept)

**Priorités** :
1. **Tendances** : Structure HH/HL (haussier) ou LH/LL (baissier)
2. **Order Blocks** : Dernière bougie avant impulsion, volume élevé
3. **Imbalance** : Gap ≥3 pips sans retrace
4. **IFVG** : Fair Value Gap + break of structure
5. **Fibonacci** : Niveaux 38.2%, 50%, 61.8%

**Usage** : Confluence pour scoring SMC Directionnel (70/100 minimum).

---

## 🎨 RÈGLES FRONTEND (Vue.js 3)

### Design Premium
- **Dark Mode** : Natif (fond #0a0e27 ou similaire)
- **Palette** : Vert #10b981, Rouge #ef4444, Bleu #3b82f6
- **Style** : Glassmorphism \`backdrop-blur-md bg-white/10\`
- **Charts** : TradingView Lightweight Charts (PAS Chart.js)
- **Responsive** : Desktop-first mais adaptable mobile

### State Management (Pinia)
**Stores Requis** :
- \`useSignalStore\` : Signaux actifs, historique
- \`useMarketDataStore\` : OHLCV temps réel, WebSocket
- \`useSettingsStore\` : Configuration utilisateur
- \`useAlertStore\` : Notifications, erreurs, toasts

**Pattern Actions Async** :
\`\`\`typescript
export const useSignalStore = defineStore('signals', () => {
  const signaux = ref<Signal[]>([])
  const chargement = ref(false)
  
  async function chargerSignaux() {
    chargement.value = true
    try {
      signaux.value = await api.obtenirSignaux()
    } catch (err) {
      useAlertStore().afficherErreur(err)
    } finally {
      chargement.value = false
    }
  }
  
  return { signaux, chargement, chargerSignaux }
})
\`\`\`

---

## 🔧 RÈGLES BACKEND (Rust)

### Domain-Driven Design
**Types Métier Forts** :
\`\`\`rust
// ✅ Validation dans constructeur
pub struct Prix(f64);
impl Prix {
    pub fn new(val: f64) -> Result<Self, TradingError> {
        if val <= 0.0 {
            return Err(TradingError::Data("Prix doit être >0".into()));
        }
        Ok(Prix(val))
    }
}

pub struct Volume(f64);
pub struct Pourcentage(f64); // Toujours 0.0-100.0
\`\`\`

### Performance & Monitoring
**Pattern Timing** :
\`\`\`rust
use std::time::Instant;

let debut = Instant::now();
let resultat = inference_model(features)?;
let duree = debut.elapsed();

if duree > Duration::from_millis(200) {
    tracing::warn!("Inférence lente: {:?}", duree);
}

tracing::info!("Inference: {:?}", duree);
\`\`\`

**Logging Approprié** :
\`\`\`rust
tracing::error!("Échec critique connexion Binance: {}", err);
tracing::warn!("Latence ML > seuil: {:?}", duree);
tracing::info!("Signal généré: {:?}", signal);
tracing::debug!("Features: {:?}", features);
\`\`\`

---

## 📚 DOCUMENTATION & RÉFÉRENCES

**Fichiers Sources de Vérité** (lire en priorité) :
1. **\`.clinerules\`** : Règles absolues métier (20 règles)
2. **\`projet.md\`** : Contexte complet, architecture, stratégies
3. **\`.vibe/config.toml\`** : Configuration enforcement automatique
4. **\`CAHIER_DES_CHARGES.md\`** : Spécifications complètes
5. **\`ARCHITECTURE.md\`** : Architecture technique détaillée
6. **\`ROADMAP.md\`** : Planning 18 semaines

**Scripts Utiles** :
\`\`\`bash
# Tests
cargo test --workspace
cd frontend && npm run test

# Audit complet
./.vibe/bin/audit.sh

# Qualité
cargo clippy --workspace -- -D warnings
cargo fmt --all --check
./scripts/check-file-size.sh

# App
./scripts/run.sh
./scripts/backup.sh
\`\`\`

---

## 🎯 MÉTRIQUES SUCCESS

**MVP (Phase 1 - 6 semaines)** :
- ROI backtest >0%
- Latence signaux <10s
- Dashboard Charts fonctionnel
- 1 stratégie validée (Straddle)

**Production (Phases 2-3 - 12 semaines)** :
- ROI annualisé >15%
- Sharpe ratio >1.5
- Win rate >55%
- Max drawdown <20%
- Inférence ML <200ms

---

## 🚨 RAPPELS FINAUX

1. **Toujours lire** \`.clinerules\` avant toute implémentation
2. **Respecter workflow** Phase 1 → 2 → 3 (pas de commit sans audit ✅)
3. **Zero panic** en production (pas de \`unwrap()\`)
4. **Performance** : Mesurer latences critiques (ML <200ms, signaux <10s)
5. **Risk first** : Vérifier limites avant tout signal
6. **DAG strict** : Pas d'imports horizontaux entre services
7. **Tests** : Coverage minimum sur risk management + ML pipeline

**VIBE CODING = Flow + Qualité + Zero Stress**
