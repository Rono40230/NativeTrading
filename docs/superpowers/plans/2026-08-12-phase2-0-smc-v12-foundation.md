# Phase 2.0 — SMC v12 Foundation (calibration + ATR + pivots + structure + BOS)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development.

**Goal:** Implémenter le socle du moteur SMC v12 en Rust : détection d'actif + calibration, ATR14, pivots/swings, structure HH/HL/LH/LL, et BOS — dans un nouveau module `smc::v12` coexistant avec l'ancien SMC. C'est la fondation sur laquelle tous les autres indicateurs v12 (liquidités, OB, FVG, scoring…) seront bâtis.

**Architecture:** Nouveau sous-module `smc/src/v12/` dans le crate `smc`. Chaque indicateur = un struct avec `update(bar)`. Un `SmcV12Engine` orchestre dans l'ordre strict du Pine. Coexistence avec `smc::scorer` (l'ancien) jusqu'à validation. Tests unitaires + intégration sur les 700 bars XAUUSD M15 du spike.

**Tech Stack:** Rust, crate `smc` (existant), `common::Candle` (type bar), `indicators` (ATR si dispo).

## Global Constraints

- **Zéro panic Vibe** ; limite 600 (D0) ; **test avant validation** (`cargo test -p smc`).
- **Commit local par tâche** ; **push = propriétaire** ; **l'assistant pose ses questions et attend**.
- **Parité Pine** : chaque indicateur doit reproduire EXACTEMENT la logique du v12 (lignes Pine citées). Les tests vérifient la logique, pas juste la compilation.
- **Anti-repaint** : ne traiter QUE les bars clôturées (équivalent `barstate.isconfirmed`).

### Build backend — env CUDA (voir plans précédents).

---

## File Structure

| Fichier | Action | Rôle |
|---------|--------|------|
| `smc/src/v12/mod.rs` | **Créer** | `SmcV12Engine` orchestrateur + ré-exports |
| `smc/src/v12/types.rs` | **Créer** | Types partagés (`BarInput`, `Pivot`, `BosEvent`, `StructureEvent`) |
| `smc/src/v12/calibration.rs` | **Créer** | Détection actif + tables de calibration (swingLength, seuils, pip, slMode…) |
| `smc/src/v12/atr.rs` | **Créer** | ATR14 (Wilder) |
| `smc/src/v12/pivots.rs` | **Créer** | Détection pivots high/low + maintenance sh1/sl1/sh2/sl2 |
| `smc/src/v12/structure.rs` | **Créer** | HH/HL/LH/LL + bullCount/bearCount + tendance |
| `smc/src/v12/bos.rs` | **Créer** | BOS haussier/baissier + anti-doublon |
| `smc/src/v12/tests.rs` | **Créer** | Tests unitaires + intégration 700 bars |
| `smc/src/lib.rs` | **Modifier** | Ajouter `pub mod v12;` |

---

## Task 0: Branche + baseline

- [ ] `git checkout main && git checkout -b phase2-0-smc-v12-foundation`
- [ ] Baseline : `cargo test -p smc` (tests existants verts).

---

## Task 1: Module v12 + types + calibration

### 1.1 Créer `smc/src/v12/mod.rs`

```rust
//! SMC v12 — reproduction fidèle de smc_indicateur_v12.pine.
//! Coexiste avec l'ancien smc::scorer jusqu'à validation, puis bascule.
pub mod types;
pub mod calibration;
pub mod atr;
pub mod pivots;
pub mod structure;
pub mod bos;

pub use types::*;
pub use calibration::AssetCalibration;
pub use atr::Atr14;
pub use pivots::PivotDetector;
pub use structure::StructureDetector;
pub use bos::BosDetector;

/// Le moteur SMC v12 — orchestre tous les indicateurs dans l'ordre strict du Pine.
pub struct SmcV12Engine {
    pub calibration: AssetCalibration,
    pub atr: Atr14,
    pub pivots: PivotDetector,
    pub structure: StructureDetector,
    pub bos: BosDetector,
}

impl SmcV12Engine {
    /// Crée le moteur pour un actif + timeframe donnés.
    pub fn new(asset: &str, timeframe: &str) -> Self {
        let cal = AssetCalibration::detect(asset, timeframe);
        Self {
            calibration: cal.clone(),
            atr: Atr14::new(),
            pivots: PivotDetector::new(cal.swing_length),
            structure: StructureDetector::new(),
            bos: BosDetector::new(),
        }
    }

    /// Traite une nouvelle bar clôturée. Ordre strict = ordre Pine.
    pub fn update(&mut self, bar: &BarInput) -> SmcOutput {
        self.atr.update(bar);
        self.pivots.update(bar);
        let pivot_event = self.pivots.last_event();
        self.structure.update(bar, &pivot_event);
        let struct_event = self.structure.last_event();
        self.bos.update(bar, &self.pivots, &self.structure);

        SmcOutput {
            atr14: self.atr.value(),
            pivot: pivot_event,
            structure: struct_event,
            bos: self.bos.last_event(),
            sh1: self.pivots.sh1(),
            sl1: self.pivots.sl1(),
            tendance_haussiere: self.structure.tendance_haussiere(),
            tendance_baissiere: self.structure.tendance_baissiere(),
        }
    }
}
```

### 1.2 Créer `smc/src/v12/types.rs`

```rust
/// Une bar OHLCV clôturée (équivalent Pine barstate.isconfirmed).
#[derive(Debug, Clone)]
pub struct BarInput {
    pub timestamp: i64,   // Unix secondes
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

/// Événement pivot détecté (high ou low).
#[derive(Debug, Clone, Default)]
pub struct PivotEvent {
    pub is_pivot_high: bool,
    pub is_pivot_low: bool,
    pub pivot_high_price: Option<f64>,
    pub pivot_low_price: Option<f64>,
    pub pivot_bar_index: Option<usize>,  // index de la bar pivot (pas la bar courante)
}

/// Structure : HH/HL/LH/LL + tendance.
#[derive(Debug, Clone, Default)]
pub struct StructureEvent {
    pub is_hh: bool, pub is_hl: bool, pub is_lh: bool, pub is_ll: bool,
    pub bull_count: u32, pub bear_count: u32,
    pub tendance_haussiere: bool, pub tendance_baissiere: bool,
}

/// BOS détecté.
#[derive(Debug, Clone, Default)]
pub struct BosEvent {
    pub bullish: bool, pub bearish: bool,
    pub level: Option<f64>,
    pub bar_index: Option<usize>,
}

/// Sortie complète du moteur pour une bar.
#[derive(Debug, Clone, Default)]
pub struct SmcOutput {
    pub atr14: f64,
    pub pivot: PivotEvent,
    pub structure: StructureEvent,
    pub bos: BosEvent,
    pub sh1: Option<f64>,  // dernier swing high
    pub sl1: Option<f64>,  // dernier swing low
    pub tendance_haussiere: bool,
    pub tendance_baissiere: bool,
}
```

### 1.3 Créer `smc/src/v12/calibration.rs`

Reproduit le Module 0 du Pine (lignes 12-111). Tables EXACTES de la spec v12.

```rust
/// Calibration par actif — tables exactes du v12 Pine (Module 0, lignes 12-111).
#[derive(Debug, Clone)]
pub struct AssetCalibration {
    pub is_xau: bool, pub is_xag: bool, pub is_nas: bool, pub is_btc: bool, pub is_dax: bool,
    pub asset_reconnu: bool,
    pub swing_length: usize,
    pub pip_value: f64,
    pub pv_lot: f64,
    pub sl_mode: SlMode,
    // Seuils scoring (utilisés plus tard en 2.5)
    pub seuil_moyen: i32, pub seuil_fort: i32, pub seuil_instit: i32, pub score_max: i32,
    pub w_fvg: i32, pub w_sweep: i32, pub w_atr: i32, pub w_ote: i32, pub w_kz: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SlMode { BasOb, Atr1x, Atr15x, Atr2x }

impl AssetCalibration {
    pub fn detect(asset: &str, timeframe: &str) -> Self {
        let a = asset.to_uppercase();
        let is_xau = a.contains("XAU");
        let is_xag = a.contains("XAG");
        let is_nas = a.contains("NAS") || a.contains("NDX") || a.contains("US100");
        let is_btc = a.contains("BTC");
        let is_dax = a.contains("DAX") || a.contains("GER40") || a.contains("DE30");
        let asset_reconnu = is_xau || is_xag || is_nas || is_btc || is_dax;

        // _autoSwing (Pine lignes 42-47)
        let tf_m15 = parse_tf_minutes(timeframe) <= 15;
        let swing_length = if tf_m15 { 3 } else {
            match (is_xau, is_xag, is_nas, is_btc, is_dax) {
                (true, _, _, _, _) => 5,
                (_, true, _, _, _) => 4,
                _ => 5,
            }
        };

        // _pipValue / _pvLot (Pine lignes 33-36)
        let (pip_value, pv_lot) = if is_xau { (0.1, 10.0) }
            else if is_xag { (0.01, 50.0) }
            else if is_nas { (1.0, 20.0) }
            else if is_btc { (1.0, 1.0) }
            else if is_dax { (1.0, 25.0) }
            else { (1.0, 1.0) };

        // _autoSlMode (Pine lignes 78-81)
        let sl_mode = if is_btc { SlMode::Atr2x }
            else if is_xag { SlMode::Atr15x }
            else if is_xau || is_nas || is_dax { SlMode::Atr1x }
            else { SlMode::BasOb };

        // Seuils scoring (Pine lignes 986-991)
        let (seuil_moyen, seuil_fort, seuil_instit, score_max) = if is_xau { (7, 10, 12, 13) }
            else if is_xag { (7, 99, 99, 14) }
            else if is_nas { (10, 15, 17, 19) }
            else if is_dax { (11, 16, 19, 21) }
            else if is_btc { (8, 99, 99, 15) }
            else { (7, 99, 99, 13) };

        let (w_fvg, w_sweep, w_atr, w_ote, w_kz) = if is_xau { (5, 4, 2, 5, 3) }
            else if is_xag { (5, 2, 2, 2, 3) }
            else if is_nas { (4, 4, 2, 5, 3) }
            else if is_dax { (4, 4, 2, 5, 3) }
            else if is_btc { (3, 1, 2, 2, 2) }
            else { (4, 4, 2, 5, 3) };

        Self { is_xau, is_xag, is_nas, is_btc, is_dax, asset_reconnu, swing_length,
               pip_value, pv_lot, sl_mode, seuil_moyen, seuil_fort, seuil_instit, score_max,
               w_fvg, w_sweep, w_atr, w_ote, w_kz }
    }
}

fn parse_tf_minutes(tf: &str) -> u32 {
    match tf { "M1"=>1,"M5"=>5,"M15"=>15,"M30"=>30,"H1"=>60,"H4"=>240,"D1"=>1440,"W1"=>10080,
               _ => tf.trim_end_matches(|c: char| !c.is_ascii_digit()).parse().unwrap_or(15) }
}
```

### 1.4 Modifier `smc/src/lib.rs`

Ajouter `pub mod v12;`.

### 1.5 Build + commit

- [ ] `cargo build -p smc` → OK.
- [ ] **Commit** : `feat(smc): module v12 — structure + types + calibration asset`

---

## Task 2: ATR14 (Wilder)

### 2.1 Créer `smc/src/v12/atr.rs`

Reproduit `atr14 = ta.atr(14)` (Pine ligne 421). Wilder's smoothing.

```rust
/// ATR14 (Wilder) — équivalent ta.atr(14) en Pine.
pub struct Atr14 {
    period: usize,
    bars: Vec<BarInput>,
    atr: f64,
    initialized: bool,
}

impl Atr14 {
    pub fn new() -> Self { Self { period: 14, bars: Vec::new(), atr: 0.0, initialized: false } }

    pub fn update(&mut self, bar: &BarInput) {
        self.bars.push(bar.clone());
        let n = self.bars.len();
        if n == 1 { return; }

        let prev_close = self.bars[n - 2].close;
        let tr = (bar.high - bar.low)
            .max((bar.high - prev_close).abs())
            .max((bar.low - prev_close).abs());

        if !self.initialized && n >= self.period + 1 {
            // Premier ATR = moyenne simple des TR sur `period` bars
            let sum: f64 = (1..=self.period).map(|i| {
                let p = &self.bars[i];
                let pc = self.bars[i - 1].close;
                (p.high - p.low).max((p.high - pc).abs()).max((p.low - pc).abs())
            }).sum();
            self.atr = sum / self.period as f64;
            self.initialized = true;
        } else if self.initialized {
            // Wilder smoothing: ATR = (ATR_prev × (period-1) + TR) / period
            self.atr = (self.atr * (self.period as f64 - 1.0) + tr) / self.period as f64;
        }
    }

    pub fn value(&self) -> f64 { self.atr }
}
```

### 2.2 Test

```rust
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn atr14_se_calcule_apres_14_bars() {
        let mut atr = Atr14::new();
        for i in 0..20 {
            let bar = BarInput { timestamp: i, open: 100.0, high: 101.0 + i as f64 * 0.1,
                                  low: 99.0, close: 100.5, volume: 100.0 };
            atr.update(&bar);
        }
        assert!(atr.value() > 0.0, "ATR doit être > 0 après 14+ bars");
        assert!(atr.value() < 5.0, "ATR doit être raisonnable (pas explosif)");
    }
}
```

### 2.3 Commit : `feat(smc/v12): ATR14 Wilder + test`

---

## Task 3: Pivots / Swings

### 3.1 Créer `smc/src/v12/pivots.rs`

Reproduit MODULE 1 Pine (lignes 314-412) : `ta.pivothigh(high, swingLength, swingLength)`.

**Logique Pine** : à la bar courante (clôturée), si la bar `bar_index - swingLength` a un high strictement supérieur aux `swingLength` bars avant ET après → c'est un pivot high. Même chose pour low.

```rust
pub struct PivotDetector {
    swing_length: usize,
    bars: Vec<BarInput>,           // historique (ring buffer potentiel)
    sh1: Option<f64>,              // dernier swing high
    sh2: Option<f64>,              // avant-dernier
    sl1: Option<f64>,              // dernier swing low
    sl2: Option<f64>,
    last_event: PivotEvent,
    // Pour l'anti-doublon BOS
    last_pivot_high_bar: Option<usize>,
    last_pivot_low_bar: Option<usize>,
}

impl PivotDetector {
    pub fn new(swing_length: usize) -> Self { ... }

    pub fn update(&mut self, bar: &BarInput) {
        self.bars.push(bar.clone());
        let n = self.bars.len();
        let sl = self.swing_length;

        // Il faut au moins 2*sl+1 bars pour détecter un pivot
        if n < 2 * sl + 1 { return; }

        // La bar pivot candidate est à n - sl - 1 (il y a sl bars après elle)
        let pivot_idx = n - sl - 1;
        let pivot_high = self.bars[pivot_idx].high;
        let pivot_low = self.bars[pivot_idx].low;

        // Vérifier pivot high : strictement > toutes les bars dans [pivot_idx-sl, pivot_idx-1] ET [pivot_idx+1, pivot_idx+sl]
        let is_ph = (1..=sl).all(|i| {
            pivot_high > self.bars[pivot_idx - i].high && pivot_high > self.bars[pivot_idx + i].high
        });
        // NOTE: Pine ta.pivothigh utilise >= (pas strict). Vérifier la spec.
        // Correction: Pine utilise >= pour les bars AVANT et > pour APRÈS (ou vice-versa selon version).
        // Le PseudoCode.md précise ce point. Utiliser la logique exacte du v12.

        let is_pl = (1..=sl).all(|i| {
            pivot_low < self.bars[pivot_idx - i].low && pivot_low < self.bars[pivot_idx + i].low
        });

        self.last_event = PivotEvent::default();
        if is_ph {
            self.sh2 = self.sh1;
            self.sh1 = Some(pivot_high);
            self.last_pivot_high_bar = Some(pivot_idx);
            self.last_event.is_pivot_high = true;
            self.last_event.pivot_high_price = Some(pivot_high);
            self.last_event.pivot_bar_index = Some(pivot_idx);
        }
        if is_pl {
            self.sl2 = self.sl1;
            self.sl1 = Some(pivot_low);
            self.last_pivot_low_bar = Some(pivot_idx);
            self.last_event.is_pivot_low = true;
            self.last_event.pivot_low_price = Some(pivot_low);
        }
    }

    pub fn sh1(&self) -> Option<f64> { self.sh1 }
    pub fn sl1(&self) -> Option<f64> { self.sl1 }
    pub fn sh2(&self) -> Option<f64> { self.sh2 }
    pub fn sl2(&self) -> Option<f64> { self.sl2 }
    pub fn last_event(&self) -> PivotEvent { self.last_event.clone() }
}
```

> ⚠ **Point critique à vérifier** : Pine `ta.pivothigh` utilise-t-il `>=` ou `>` pour la comparaison ? Le PseudoCode.md précise ce point (les pivots plateaux). L'implementer doit relire le PseudoCode §5bis ("Pivots plateaux") et reproduire EXACTEMENT.

### 3.2 Test : pivot détecté sur série connue

```rust
#[test]
fn pivot_high_detecte_sur_pattern_clair() {
    let mut det = PivotDetector::new(3);
    // Construire 7 bars: low-low-LOW(peak)-low-low-low-low
    // Le pivot high doit être à la bar 2 (index 2)
    let prices = [100.0, 100.0, 110.0, 100.0, 100.0, 100.0, 100.0];
    for (i, &p) in prices.iter().enumerate() {
        det.update(&BarInput { timestamp: i as i64, open: p, high: p+1.0, low: p-1.0, close: p, volume: 100.0 });
    }
    assert!(det.last_event().is_pivot_high, "Dout détecter un pivot high");
    assert_eq!(det.sh1(), Some(111.0)); // high de la bar 2 = 110+1
}
```

### 3.3 Commit : `feat(smc/v12): pivots/swings sh1/sl1/sh2/sl2 + test`

---

## Task 4: Structure (HH/HL/LH/LL + tendance)

### 4.1 Créer `smc/src/v12/structure.rs`

Reproduit MODULE 1 Pine (lignes 368-412).

```rust
pub struct StructureDetector {
    bull_count: u32,
    bear_count: u32,
    last_event: StructureEvent,
}

impl StructureDetector {
    pub fn new() -> Self { Self { bull_count: 0, bear_count: 0, last_event: StructureEvent::default() } }

    pub fn update(&mut self, _bar: &BarInput, pivot: &PivotEvent) {
        let mut ev = StructureEvent::default();

        if pivot.is_pivot_high {
            if let (Some(sh1), Some(sh2)) = (pivot.pivot_high_price, /* sh2 from pivots */) {
                if sh1 > sh2 { ev.is_hh = true; } else { ev.is_lh = true; }
            }
        }
        if pivot.is_pivot_low {
            // idem avec sl1/sl2 → HL ou LL
        }

        // Compteurs tendance (Pine lignes 373-380)
        if ev.is_hh || ev.is_hl { self.bull_count += 1; self.bear_count = self.bear_count.saturating_sub(1); }
        if ev.is_lh || ev.is_ll { self.bear_count += 1; self.bull_count = self.bull_count.saturating_sub(1); }

        ev.bull_count = self.bull_count;
        ev.bear_count = self.bear_count;
        ev.tendance_haussiere = self.bull_count >= 2;
        ev.tendance_baissiere = self.bear_count >= 2;
        self.last_event = ev;
    }

    pub fn tendance_haussiere(&self) -> bool { self.bull_count >= 2 }
    pub fn tendance_baissiere(&self) -> bool { self.bear_count >= 2 }
    pub fn last_event(&self) -> StructureEvent { self.last_event.clone() }
}
```

> **Note** : le struct detector a besoin de sh2/sl2 (du PivotDetector) pour comparer. Soit passer sh2/sl2 en paramètre, soit accéder via le moteur.

### 4.2 Test + commit : `feat(smc/v12): structure HH/HL/LH/LL + tendance`

---

## Task 5: BOS (Break of Structure)

### 5.1 Créer `smc/src/v12/bos.rs`

Reproduit MODULE 2 Pine (lignes 414-560).

**Logique Pine** : BOS haussier = `barstate.isconfirmed AND sh1 non-na AND close > sh1 AND close[1] <= sh1 AND (dernierSH1_sig na OR bar_pivot != dernierSH1_sig)`.

```rust
pub struct BosDetector {
    dernier_sh1_sig: Option<usize>,  // anti-doublon : bar du dernier pivot sh1 signalé
    dernier_bosh_level: Option<f64>,
    dernier_bosh_bar: Option<usize>,
    dernier_boss_level: Option<f64>,
    dernier_boss_bar: Option<usize>,
    last_event: BosEvent,
    last_close: Option<f64>,
}

impl BosDetector {
    pub fn new() -> Self { ... }

    pub fn update(&mut self, bar: &BarInput, pivots: &PivotDetector, structure: &StructureDetector) {
        let mut ev = BosEvent::default();
        let bar_idx = /* current bar index */;

        // BOS haussier (Pine ligne 437)
        if let (Some(sh1), Some(prev_close), Some(pivot_bar)) =
            (pivots.sh1(), self.last_close, pivots.last_pivot_high_bar()) {
            let anti_doublon = self.dernier_sh1_sig.map_or(true, |sig| sig != pivot_bar);
            if bar.close > sh1 && prev_close <= sh1 && anti_doublon {
                ev.bullish = true;
                ev.level = Some(sh1);
                ev.bar_index = Some(bar_idx);
                self.dernier_sh1_sig = Some(pivot_bar);
                self.dernier_bosh_level = Some(sh1);
                self.dernier_bosh_bar = Some(bar_idx);
            }
        }

        // BOS baissier (symétrique avec sl1)
        // ... same logic with sl1, close < sl1, prev_close >= sl1

        self.last_close = Some(bar.close);
        self.last_event = ev;
    }
}
```

### 5.2 Test : BOS détecté quand close casse sh1

```rust
#[test]
fn bos_haussier_detecte_quand_close_casse_sh1() {
    // Construire une séquence: bars créer un pivot high, puis une bar close au-dessus
    // Vérifier ev.bullish == true
}
```

### 5.3 Commit : `feat(smc/v12): BOS haussier/baissier + anti-doublon`

---

## Task 6: Test d'intégration sur 700 bars XAUUSD M15

### 6.1 Créer `smc/src/v12/tests.rs`

```rust
#[cfg(test)]
mod integration {
    use super::*;
    use std::io::BufRead;

    fn load_xauusd_m15() -> Vec<BarInput> {
        let file = std::fs::File::open("/mnt/IA/nautilus-smc-spike/xauusd_m15.csv").unwrap();
        let mut bars = Vec::new();
        for line in std::io::BufReader::new(file).lines().skip(1) {
            let l = line.unwrap();
            let f: Vec<&str> = l.split(',').collect();
            bars.push(BarInput {
                timestamp: f[0].parse().unwrap(),
                open: f[1].parse().unwrap(), high: f[2].parse().unwrap(),
                low: f[3].parse().unwrap(), close: f[4].parse().unwrap(),
                volume: f[5].parse().unwrap(),
            });
        }
        bars
    }

    #[test]
    fn engine_traite_700_bars_xauusd_sans_panic() {
        let bars = load_xauusd_m15();
        let mut engine = SmcV12Engine::new("XAUUSD", "M15");
        let mut pivot_count = 0;
        let mut bos_count = 0;
        for bar in &bars {
            let out = engine.update(bar);
            if out.pivot.is_pivot_high || out.pivot.is_pivot_low { pivot_count += 1; }
            if out.bos.bullish || out.bos.bearish { bos_count += 1; }
        }
        println!("700 bars XAUUSD M15 → {} pivots, {} BOS détectés", pivot_count, bos_count);
        assert!(pivot_count > 0, "Doit détecter des pivots sur 700 bars");
        assert!(bos_count > 0, "Doit détecter des BOS");
        // L'utilisateur compare ces comptes avec ce qu'il voit sur TradingView
    }
}
```

### 6.2 Commit : `test(smc/v12): intégration 700 bars XAUUSD M15`

---

## Task 7: Validation + rapport

- [ ] `cargo test -p smc` → tous verts.
- [ ] Le test d'intégration affiche les comptes de pivots + BOS → **l'utilisateur compare avec TradingView** (compter visuellement les labels HH/HL/BOS sur la même période XAUUSD M15).
- [ ] Rapport au propriétaire. Ne pas pousser.

---

## Self-Review

**Dépendances Pine couvertes** :
- Module 0 (calibration) ✓ — Task 1
- Module 1 (structure/pivots) ✓ — Tasks 3+4
- Module 2 (BOS) ✓ — Task 5
- ATR14 ✓ — Task 2

**Risques** :
- **Comparaison pivot `>=` vs `>`** : Pine `ta.pivothigh` peut utiliser `>=` pour un côté. L'implementer doit vérifier PseudoCode.md §5bis. Si les pivots divergent de TV, c'est le premier point à vérifier.
- **sh2/sl2 accessibilité** : le StructureDetector a besoin de sh2/sl2 du PivotDetector. Le moteur doit passer ces valeurs (via `pivots.sh2()`).
- **Index des bars** : le Pine utilise `bar_index` global. En Rust (replay), utiliser un compteur de bar (index dans le Vec). Cohérence importante pour l'anti-doublon BOS.
- **Coexistence** : l'ancien `smc::scorer` n'est PAS modifié. Le nouveau `smc::v12` est juste ajouté. Basculage ultérieur.
