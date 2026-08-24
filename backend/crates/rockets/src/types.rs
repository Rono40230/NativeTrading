//! Types Rockets — paramètres (définition canonique + carte paramètres).

/// Profil de risque — mêmes pourcentages que le Journal de Trading
/// (choix du propriétaire, PAS déduit du classement — décision 24/08).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfilRisque {
    PeuRisque,
    Neutre,
    Risque,
}

impl ProfilRisque {
    /// Fraction du capital risquée par rocket (actions/crypto ; ETF à venir
    /// avec MT5 : 2/3/4 %).
    pub fn fraction(self) -> f64 {
        match self {
            ProfilRisque::PeuRisque => 0.005,
            ProfilRisque::Neutre => 0.01,
            ProfilRisque::Risque => 0.02,
        }
    }
    pub fn libelle(self) -> &'static str {
        match self {
            ProfilRisque::PeuRisque => "Peu Risqué",
            ProfilRisque::Neutre => "Neutre",
            ProfilRisque::Risque => "Risqué",
        }
    }
}

/// Paramètres de la stratégie (carte Paramètres › Rockets).
#[derive(Debug, Clone)]
pub struct ParamsRockets {
    /// Profil de risque par défaut (choix propriétaire, comme au journal).
    pub profil: ProfilRisque,
    /// Plafond de position en % du capital (montant — canonique : 5).
    pub plafond_position_pct: f64,
    /// Trailing stop en % du prix, posé à la neutralisation R1 (défaut 5,
    /// réglable — décision 24/08).
    pub trailing_pct: f64,
    /// Volume au pivot ≥ × de la MM50 (canonique : 1,5 = 150 %).
    pub volume_pivot_mult: f64,
    /// Cassure décisive minimale au-delà du pivot en % (canonique : 3).
    pub cassure_min_pct: f64,
    /// Seuil de conviction du ranker — sous ce seuil, la cassure est
    /// écartée (0 = avis purement informatif). Défaut 40.
    pub conviction_min: i64,
}

impl Default for ParamsRockets {
    fn default() -> Self {
        Self {
            profil: ProfilRisque::Neutre,
            plafond_position_pct: 5.0,
            trailing_pct: 5.0,
            volume_pivot_mult: 1.5,
            cassure_min_pct: 3.0,
            conviction_min: 40,
        }
    }
}
