//! Module commun de suivi de position — SL progressif + trailing stop + vente partielle.
//!
//! Utilisé par : Straddle moniteur, SMC (futur).
//! Rockets délègue en interne via `rockets_niveaux.rs` (API publique inchangée).
//!
//! Progression SL : TP1 → BE | TP2 → TP1 | après TP2 → trailing (peak − ATR × coeff).
//! Verdicts : Tp1Partiel | Tp2Partiel | TrailingTouche | Cloture { label } | Rien.

// ── Configuration ─────────────────────────────────────────────────────────────

/// Paramètres de gestion de position, configurables par stratégie.
#[derive(Debug, Clone)]
pub struct PositionConfig {
    /// Prix d'entrée dans la position.
    pub prix_entree: f64,
    /// Stop-loss initial (absolu).
    pub stop_loss: f64,
    /// TP1 — premier objectif (vente partielle si activée, sinon juste remontée SL).
    pub tp1: f64,
    /// TP2 — deuxième objectif (vente partielle + activation trailing).
    pub tp2: f64,
    /// ATR au moment du signal, utilisé pour calculer le trailing stop.
    pub atr: f64,
    /// Coefficient multiplicateur pour le trailing stop. `trailing = peak − atr × coeff`
    pub trailing_coeff: f64,
    /// `true` = vente partielle à TP1 et TP2. `false` = SL progressif uniquement, pas de sortie partielle.
    pub vente_partielle: bool,
}

// ── Verdicts ──────────────────────────────────────────────────────────────────

/// Résultat d'un tick de surveillance.
#[derive(Debug, Clone, PartialEq)]
pub enum Verdict {
    /// Rien à faire ce tick.
    Rien,
    /// TP1 franchi pour la 1ère fois — vente partielle (si `vente_partielle = true`).
    /// SL doit remonter au BE dans tous les cas.
    Tp1Partiel,
    /// TP2 franchi pour la 1ère fois — vente partielle (si `vente_partielle = true`).
    /// SL doit remonter à TP1. Trailing stop s'active.
    Tp2Partiel,
    /// Trailing stop touché après TP2 — clôture du solde.
    TrailingTouche { prix_cloture: f64 },
    /// SL effectif progressif touché — clôture totale.
    /// `label` : "sl" | "be" | "tp1" | "tp2" | "invalide"
    Cloture { label: &'static str, prix_cloture: f64 },
}

// ── SL effectif ───────────────────────────────────────────────────────────────

/// Calcule le SL effectif progressif selon le peak atteint.
///
/// | Peak atteint | SL effectif   | Label à la clôture |
/// |---|---|---|
/// | ≥ TP2        | TP1           | "tp1"              |
/// | ≥ TP1        | prix_entree   | "be"               |
/// | < TP1        | stop_loss     | "sl"               |
pub fn sl_effectif(cfg: &PositionConfig, peak: f64) -> (f64, &'static str) {
    if peak >= cfg.tp2 {
        (cfg.tp1, "tp1")
    } else if peak >= cfg.tp1 {
        (cfg.prix_entree, "be")
    } else {
        (cfg.stop_loss, "sl")
    }
}

// ── Calcul du verdict ─────────────────────────────────────────────────────────

/// Calcule le verdict pour un tick de surveillance.
///
/// # Paramètres
/// - `cfg`           : configuration de la position
/// - `prix`          : prix actuel
/// - `peak`          : nouveau peak (après mise à jour : `peak_precedent.max(prix)`)
/// - `peak_precedent`: peak avant ce tick (pour détecter les 1ers franchissements)
///
/// # Garanties
/// - TP1 et TP2 ne sont signalés qu'une seule fois (détection sur `peak_precedent < seuil && peak >= seuil`)
/// - Le trailing n'est actif qu'après TP2
/// - Le SL progressif est toujours vérifié en dernier
pub fn calculer_verdict(
    cfg: &PositionConfig,
    prix: f64,
    peak: f64,
    peak_precedent: f64,
) -> Verdict {
    // ── 1. Franchissements TP (vente partielle ou remontée SL) ────────────────
    // TP1 : premier franchissement uniquement
    if peak_precedent < cfg.tp1 && peak >= cfg.tp1 {
        return Verdict::Tp1Partiel;
    }
    // TP2 : premier franchissement uniquement (peak_precedent déjà ≥ TP1 sinon TP1 aurait été retourné)
    if peak_precedent >= cfg.tp1 && peak_precedent < cfg.tp2 && peak >= cfg.tp2 {
        return Verdict::Tp2Partiel;
    }

    // ── 2. Trailing stop (actif seulement après TP2) ──────────────────────────
    if peak >= cfg.tp2 {
        let trailing_stop = peak - cfg.atr * cfg.trailing_coeff;
        if prix < trailing_stop {
            return Verdict::TrailingTouche { prix_cloture: prix };
        }
    }

    // ── 3. SL effectif progressif ──────────────────────────────────────────────
    let (sl, label) = sl_effectif(cfg, peak);

    // Position jamais ouverte (peak n'a jamais dépassé l'entrée) → invalide
    if peak < cfg.prix_entree && prix <= sl {
        return Verdict::Cloture { label: "invalide", prix_cloture: prix };
    }

    if prix <= sl {
        return Verdict::Cloture { label, prix_cloture: prix };
    }

    Verdict::Rien
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg_base() -> PositionConfig {
        PositionConfig {
            prix_entree: 1.00,
            stop_loss:   0.90,
            tp1:         1.10,
            tp2:         1.20,
            atr:         0.05,
            trailing_coeff: 2.0,
            vente_partielle: true,
        }
    }

    // ── Rien à faire ─────────────────────────────────────────────────────────

    #[test]
    fn rien_si_prix_stable() {
        let cfg = cfg_base();
        // Prix entre entrée et TP1, peak idem
        assert_eq!(calculer_verdict(&cfg, 1.05, 1.05, 1.05), Verdict::Rien);
    }

    // ── Franchissements TP ───────────────────────────────────────────────────

    #[test]
    fn tp1_franchi_premiere_fois() {
        let cfg = cfg_base();
        // peak passe de 1.09 à 1.10 → TP1 franchi
        assert_eq!(calculer_verdict(&cfg, 1.10, 1.10, 1.09), Verdict::Tp1Partiel);
    }

    #[test]
    fn tp1_non_double_signal() {
        let cfg = cfg_base();
        // peak_precedent déjà ≥ TP1 → TP1 déjà signalé, on ne le signale plus
        let v = calculer_verdict(&cfg, 1.15, 1.15, 1.12);
        // peak_precedent ≥ tp1 && < tp2 → c'est tp2 qui n'est pas encore atteint → Rien ou TP2
        assert_ne!(v, Verdict::Tp1Partiel);
    }

    #[test]
    fn tp2_franchi_premiere_fois() {
        let cfg = cfg_base();
        // peak_precedent entre TP1 et TP2, peak ≥ TP2
        assert_eq!(
            calculer_verdict(&cfg, 1.20, 1.20, 1.15),
            Verdict::Tp2Partiel
        );
    }

    #[test]
    fn tp2_non_double_signal() {
        let cfg = cfg_base();
        // peak_precedent déjà ≥ TP2 → trailing actif, vérifier que TP2 n'est plus signalé
        let v = calculer_verdict(&cfg, 1.22, 1.22, 1.21);
        // trailing_stop = 1.22 - 0.05*2.0 = 1.12 → prix 1.22 > 1.12 → Rien
        assert_ne!(v, Verdict::Tp2Partiel);
        assert_eq!(v, Verdict::Rien);
    }

    // ── Trailing stop ─────────────────────────────────────────────────────────

    #[test]
    fn trailing_non_actif_avant_tp2() {
        let cfg = cfg_base();
        // peak entre TP1 et TP2, prix chute — trailing non actif
        // peak = 1.15, prix = 1.00 → SL effectif = prix_entree (BE) = 1.00 → touché → Cloture "be"
        let v = calculer_verdict(&cfg, 1.00, 1.15, 1.12);
        assert_eq!(v, Verdict::Cloture { label: "be", prix_cloture: 1.00 });
    }

    #[test]
    fn trailing_actif_apres_tp2_non_touche() {
        let cfg = cfg_base();
        // peak = 1.25, prix = 1.20 → trailing_stop = 1.25 - 0.1 = 1.15 → prix 1.20 > 1.15 → Rien
        assert_eq!(calculer_verdict(&cfg, 1.20, 1.25, 1.24), Verdict::Rien);
    }

    #[test]
    fn trailing_actif_apres_tp2_touche() {
        let cfg = cfg_base();
        // peak = 1.30, trailing_stop = 1.30 - 0.05*2.0 = 1.20 → prix 1.19 < 1.20 → TrailingTouche
        assert_eq!(
            calculer_verdict(&cfg, 1.19, 1.30, 1.29),
            Verdict::TrailingTouche { prix_cloture: 1.19 }
        );
    }

    // ── SL progressif ─────────────────────────────────────────────────────────

    #[test]
    fn sl_original_si_jamais_tp1() {
        let cfg = cfg_base();
        // peak sous TP1, prix touche SL
        assert_eq!(
            calculer_verdict(&cfg, 0.90, 1.05, 1.04),
            Verdict::Cloture { label: "sl", prix_cloture: 0.90 }
        );
    }

    #[test]
    fn be_apres_tp1() {
        let cfg = cfg_base();
        // peak atteint TP1 puis prix retombe au BE
        assert_eq!(
            calculer_verdict(&cfg, 1.00, 1.12, 1.11),
            Verdict::Cloture { label: "be", prix_cloture: 1.00 }
        );
    }

    #[test]
    fn sl_tp1_apres_tp2() {
        let cfg = cfg_base();
        // TP2 déjà signalé (peak_precedent >= tp2)
        // peak = 1.25, prix = 1.20 → trailing_stop = 1.25 - 0.10 = 1.15 → prix 1.20 > 1.15 → Rien
        assert_eq!(calculer_verdict(&cfg, 1.20, 1.25, 1.24), Verdict::Rien);
    }

    #[test]
    fn sl_tp1_effectif_apres_tp2() {
        let cfg = cfg_base();
        // TP2 déjà signalé (peak_precedent >= tp2)
        // peak = 1.25, trailing_stop = 1.25 - 0.10 = 1.15 → prix 1.09 < 1.15 → TrailingTouche
        assert_eq!(
            calculer_verdict(&cfg, 1.09, 1.25, 1.24),
            Verdict::TrailingTouche { prix_cloture: 1.09 }
        );
    }

    #[test]
    fn invalide_si_jamais_ouvert() {
        let cfg = cfg_base();
        // peak n'a jamais dépassé prix_entree → invalide
        assert_eq!(
            calculer_verdict(&cfg, 0.89, 0.99, 0.98),
            Verdict::Cloture { label: "invalide", prix_cloture: 0.89 }
        );
    }

    // ── sl_effectif ───────────────────────────────────────────────────────────

    #[test]
    fn sl_effectif_avant_tp1() {
        let cfg = cfg_base();
        let (sl, label) = sl_effectif(&cfg, 1.05);
        assert_eq!(sl, 0.90);
        assert_eq!(label, "sl");
    }

    #[test]
    fn sl_effectif_apres_tp1() {
        let cfg = cfg_base();
        let (sl, label) = sl_effectif(&cfg, 1.12);
        assert_eq!(sl, 1.00);
        assert_eq!(label, "be");
    }

    #[test]
    fn sl_effectif_apres_tp2() {
        let cfg = cfg_base();
        let (sl, label) = sl_effectif(&cfg, 1.25);
        assert_eq!(sl, 1.10);
        assert_eq!(label, "tp1");
    }
}
