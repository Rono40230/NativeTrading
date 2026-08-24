//! Gestion d'une position rocket — cycle de vie du Journal de Trading
//! (définition canonique, validée 24/08) :
//!   entrée stop-limit au pivot → invalidation (−1R) OU
//!   R1 atteint → vendre 50 % + trailing % → sortie sur trailing.

use crate::types::ParamsRockets;

/// Position rocket vivante (état journalisé).
#[derive(Debug, Clone)]
pub struct PositionRocket {
    pub symbole: String,
    pub entree: f64,
    /// Invalidation initiale (= 1R sous l'entrée).
    pub stop: f64,
    /// R1 = entrée + 1R.
    pub r1: f64,
    /// Vrai après la vente de 50 % (neutralisation).
    pub neutralise: bool,
    /// Trailing stop actif (posé à la neutralisation, en prix absolu).
    pub trailing: Option<f64>,
}

impl PositionRocket {
    pub fn nouvelle(symbole: &str, entree: f64, stop: f64) -> Self {
        Self {
            symbole: symbole.to_string(),
            entree,
            stop,
            r1: entree + (entree - stop),
            neutralise: false,
            trailing: None,
        }
    }

    /// Risque unitaire (1R).
    pub fn risque(&self) -> f64 {
        self.entree - self.stop
    }
}

/// Verdict final d'une rocket (écrit en base avec son R réalisé).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerdictRocket {
    /// Invalidation touchée avant R1 : -1R.
    Sl,
    /// Sortie sur trailing après neutralisation : R mixte (50 % à 1R +
    /// 50 % à la sortie) — le R exact est porté par l'action.
    Ts,
    /// Sortie sur trailing SANS neutralisation préalable (cas bord).
    TsSec,
}

/// Action produite par un pas de gestion sur une bougie D1 close.
#[derive(Debug, Clone, PartialEq)]
pub enum ActionRocket {
    Rien,
    /// R1 atteint : vendre 50 % + poser le trailing (prix du jour).
    Neutraliser { prix: f64, trailing: f64 },
    /// Clôture complète.
    Cloturer { prix: f64, verdict: VerdictRocket, r_realise: f64 },
}

/// Un pas de gestion sur bougie D1 confirmée (high/low/close du jour).
/// Ordre journal : invalidation d'abord, puis R1, puis trailing.
pub fn pas_gestion(p: &mut PositionRocket, high: f64, low: f64, close: f64, params: &ParamsRockets) -> ActionRocket {
    let risque = p.risque();

    // 1. Invalidation (le stop initial tant qu'on n'est pas neutralisé).
    let stop_courant = p.trailing.unwrap_or(p.stop);
    if low <= stop_courant {
        if !p.neutralise {
            // Sortie sèche : toute la position à l'invalidation.
            let r = -1.0;
            return ActionRocket::Cloturer { prix: stop_courant, verdict: VerdictRocket::Sl, r_realise: r };
        }
        // Neutralisé : 50 % déjà vendus à R1 (+0,5 R), solde au trailing.
        let r_solde = (stop_courant - p.entree) / risque;
        let r = 0.5 * 1.0 + 0.5 * r_solde;
        return ActionRocket::Cloturer { prix: stop_courant, verdict: VerdictRocket::Ts, r_realise: r };
    }

    // 2. R1 atteint et pas encore neutralisé → vendre 50 % + trailing %.
    if !p.neutralise && high >= p.r1 {
        let prix_r1 = p.r1;
        let trailing = close * (1.0 - params.trailing_pct / 100.0);
        p.neutralise = true;
        p.trailing = Some(trailing.max(p.stop));
        return ActionRocket::Neutraliser { prix: prix_r1, trailing: p.trailing.unwrap_or(trailing) };
    }

    // 3. Trailing : remonte avec le prix, jamais vers l'arrière.
    if p.neutralise {
        if let Some(t) = p.trailing {
            let cible = close * (1.0 - params.trailing_pct / 100.0);
            if cible > t {
                p.trailing = Some(cible);
            }
        }
    }

    ActionRocket::Rien
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ProfilRisque;

    fn params() -> ParamsRockets {
        ParamsRockets { profil: ProfilRisque::Neutre, trailing_pct: 5.0, ..Default::default() }
    }

    fn position() -> PositionRocket {
        // Entrée 100, stop 92 → 1R = 8, R1 = 108.
        PositionRocket::nouvelle("TEST", 100.0, 92.0)
    }

    #[test]
    fn invalidation_avant_r1_moins_1r() {
        let mut p = position();
        let a = pas_gestion(&mut p, 99.0, 91.5, 93.0, &params());
        match a {
            ActionRocket::Cloturer { verdict, r_realise, prix } => {
                assert_eq!(verdict, VerdictRocket::Sl);
                assert!((r_realise + 1.0).abs() < 1e-9);
                assert!((prix - 92.0).abs() < 1e-9);
            }
            autre => panic!("clôture SL attendue : {:?}", autre),
        }
    }

    #[test]
    fn r1_neutralise_puis_trailing_sort() {
        let mut p = position();
        // Jour 1 : R1 touché (high 110), close 108 → trailing = 108×0,95.
        let a = pas_gestion(&mut p, 110.0, 105.0, 108.0, &params());
        match &a {
            ActionRocket::Neutraliser { prix, trailing } => {
                assert!((prix - 108.0).abs() < 1e-9);
                assert!((trailing - 108.0 * 0.95).abs() < 1e-9, "trailing 5 % de la clôture");
            }
            autre => panic!("neutralisation attendue : {:?}", autre),
        }
        assert!(p.neutralise);
        // Jour 2 : nouveau plus haut 115, close 114 → trailing remonte.
        pas_gestion(&mut p, 115.0, 110.0, 114.0, &params());
        assert!((p.trailing.unwrap_or(0.0) - 114.0 * 0.95).abs() < 1e-9, "jamais vers l'arrière");
        // Jour 3 : chute sous le trailing (108,3) → sortie mixte.
        let t = p.trailing.unwrap_or(0.0);
        let a = pas_gestion(&mut p, 112.0, t - 1.0, 110.0, &params());
        match a {
            ActionRocket::Cloturer { verdict: VerdictRocket::Ts, r_realise, .. } => {
                let attendu = 0.5 + 0.5 * (t - 100.0) / 8.0;
                assert!((r_realise - attendu).abs() < 1e-9, "R = {:.3}", r_realise);
            }
            autre => panic!("clôture TS attendue : {:?}", autre),
        }
    }

    #[test]
    fn trailing_ne_descend_jamais() {
        let mut p = position();
        pas_gestion(&mut p, 110.0, 105.0, 108.0, &params()); // neutralise, T=102,6
        let t_avant = p.trailing.unwrap_or(0.0);
        pas_gestion(&mut p, 107.0, 103.0, 104.0, &params()); // baisse
        assert!(p.trailing.unwrap_or(0.0) >= t_avant, "le trailing ne recule pas");
    }
}
