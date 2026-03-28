use super::{Signal, Strategy};
use common::{Candle, Direction, Result};
use indicators::calculer_atr;
use ml::PipelineML;

/// Paramètres Straddle — source unique : DB (partage live + backtest).
pub use db::strategies_params::StraddleParams;

/// Stratégie Straddle — volatilité extrême + IA indécise
///
/// Déclencheur : ATR > 150% de sa moyenne ET confiance ML < 60%
/// Exécution : positions opposées simultanées (LONG + SHORT)
/// TP : ATR × params.tp_mult_1 | SL : ATR × params.sl_mult
/// Risk : 1% par direction (2% total)
pub struct StraddleStrategy {
    pub pipeline_ml: Option<PipelineML>,
    pub params: StraddleParams,
}

impl StraddleStrategy {
    pub fn new() -> Self {
        Self {
            pipeline_ml: None,
            params: StraddleParams::default(),
        }
    }

    pub fn avec_params(params: StraddleParams) -> Self {
        Self {
            pipeline_ml: None,
            params,
        }
    }

    pub fn avec_ml(pipeline: PipelineML) -> Self {
        Self {
            pipeline_ml: Some(pipeline),
            params: StraddleParams::default(),
        }
    }
}

impl Default for StraddleStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl Strategy for StraddleStrategy {
    fn analyze(&self, bougies: &[Candle]) -> Result<Option<Signal>> {
        if bougies.len() < 30 {
            return Ok(None);
        }

        let atr = calculer_atr(bougies, 14);
        let n = bougies.len();
        let atr_courant = atr[n - 1];

        if atr_courant.is_nan() {
            return Ok(None);
        }

        // Moyenne ATR sur les 14 dernières valeurs non-NaN
        let atr_valides: Vec<f64> = atr[n.saturating_sub(14)..n]
            .iter()
            .copied()
            .filter(|v| !v.is_nan())
            .collect();

        if atr_valides.is_empty() {
            return Ok(None);
        }
        let atr_moyen = atr_valides.iter().sum::<f64>() / atr_valides.len() as f64;
        let ratio_atr = atr_courant / atr_moyen.max(1e-10);

        // Condition 1 : volatilité extrême (ATR > seuil × sa moyenne)
        if ratio_atr <= self.params.atr_seuil {
            return Ok(None);
        }

        // Condition 2 : IA indécise (si modèle disponible)
        let ia_indecise = match &self.pipeline_ml {
            Some(pipeline) if pipeline.est_pret() => {
                match pipeline.predire(bougies) {
                    Ok(pred) => !pred.est_confiant, // confiance < 60% = indécis
                    Err(e) => {
                        tracing::warn!("Straddle: erreur ML, indécision assumée: {}", e);
                        true // ML en erreur → indécis par défaut
                    }
                }
            }
            // Sans ML (backtest ou modèle non encore entraîné) → indécis par défaut
            _ => true,
        };

        if !ia_indecise {
            return Ok(None);
        }

        let prix_entree = bougies[n - 1].close;
        let tp = prix_entree + atr_courant * self.params.tp_mult_1;
        let sl = prix_entree - atr_courant * self.params.sl_mult;

        tracing::info!(
            "Signal STRADDLE: prix={:.2} ATR={:.4} ratio={:.2}x TP={:.2} SL={:.2}",
            prix_entree,
            atr_courant,
            ratio_atr,
            tp,
            sl
        );

        let tp2 = prix_entree + atr_courant * self.params.tp_mult_2;
        let tp3 = prix_entree + atr_courant * self.params.tp_mult_3;

        Ok(Some(Signal {
            direction: Direction::Both,
            confiance: ratio_atr.min(3.0) / 3.0, // normalisé 0-1
            prix_entree,
            stop_loss: sl,
            take_profit: tp,
            take_profit_2: Some(tp2),
            take_profit_3: Some(tp3),
        }))
    }
}

/// Straddle filtré sur le pic de volatilité précis `timing_optimal ± fenetre_min`.
/// Seules les bougies dont le timestamp UTC tombe dans cette fenêtre minute sont
/// éligibles à générer un signal. Jour optionnel (0=Lundi…4=Vendredi).
pub struct StraddleCreneauStrategy {
    inner: StraddleStrategy,
    /// Heure + minute du pic (ex: (14, 32) pour "14:32")
    pub timing_heure: u32,
    pub timing_minute: u32,
    /// Fenêtre de tolérance en minutes de part et d'autre du timing
    pub fenetre_min: u32,
    pub jour_semaine: Option<i64>,
}

impl StraddleCreneauStrategy {
    /// `timing` : "HH:MM" — pic de volatilité exact détecté par l'analyse de précision.
    /// `fenetre_min` : tolérance en minutes (défaut recommandé : 5 à 15 min).
    pub fn new(timing: &str, fenetre_min: u32, jour_semaine: Option<i64>) -> Self {
        let (h, m) = parse_timing_hm(timing);
        Self {
            inner: StraddleStrategy::new(),
            timing_heure: h,
            timing_minute: m,
            fenetre_min,
            jour_semaine,
        }
    }

    pub fn avec_params(
        timing: &str,
        fenetre_min: u32,
        jour_semaine: Option<i64>,
        params: StraddleParams,
    ) -> Self {
        let (h, m) = parse_timing_hm(timing);
        Self {
            inner: StraddleStrategy::avec_params(params),
            timing_heure: h,
            timing_minute: m,
            fenetre_min,
            jour_semaine,
        }
    }
}

/// Parse "HH:MM" en (heure, minute) — retourne (0, 0) si invalide.
fn parse_timing_hm(s: &str) -> (u32, u32) {
    let mut it = s.splitn(2, ':');
    let h = it.next().and_then(|v| v.parse().ok()).unwrap_or(0);
    let m = it.next().and_then(|v| v.parse().ok()).unwrap_or(0);
    (h, m)
}

impl Strategy for StraddleCreneauStrategy {
    fn analyze(&self, bougies: &[Candle]) -> Result<Option<Signal>> {
        use chrono::{Datelike, Timelike};
        if let Some(last) = bougies.last() {
            // Filtrage par jour
            if let Some(jour) = self.jour_semaine {
                if last.timestamp.weekday().num_days_from_monday() as i64 != jour {
                    return Ok(None);
                }
            }
            // Contrôle sur le timing précis ± fenetre_min
            let h = last.timestamp.hour();
            let m = last.timestamp.minute();
            let ts_min = h * 60 + m;
            let cible_min = self.timing_heure * 60 + self.timing_minute;
            let delta = (ts_min as i64 - cible_min as i64).unsigned_abs() as u32;
            if delta > self.fenetre_min {
                return Ok(None);
            }
        }
        self.inner.analyze(bougies)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use common::Candle;

    fn bougie_plate(c: f64) -> Candle {
        Candle {
            timestamp: Utc::now(),
            open: c,
            high: c + 1.0,
            low: c - 1.0,
            close: c,
            volume: 1000.0,
        }
    }

    fn bougie_volatile(c: f64, range: f64) -> Candle {
        Candle {
            timestamp: Utc::now(),
            open: c,
            high: c + range,
            low: c - range,
            close: c,
            volume: 5000.0,
        }
    }

    #[test]
    fn analyse_retourne_none_si_moins_de_30_bougies() {
        let strat = StraddleStrategy::new();
        let bougies: Vec<Candle> = (1..=25).map(|i| bougie_plate(i as f64 * 100.0)).collect();
        assert!(strat.analyze(&bougies).unwrap().is_none());
    }

    #[test]
    fn analyse_retourne_none_si_atr_plat() {
        let strat = StraddleStrategy::new();
        // Toutes les bougies identiques → ATR ratio = 1.0 ≤ seuil 1.5
        let bougies: Vec<Candle> = (0..35).map(|_| bougie_plate(100.0)).collect();
        assert!(strat.analyze(&bougies).unwrap().is_none());
    }

    #[test]
    fn analyse_retourne_signal_sans_ml() {
        // Sans modèle ML, la stratégie considère l'IA indécise par défaut et doit émettre un signal
        // si la volatilité est suffisamment extrême (ATR ratio > seuil).
        let strat = StraddleStrategy::new(); // pipeline_ml = None
        let mut bougies: Vec<Candle> = (0..34).map(|_| bougie_plate(100.0)).collect();
        bougies.push(bougie_volatile(100.0, 150.0));
        // Comportement attendu : signal émis (indécision assumée sans ML)
        assert!(strat.analyze(&bougies).unwrap().is_some());
    }
}
