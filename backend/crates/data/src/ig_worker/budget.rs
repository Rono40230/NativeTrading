//! Budget de data points IG REST, fenêtre glissante 7 jours.
//!
//! IG limite le REST historique à ~10 000 data points/semaine par compte
//! (1 bougie renvoyée = 1 point, même si déjà en DB) ; au-delà l'erreur
//! `exceeded-account-historical-data` bloque TOUTE l'application. Le worker
//! s'auto-limite donc à un budget inférieur et se met en pause (WARN unique)
//! quand il est épuisé, reprenant au fil de l'expiration de la fenêtre.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Fenêtre glissante du quota IG (7 jours).
const FENETRE_QUOTA: Duration = Duration::from_secs(7 * 86_400);
/// Part du quota hebdo laissée au worker — le reste reste disponible pour
/// les fetchs à la demande du reste de l'app (charts, pip_updater…).
const BUDGET_HEBDO_POINTS: usize = 8_000;

/// Compteur de points consommés sur une fenêtre glissante de 7 jours.
pub(super) struct BudgetQuota {
    /// Historique (instant, points) des consommations dans la fenêtre.
    consos: VecDeque<(Instant, usize)>,
    /// Somme des points encore dans la fenêtre.
    total: usize,
    /// true si l'épuisement courant a déjà été loggué (anti-spam).
    epuise_logge: bool,
}

impl BudgetQuota {
    pub(super) fn new() -> Self {
        Self {
            consos: VecDeque::new(),
            total: 0,
            epuise_logge: false,
        }
    }

    /// Expire les consommations sorties de la fenêtre et retourne le total.
    pub(super) fn consomme(&mut self) -> usize {
        let maintenant = Instant::now();
        while let Some((instant, points)) = self.consos.front().copied() {
            if maintenant.duration_since(instant) >= FENETRE_QUOTA {
                self.consos.pop_front();
                self.total = self.total.saturating_sub(points);
            } else {
                break;
            }
        }
        self.total
    }

    /// Autorise une requête dont le coût maximal est `cout_max` bougies ?
    pub(super) fn autorise(&mut self, cout_max: usize) -> bool {
        let ok = self.consomme().saturating_add(cout_max) <= BUDGET_HEBDO_POINTS;
        if !ok && !self.epuise_logge {
            tracing::warn!(
                "IG worker: budget hebdo de data points épuisé ({}/{}) — pause des fetchs, reprise au fil de la fenêtre glissante",
                self.total,
                BUDGET_HEBDO_POINTS
            );
            self.epuise_logge = true;
        }
        ok
    }

    /// Consigne `points` bougies renvoyées par IG.
    pub(super) fn consigner(&mut self, points: usize) {
        if points == 0 {
            return;
        }
        self.consos.push_back((Instant::now(), points));
        self.total += points;
        self.epuise_logge = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn budget_quota_autorise_puis_bloque() {
        let mut budget = BudgetQuota::new();
        assert!(budget.autorise(200), "budget neuf disponible");
        budget.consigner(BUDGET_HEBDO_POINTS - 2);
        assert!(budget.autorise(2), "il reste juste la place d'un update");
        assert!(!budget.autorise(200), "plus la place d'un backfill");
        budget.consigner(2);
        assert!(!budget.autorise(2), "budget épuisé → refus");
        assert_eq!(budget.consomme(), BUDGET_HEBDO_POINTS);
        budget.consigner(0); // no-op
        assert_eq!(budget.consomme(), BUDGET_HEBDO_POINTS);
    }
}
