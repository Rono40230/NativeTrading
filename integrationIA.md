# Plan d'intégration IA/LLM — Stratégies Rockets & SMC

> Objectif : porter Rockets et SMC au même niveau adaptatif que Straddle.  
> Pipeline cible : **Scan passif → Feedback structuré → Calibration auto → Few-shot dynamique**

---

## Référence : ce qu'a déjà Straddle

```
[scan_pics]     5 min  → straddle_pics       (ratio ATR > 1.3)
[boucle]       15 min  → signal_ollama        (few-shot + seuil calibré)
[feedback_job]  5 min  → straddle_feedback    (verdict, pnl_r, gagnant)
[calibration]      6 h → straddle_calibration (grid search WR×PF par asset+catégorie)
```

Tables dédiées : `straddle_pics`, `straddle_feedback`, `straddle_calibration`  
Fichiers backend : `straddle_scan_pics.rs`, `straddle_boucle.rs`, `straddle_signal_ollama.rs`,  
`straddle_feedback_job.rs`, `straddle_calibration.rs`, `straddle_categorisation.rs`,  
`straddle_monitoring_handlers.rs`, `straddle_ml_handlers.rs`

---

## Priorité 1 — Rockets (effort estimé : 3-4 jours)

**Pourquoi en premier :** le scan automatique Binance existe déjà (`rockets_scan.rs`), le filtre LLM existe (`ollama/rockets_filtre.rs`), le suivi TP pyramidal existe (`rockets_suivi.rs`). Il manque uniquement la couche analytique (feedback + calibration + few-shot).

### État actuel

| Composant | Fichier | Statut |
|---|---|---|
| Scan auto Binance | `rockets_scan.rs` | ✅ |
| Filtre LLM pré-signal | `ollama/rockets_filtre.rs` | ✅ (conviction ≥ 65 fixe) |
| Sauvegarde signal | `rockets_sauvegarder.rs` | ✅ |
| Suivi TP pyramidal | `rockets_suivi.rs` | ✅ |
| Analyse LLM hebdo | `rockets_analyse_handler.rs` | ✅ |
| Feedback structuré | — | ❌ |
| Calibration seuils | — | ❌ |
| Few-shot dynamique | — | ❌ |
| Monitoring métriques | — | ❌ |

### Gap critique : cache scan volatile

Le `static SCAN_RESULTS: OnceLock<Arc<RwLock<Vec<ScanResultat>>>>` est perdu au redémarrage.  
Les feedbacks ne peuvent pas être calculés si les données de scan (atr14, ratio_volume, phase) ne sont pas persistées.

---

### Étape R1 — Migration DB `rockets_feedback`

**Fichier :** `backend/crates/db/migrations/0028_rockets_feedback.sql`

```sql
CREATE TABLE IF NOT EXISTS rockets_feedback (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    signal_id       INTEGER NOT NULL UNIQUE,     -- FK → rockets_signaux.id
    ticker          TEXT    NOT NULL,
    phase           TEXT    NOT NULL,            -- 'breakout'|'prelancement'|'momentum'
    session_active  TEXT    NOT NULL DEFAULT 'Off',
    timestamp_signal INTEGER NOT NULL,
    score_scan      INTEGER NOT NULL,            -- score /100 au moment du scan
    conviction_llm  INTEGER NOT NULL,            -- conviction /100 retournée par LLM
    ratio_volume    REAL    NOT NULL DEFAULT 0.0,
    atr_ratio       REAL    NOT NULL DEFAULT 0.0,
    rsi             REAL    NOT NULL DEFAULT 50.0,
    verdict         TEXT,                        -- 'tp1'|'tp2'|'tp3'|'sl'|'expire'
    pnl_r           REAL,                        -- R multiple réalisé
    gagnant         INTEGER,                     -- 1|0|NULL
    duree_trade_min INTEGER,
    ferme_le        INTEGER,
    cree_le         INTEGER NOT NULL DEFAULT (unixepoch())
);
CREATE INDEX IF NOT EXISTS idx_rockets_feedback_ticker  ON rockets_feedback(ticker, phase);
CREATE INDEX IF NOT EXISTS idx_rockets_feedback_verdict ON rockets_feedback(verdict, gagnant);
CREATE INDEX IF NOT EXISTS idx_rockets_feedback_cree_le ON rockets_feedback(cree_le DESC);
```

**Fichier DB :** `backend/crates/db/src/rockets_feedback.rs`

Fonctions à créer :
- `inserer_feedback(pool, NouveauFeedback) -> Result<i64>`
- `réconcilier_feedback(pool, signal_id, verdict, pnl_r, gagnant, duree_min) -> Result<()>`
- `lister_feedbacks_ticker(pool, ticker, phase, limit) -> Result<Vec<RocketsFeedbackRow>>`  ← pour few-shot
- `stats_globales(pool) -> Result<RocketsStatsGlobales>`  ← pour monitoring
- `stats_par_phase(pool) -> Result<Vec<RocketsStatPhase>>`  ← pour monitoring

---

### Étape R2 — Migration DB `rockets_calibration`

**Fichier :** `backend/crates/db/migrations/0029_rockets_calibration.sql`

```sql
CREATE TABLE IF NOT EXISTS rockets_calibration (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    phase           TEXT    NOT NULL,            -- 'breakout'|'prelancement'|'momentum'
    session         TEXT    NOT NULL DEFAULT 'all', -- 'london'|'ny'|'asia'|'all'
    score_min       INTEGER NOT NULL DEFAULT 65, -- seuil score scan optimal
    conviction_min  INTEGER NOT NULL DEFAULT 65, -- seuil conviction LLM optimal
    nb_trades       INTEGER NOT NULL DEFAULT 0,
    win_rate        REAL    NOT NULL DEFAULT 0.0,
    pnl_moyen_r     REAL    NOT NULL DEFAULT 0.0,
    fiabilite       TEXT    NOT NULL DEFAULT 'insuffisant',
    invalide        INTEGER NOT NULL DEFAULT 0,
    maj_le          INTEGER NOT NULL DEFAULT (unixepoch()),
    UNIQUE(phase, session)
);
```

**Fichier DB :** `backend/crates/db/src/rockets_calibration.rs`

Fonctions : `sauvegarder(pool, row)`, `lire(pool, phase, session)`, `lister_tout(pool)`

Grilles de calibration :
- `SCORE_GRID: &[i64] = &[55, 60, 65, 70, 75, 80]`
- `CONVICTION_GRID: &[i64] = &[55, 60, 65, 70, 75]`

---

### Étape R3 — Modifier `rockets_sauvegarder.rs` : insérer feedback au moment du signal

**Modification de `filtrer_sauvegarder_publier()`** — après le `signal_engine.publier()` :

```rust
// Récupérer l'ID du signal inséré (retour de rockets::sauvegarder)
// Lire session active (chrono::Utc::now() → session_from_hour)
// Insérer rockets_feedback (verdict=NULL, gagnant=NULL)
db::rockets_feedback::inserer_feedback(pool, NouveauFeedback {
    signal_id: id_insere,
    ticker: r.ticker.clone(),
    phase: phase_sauvegardee.to_string(),
    session_active,
    timestamp_signal,
    score_scan: score_actuel,
    conviction_llm: rep.conviction,
    ratio_volume: r.ratio_volume,
    atr_ratio: r.atr_ratio,
    rsi: r.rsi,
}).await?;
```

---

### Étape R4 — Modifier `rockets_suivi.rs` : réconcilier feedback sur clôture

**Dans `demarrer_worker_suivi`** — après chaque `maj_verdict(fermé)` ou `enregistrer_tp_partiel` :

```rust
// Calculer pnl_r = (prix_sortie - prix_entree) / atr14
// Réconcilier rockets_feedback avec verdict + pnl_r + gagnant + duree_min
db::rockets_feedback::réconcilier_feedback(pool, signal_id, verdict, pnl_r, gagnant, duree_min).await?;
```

**Note TP partiels :** TP1 et TP2 réconcilier avec `gagnant=1` mais `pnl_r` partiel. TP3/SL = clôture finale.

---

### Étape R5 — Créer `rockets_calibration.rs` (job 6h)

**Fichier :** `backend/crates/api/src/rockets_calibration.rs`  
**Modèle :** `straddle_calibration.rs` (même structure grid search)

```
pub fn demarrer_calibration_rockets(db: Arc<Database>)
    // Spawn: sleep(600s) puis loop: recalibrer_tous_rockets, sleep(6h)

async fn recalibrer_tous_rockets(db)
    // Pour chaque (phase, session) avec nb_trades >= 20 :
    //   grille SCORE_GRID × CONVICTION_GRID
    //   sélectionner combo (score_min, conviction_min) qui maximise WR×PF
    //   fiabilite: 'insuffisant'(<20), 'faible'(<35), 'correct'(<50), 'fort'(>=50)
    //   invalide=true si aucun combo atteint WR>=50% sur tous les feedbacks
    //   sauvegarder rockets_calibration
```

**Déclaration dans `main.rs` :** `tokio::spawn(rockets_calibration::demarrer_calibration_rockets(db.clone()))`

---

### Étape R6 — Few-shot dynamique dans `rockets_sauvegarder.rs`

**Modification du prompt dans `filtrer_sauvegarder_publier()`** — avant l'appel à `ollama/rockets_filtre.rs` :

```rust
// Charger les 5 derniers feedbacks pour ce (ticker, phase)
let feedbacks = db::rockets_feedback::lister_feedbacks_ticker(pool, &r.ticker, phase, 5).await?;

// Charger seuils calibrés pour cette (phase, session)
let calib = db::rockets_calibration::lire(pool, phase, &session_active).await?;
let conviction_seuil = calib.map(|c| c.conviction_min).unwrap_or(65);

// Passer feedbacks à ollama/rockets_filtre::filtrer_signal_avec_few_shot(candidat, historique, &feedbacks)
```

**Modification `ollama/rockets_filtre.rs` :** ajouter paramètre `feedbacks: &[RocketsFeedbackRow]` → construire bloc few-shot dans le prompt (identique à `straddle_signal_ollama::construire_prompt_few_shot`).

---

### Étape R7 — Endpoints monitoring Rockets

**Fichier :** `backend/crates/api/src/rockets_monitoring_handlers.rs`  
**Modèle :** `straddle_monitoring_handlers.rs`

Endpoints à créer :
- `GET /api/rockets/monitoring` — stats globales + par phase (WR, pnl_r moyen, nb_trades, dérive si WR < 45%)
- `GET /api/rockets/calibration` — seuils actuels par (phase, session) + fiabilité

**Déclaration dans `main.rs` routes :** ajouter les 2 endpoints.

---

### Étape R8 — Frontend : onglet Métriques IA Rockets

**Composant :** `frontend/src/components/common/RocketsMonitoringML.vue` (≤ 250 lignes)

Contenu :
- Bandeau dérive orange si WR global < 45%
- 4 KPI cards : total trades, win rate global, trades gagnants, P&L R moyen
- Tableau calibration par phase/session (score_min optimal, conviction_min, fiabilité badge)
- Tableau performance par phase (WR, P&L R, nb trades)

**Intégration :** `PromptsIAView.vue` → onglet Rockets → sous-onglet "Métriques IA" → remplacer le placeholder actuel par `<RocketsMonitoringML />`

---

## Priorité 2 — SMC (effort estimé : 5-6 jours)

**Pourquoi en second :** la boucle automatique (le plus gros morceau) est à créer entièrement. Le `smc_filtre.rs` existe mais n'est connecté à rien. Le signal SMC est aujourd'hui 100% manuel (déclenchement HTTP frontend).

### État actuel

| Composant | Fichier | Statut |
|---|---|---|
| Scoring SMC | `smc_handlers.rs` + crate `smc/` | ✅ |
| Signal manuel | `ollama_signal_ia_handler.rs` | ✅ (POST HTTP seulement) |
| Filtre LLM | `ollama/smc_filtre.rs` | ✅ (non connecté) |
| Worker verdict | `signaux_handlers.rs` | ✅ (table `signaux` commune) |
| Analyse LLM hebdo | `smc_analyse_handler.rs` | ✅ |
| Boucle auto | — | ❌ |
| Feedback structuré | — | ❌ |
| Calibration seuils | — | ❌ |
| Catégorisation | — | ❌ |
| Few-shot dynamique | — | ❌ |
| Monitoring métriques | — | ❌ |

---

### Étape S1 — Migration DB `smc_feedback`

**Fichier :** `backend/crates/db/migrations/0030_smc_feedback.sql`

```sql
CREATE TABLE IF NOT EXISTS smc_feedback (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    signal_id           TEXT    NOT NULL UNIQUE,     -- FK → signaux.id
    asset               TEXT    NOT NULL,
    timeframe           TEXT    NOT NULL,
    timestamp_signal    INTEGER NOT NULL,
    categorie           TEXT    NOT NULL DEFAULT 'ob_seul',
    -- catégories: 'triple_confluence'|'ob_ifvg'|'ob_imbalance'|'ob_seul'|'fib_seul'|'choc_isole'
    session_active      TEXT    NOT NULL DEFAULT 'Off',
    score_smc           REAL    NOT NULL,            -- /100 au moment du signal
    confiance_ml        REAL    NOT NULL DEFAULT 0.0,
    kill_zone_active    INTEGER NOT NULL DEFAULT 0,
    sweep_detecte       INTEGER NOT NULL DEFAULT 0,
    conviction_llm      INTEGER NOT NULL,            -- /100
    verdict             TEXT,                        -- 'TP1'|'TP2'|'TP3'|'SL'|'expire'
    pnl_r               REAL,
    gagnant             INTEGER,                     -- 1|0|NULL
    duree_trade_min     INTEGER,
    ferme_le            INTEGER,
    cree_le             INTEGER NOT NULL DEFAULT (unixepoch())
);
CREATE INDEX IF NOT EXISTS idx_smc_feedback_asset     ON smc_feedback(asset, timeframe, categorie);
CREATE INDEX IF NOT EXISTS idx_smc_feedback_verdict   ON smc_feedback(verdict, gagnant);
CREATE INDEX IF NOT EXISTS idx_smc_feedback_cree_le   ON smc_feedback(cree_le DESC);
```

---

### Étape S2 — Migration DB `smc_calibration`

**Fichier :** `backend/crates/db/migrations/0031_smc_calibration.sql`

```sql
CREATE TABLE IF NOT EXISTS smc_calibration (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    asset           TEXT    NOT NULL,
    timeframe       TEXT    NOT NULL,
    categorie       TEXT    NOT NULL,
    score_smc_seuil REAL    NOT NULL DEFAULT 70.0,   -- seuil score SMC optimal
    conviction_seuil INTEGER NOT NULL DEFAULT 70,    -- seuil conviction LLM optimal
    nb_trades       INTEGER NOT NULL DEFAULT 0,
    win_rate        REAL    NOT NULL DEFAULT 0.0,
    pnl_moyen_r     REAL    NOT NULL DEFAULT 0.0,
    fiabilite       TEXT    NOT NULL DEFAULT 'insuffisant',
    invalide        INTEGER NOT NULL DEFAULT 0,
    maj_le          INTEGER NOT NULL DEFAULT (unixepoch()),
    UNIQUE(asset, timeframe, categorie)
);
```

Grilles :
- `SCORE_SMC_GRID: &[f64] = &[60.0, 65.0, 70.0, 75.0, 80.0]`
- `CONVICTION_GRID: &[i64] = &[55, 60, 65, 70, 75]`

---

### Étape S3 — Catégorisation SMC

**Fichier :** `backend/crates/api/src/smc_categorisation.rs`  
**Modèle :** `straddle_categorisation.rs`

Catégories ordonnées (priorité décroissante) :
```
triple_confluence  = score_ob + ifvg + imbalance (tous 3 présents)
ob_ifvg           = OB + IFVG détectés
ob_imbalance      = OB + Imbalance détectés
ob_seul           = OB seul
fib_confluence    = Fibonacci niveau + au moins 1 autre indicateur
fib_seul          = Fibonacci niveau seul
choc_isole        = aucune confluence claire
```

```rust
pub struct ResultatCategSmc {
    pub categorie: CategorieSmc,
    pub confluences_actives: Vec<String>,  // ["OB", "IFVG", "Imbalance"]
    pub session_active: String,
    pub kill_zone_active: bool,
    pub sweep_detecte: bool,
}

pub fn categoriser_smc(
    score_ob: f64, score_ifvg: f64, score_imbalance: f64,
    fib_actif: bool, kill_zone_active: bool, sweep_detecte: bool,
    asset: &str, now: DateTime<Utc>,
) -> ResultatCategSmc
```

---

### Étape S4 — DB `smc_feedback.rs` + `smc_calibration.rs`

**Fichiers :** `backend/crates/db/src/smc_feedback.rs` + `smc_calibration.rs`

`smc_feedback.rs` — fonctions :
- `inserer_feedback(pool, NouveauFeedbackSmc) -> Result<i64>`
- `reconcilier_feedback(pool, signal_id, verdict, pnl_r, gagnant, duree_min) -> Result<()>`
- `lister_feedbacks_asset_categorie(pool, asset, tf, categorie, limit) -> Result<Vec<SmcFeedbackRow>>`
- `stats_globales(pool) -> Result<SmcStatsGlobales>`
- `stats_par_categorie(pool) -> Result<Vec<SmcStatCategorie>>`

`smc_calibration.rs` — fonctions : `sauvegarder`, `lire(pool, asset, tf, categorie)`, `lister_tout`

---

### Étape S5 — Créer `smc_boucle.rs` (cœur du système auto)

**Fichier :** `backend/crates/api/src/smc_boucle.rs`  
**Modèle :** `straddle_boucle.rs` (262 lignes, même architecture)

```rust
const INTERVALLE_SEC: u64 = 900; // 15 min
const DELAI_MIN_ENTRE_SIGNAUX_MIN: i64 = 30; // éviter spam même asset+TF

pub fn demarrer_boucle_smc(db: Arc<Database>, signal_engine: Arc<SignalEngine>)
    // Spawn: sleep(60s) puis loop: executer_cycle_smc, sleep(INTERVALLE_SEC)

async fn executer_cycle_smc(db, signal_engine)
    // Pour chaque (asset, timeframe) dans ASSETS_ACTIFS × TIMEFRAMES_ACTIFS :
    //   1. Calculer scoring SMC via crate smc:: (tendance, OB, IFVG, imbalance, fib)
    //   2. Catégoriser via smc_categorisation::categoriser_smc(...)
    //   3. Charger seuils calibrés : db::smc_calibration::lire(asset, tf, categorie)
    //      → fallback: score_smc_seuil=70.0, conviction_seuil=70
    //   4. Skip si categorie.invalide = true
    //   5. Skip si signal ouvert récent sur (asset, tf) < DELAI_MIN_ENTRE_SIGNAUX_MIN
    //   6. Skip si score_smc < seuils.score_smc_seuil
    //   7. Charger feedbacks few-shot : db::smc_feedback::lister_feedbacks_asset_categorie(5)
    //   8. Appeler smc_signal_ollama::appeler_smc_et_publier(...)
```

---

### Étape S6 — Créer `smc_signal_ollama.rs`

**Fichier :** `backend/crates/api/src/smc_signal_ollama.rs`  
**Modèle :** `straddle_signal_ollama.rs` (146 lignes)

```rust
pub struct ParamsSmc<'a> {
    pub asset_str: &'a str,
    pub timeframe_str: &'a str,
    pub direction: &'a str,           // direction issue du scoring SMC (haussier/baissier)
    pub prix: f64,
    pub score_smc: f64,
    pub confiance_ml: f64,
    pub candidat_filtre: &'a SignalSMCCandidat,  // struct existante dans ollama/smc_filtre.rs
    pub feedbacks: &'a [db::smc_feedback::SmcFeedbackRow],
    pub conviction_seuil: i64,
    pub categorie: &'a smc_categorisation::CategorieSmc,
}

pub async fn appeler_smc_et_publier(
    db: &Arc<Database>,
    signal_engine: &Arc<SignalEngine>,
    params: ParamsSmc<'_>,
) -> anyhow::Result<()>
// Pipeline :
//   1. Construire prompt few-shot depuis feedbacks passés
//   2. Appeler ollama/smc_filtre::filtrer_signal_smc(candidat, &historique_convertis)
//      → rejeter si !rep.valide || rep.conviction < conviction_seuil
//   3. SL/TP avec ajustements LLM (rep.ajustements.sl_suggere, tp1_suggere)
//   4. Signal::nouveau(asset, tf, direction, score_smc, prix, sl, [tp1,tp2,tp3], "SMC Directionnel")
//   5. db.inserer_signal + inserer smc_feedback
//   6. signal_engine.publier + telegram::notifier_telegram
```

**Note :** réutiliser `ollama/smc_filtre.rs` tel quel, il est complet et correct. Simplement l'appeler depuis ce nouveau module.

---

### Étape S7 — Réconciliation feedback SMC (worker existant)

**Modification `signaux_handlers.rs`** — dans le worker 5 min, après `calculer_verdict` :

```rust
// Si verdict vient de changer (None → Some) ET strategie == "SMC Directionnel" :
// Calculer pnl_r = (prix_verdict - prix_entree) / atr (stocker atr dans smc_feedback au moment insertion)
// db::smc_feedback::reconcilier_feedback(pool, signal_id, verdict, pnl_r, gagnant, duree_min).await?;
```

**Alternative propre :** créer `smc_feedback_job.rs` dédié (modèle `straddle_feedback_job.rs`) qui poll les `signaux` ouverts de strategie="SMC Directionnel" et reconcilie.

---

### Étape S8 — Créer `smc_calibration.rs` (job 6h)

**Fichier :** `backend/crates/api/src/smc_calibration_job.rs`  
**Modèle :** `straddle_calibration.rs` — strictement identique, adapter les constantes :

```rust
const SCORE_SMC_GRID: &[f64] = &[60.0, 65.0, 70.0, 75.0, 80.0];
const CONVICTION_GRID: &[i64] = &[55, 60, 65, 70, 75];
const WIN_RATE_MIN: f64 = 0.50;
const NB_TRADES_MIN_CALIBRATION: i64 = 15; // SMC a moins de signaux que Straddle

pub fn demarrer_calibration_smc(db: Arc<Database>)
    // Spawn: sleep(600s) puis loop: recalibrer_tous_smc, sleep(6h)

async fn recalibrer_tous_smc(db)
    // lister_triplets_actifs() → (asset, timeframe, categorie) ayant nb_trades >= NB_TRADES_MIN
    // Pour chaque triplet : grid search SCORE_SMC_GRID × CONVICTION_GRID
    // sauvegarder smc_calibration avec fiabilite + invalide flag
```

---

### Étape S9 — Endpoints monitoring SMC

**Fichier :** `backend/crates/api/src/smc_monitoring_handlers.rs`

Endpoints :
- `GET /api/smc/monitoring` — stats globales par catégorie (WR, pnl_r, dérive si WR < 45%)
- `GET /api/smc/calibration` — seuils par (asset, TF, catégorie) + fiabilité

---

### Étape S10 — Frontend : onglet Métriques IA SMC

**Composant :** `frontend/src/components/common/SmcMonitoringML.vue` (≤ 250 lignes)  
**Contenu :** identique à `StraddleMonitoringML.vue` — adapter les endpoints et intitulés des catégories

**Intégration :** `PromptsIAView.vue` → onglet SMC → sous-onglet "Métriques IA" → remplacer `<MonitoringML />` (monitoring LSTM/XGBoost) par `<SmcMonitoringML />` **+ conserver** `<MonitoringML />` dans un 3e sous-onglet "Modèle LSTM"

---

## Séquence d'implémentation recommandée

### Rockets (semaine 1)

| Jour | Tâches |
|---|---|
| 1 | R1 (migration feedback) + R2 (migration calibration) + fichiers DB |
| 2 | R3 (modifier rockets_sauvegarder) + R4 (modifier rockets_suivi) |
| 3 | R5 (job calibration) + déclaration main.rs |
| 4 | R6 (few-shot dynamique) + R7 (endpoints monitoring) + R8 (frontend) |

### SMC (semaine 2)

| Jour | Tâches |
|---|---|
| 1 | S1 (migration feedback) + S2 (migration calibration) + fichiers DB |
| 2 | S3 (catégorisation) + S4 (fonctions DB) |
| 3 | S5 (smc_boucle.rs — boucle auto) |
| 4 | S6 (smc_signal_ollama.rs) + connecter smc_filtre.rs |
| 5 | S7 (réconciliation feedback) + S8 (job calibration) + S9 + S10 (endpoints + frontend) |

---

## Règles transverses à respecter

### Rust (non-négociables)
- Zéro `unwrap()` — propagation `?` ou `match`
- Timeout explicite sur tous les appels Ollama : `Duration::from_secs(90)`
- Mesurer latence LLM : `Instant::now()` + `tracing::info!("LLM SMC: {:?}", debut.elapsed())`
- Chaque nouveau fichier ≤ 300 lignes — splitter si nécessaire
- Déclarer les nouveaux `mod` dans `main.rs` + routes dans le bloc `web::scope`

### Architecture DAG
```
Layer 4: smc_boucle / rockets_calibration (orchestrateurs)
   ↓
Layer 3: smc_signal_ollama / ollama/smc_filtre (services LLM)
   ↓
Layer 2: db::smc_feedback / db::rockets_feedback (data access)
   ↓
Layer 1: common::Signal, TradingError (types partagés)
```
Interdiction : `smc_boucle` ne peut pas importer `rockets_*`, et vice versa.

### Frontend
- Zéro calcul métier en Vue — les WR, pnl_r, seuils calibrés viennent uniquement du backend
- Composants monitoring ≤ 250 lignes — extraire en sous-composants si dépassement

---

## Fichiers impactés — récapitulatif

### Rockets

| Fichier | Action |
|---|---|
| `db/migrations/0028_rockets_feedback.sql` | Créer |
| `db/migrations/0029_rockets_calibration.sql` | Créer |
| `db/src/rockets_feedback.rs` | Créer |
| `db/src/rockets_calibration.rs` | Créer |
| `api/src/rockets_sauvegarder.rs` | Modifier (inserer_feedback + few-shot) |
| `api/src/rockets_suivi.rs` | Modifier (reconcilier_feedback) |
| `api/src/rockets_calibration.rs` | Créer |
| `api/src/rockets_monitoring_handlers.rs` | Créer |
| `api/src/ollama/rockets_filtre.rs` | Modifier (ajouter feedbacks few-shot) |
| `api/src/main.rs` | Modifier (mod + routes + spawn) |
| `frontend/src/components/common/RocketsMonitoringML.vue` | Créer |
| `frontend/src/views/PromptsIAView.vue` | Modifier (remplacer placeholder Rockets) |

### SMC

| Fichier | Action |
|---|---|
| `db/migrations/0030_smc_feedback.sql` | Créer |
| `db/migrations/0031_smc_calibration.sql` | Créer |
| `db/src/smc_feedback.rs` | Créer |
| `db/src/smc_calibration.rs` | Créer |
| `api/src/smc_categorisation.rs` | Créer |
| `api/src/smc_boucle.rs` | Créer |
| `api/src/smc_signal_ollama.rs` | Créer |
| `api/src/smc_feedback_job.rs` | Créer |
| `api/src/smc_calibration_job.rs` | Créer |
| `api/src/smc_monitoring_handlers.rs` | Créer |
| `api/src/main.rs` | Modifier (mod + routes + spawn) |
| `frontend/src/components/common/SmcMonitoringML.vue` | Créer |
| `frontend/src/views/PromptsIAView.vue` | Modifier (onglet SMC Métriques IA) |
