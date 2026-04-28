use core::f64;

/// Paramètres de gestion de position pour le moteur centralisé.
#[derive(Debug, Clone)]
pub struct PositionConfig {
    pub is_long: bool,
    pub prix_entree: f64,
    pub stop_loss: f64,
    pub tp1: f64,
    pub tp2: f64,
    pub atr: f64,
    pub trailing_coeff: f64,
    pub vente_partielle_active: bool,
    pub pct_cloture_tp1: f64,
    pub pct_cloture_tp2: f64,
}

impl PositionConfig {
    /// Calcule le risque 1R pour cette position
    pub fn risque_unitaire(&self) -> f64 {
        (self.prix_entree - self.stop_loss).abs().max(1e-9)
    }

    /// Normalise un prix pour que la hausse soit toujours positive en termes de performance.
    /// Pour LONG, renvoie le prix. Pour SHORT, inverse le sens (symétrie par rapport à l'entrée).
    pub fn normalize(&self, prix: f64) -> f64 {
        if self.is_long { prix } else { self.prix_entree + (self.prix_entree - prix) }
    }

    /// Extrait la valeur de R pour un prix de clôture donné
    pub fn calculer_r(&self, prix_cloture: f64) -> f64 {
        let diff = if self.is_long {
            prix_cloture - self.prix_entree
        } else {
            self.prix_entree - prix_cloture
        };
        diff / self.risque_unitaire()
    }
}

/// Résultat d'un tick de surveillance par le moteur.
#[derive(Debug, Clone, PartialEq)]
pub enum Verdict {
    Rien,
    Tp1Partiel { r_encaisse: f64, pct_vendu: f64 },
    Tp2Partiel { r_encaisse: f64, pct_vendu: f64 },
    ClotureTotale { label: String, r_final: f64, prix_cloture: f64 },
}

/// Calcule le SL effectif progressif (dans l'espace normalisé LONG).
/// 
/// Règles:
/// Peak < TP1       -> SL = Initial
/// TP1 <= Peak < TP2 -> SL = BE (Prix entrée)
/// Peak >= TP2      -> SL = TP1
fn sl_effectif_norm(cfg: &PositionConfig, peak_norm: f64) -> (f64, &'static str) {
    let tp1_norm = cfg.normalize(cfg.tp1);
    let tp2_norm = cfg.normalize(cfg.tp2);
    let sl_norm = cfg.normalize(cfg.stop_loss);

    if peak_norm >= tp2_norm {
        (tp1_norm, "tp1")
    } else if peak_norm >= tp1_norm {
        (cfg.prix_entree, "be")
    } else {
        (sl_norm, "sl")
    }
}

/// Évalue un tick de prix par le moteur de trade universel.
///
/// # Arguments
/// * `cfg` - Configuration de la position
/// * `prix` - Prix actuel de l'actif
/// * `peak` - Meilleur prix atteint depuis l'ouverture (Peak max pour LONG, Trough min pour SHORT)
/// * `peak_precedent` - Le peak au tick précédent, essentiel pour détecter de franchissement
pub fn calculer_verdict(
    cfg: &PositionConfig,
    prix: f64,
    peak: f64,
    peak_precedent: f64,
) -> Verdict {
    let prix_n = cfg.normalize(prix);
    let peak_n = cfg.normalize(peak);
    let peak_prec_n = cfg.normalize(peak_precedent);
    
    let tp1_n = cfg.normalize(cfg.tp1);
    let tp2_n = cfg.normalize(cfg.tp2);

    // 1. Franchissements TP
    if peak_prec_n < tp1_n && peak_n >= tp1_n {
        let pct = if cfg.vente_partielle_active { cfg.pct_cloture_tp1 } else { 0.0 };
        return Verdict::Tp1Partiel { 
            r_encaisse: cfg.calculer_r(cfg.tp1),
            pct_vendu: pct,
        };
    }

    if peak_prec_n >= tp1_n && peak_prec_n < tp2_n && peak_n >= tp2_n {
        let pct = if cfg.vente_partielle_active { cfg.pct_cloture_tp2 } else { 0.0 };
        return Verdict::Tp2Partiel { 
            r_encaisse: cfg.calculer_r(cfg.tp2),
            pct_vendu: pct,
        };
    }

    // 2. Trailing Stop (actif seulement après TP2)
    if peak_n >= tp2_n {
        let trailing_stop_n = peak_n - cfg.atr * cfg.trailing_coeff;
        if prix_n < trailing_stop_n {
            return Verdict::ClotureTotale { 
                label: "trailing".into(), 
                r_final: cfg.calculer_r(prix), 
                prix_cloture: prix 
            };
        }
    }

    // 3. SL Effectif Progressif
    let (sl_n, label) = sl_effectif_norm(cfg, peak_n);

    // Clôture Invalide (par ex: prix ne déclenche jamais d'entrée ou SL touché instantanément)
    if peak_n < cfg.prix_entree && prix_n <= sl_n {
        return Verdict::ClotureTotale { 
            label: "invalide".into(), 
            r_final: cfg.calculer_r(prix), 
            prix_cloture: prix 
        };
    }

    if prix_n <= sl_n {
        return Verdict::ClotureTotale { 
            label: label.into(), 
            r_final: cfg.calculer_r(prix), 
            prix_cloture: prix 
        };
    }

    Verdict::Rien
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg_long() -> PositionConfig {
        PositionConfig {
            is_long: true,
            prix_entree: 100.0,
            stop_loss: 90.0,
            tp1: 110.0,
            tp2: 120.0,
            atr: 5.0,
            trailing_coeff: 2.0, // 10 de trailing
            vente_partielle_active: true,
            pct_cloture_tp1: 0.33,
            pct_cloture_tp2: 0.33,
        }
    }

    fn cfg_short() -> PositionConfig {
        PositionConfig {
            is_long: false,
            prix_entree: 100.0,
            stop_loss: 110.0,
            tp1: 90.0,
            tp2: 80.0,
            atr: 5.0,
            trailing_coeff: 2.0, // 10 de trailing
            vente_partielle_active: true,
            pct_cloture_tp1: 0.33,
            pct_cloture_tp2: 0.33,
        }
    }

    #[test]
    fn risque_unitaire() {
        assert_eq!(cfg_long().risque_unitaire(), 10.0);
        assert_eq!(cfg_short().risque_unitaire(), 10.0);
    }

    #[test]
    fn tp1_long() {
        let mut cfg = cfg_long();
        let v = calculer_verdict(&cfg, 110.0, 110.0, 105.0);
        assert_eq!(v, Verdict::Tp1Partiel { r_encaisse: 1.0, pct_vendu: 0.33 });
    }

    #[test]
    fn tp2_short() {
        let mut cfg = cfg_short();
        let v = calculer_verdict(&cfg, 80.0, 80.0, 85.0); // Peak passe a 80 (qui est tp2)
        assert_eq!(v, Verdict::Tp2Partiel { r_encaisse: 2.0, pct_vendu: 0.33 });
    }

    #[test]
    fn ts_long() {
        let cfg = cfg_long();
        // Peak = 130 (donc trailing a 120), prix actu = 115 -> Trailing touché
        let v = calculer_verdict(&cfg, 115.0, 130.0, 130.0);
        assert_eq!(v, Verdict::ClotureTotale { label: "trailing".into(), r_final: 1.5, prix_cloture: 115.0 });
    }
}
