//! Trade struct + enums — cycle de vie des trades (Pine `stBull*`/`stBear*` arrays).
//!
//! Reproduit les arrays parallèles Pine (lignes 2387-2419) sous une seule struct.
//! Un `Trade` naît à la génération du signal (v11 OB ou BSZones), est REMPLI quand
//! le prix revient toucher l'entrée (retest, modèle "Retest (limite)"), puis évolue
//! via le lifecycle (SL → BE → TP2-SL → TP3 → TP1 → TP2) jusqu'à clôture.

use super::types::BarInput;

/// Sens du trade. `Buy` = trade haussier (stBull*), `Sell` = baissier (stBear*).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Buy,
    Sell,
}

/// Source du trade (Pine `stBullSource`/`stBearSource`).
/// `Ob` = moteur v11 (Order Blocks), `BsZones` = moteur BSZones (Sweep→Disp→OB).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TradeSource {
    /// `0` Pine — Order Block (lifecycle `_obIdx` actif, scoreDeg possible).
    Ob,
    /// `1` Pine — BSZones (court-circuite `_obIdx`).
    BsZones,
}

/// Cause de clôture d'un trade (déclencheur Pine exact, lignes 3870-3906 / 4036-4071).
///
/// Le verdict utilisateur (TP1/TP2/TP3/SL/BE/Expire) est dérivé du meilleur TP
/// réellement touché (`best_milestone`) combiné à cette cause — cf. `Trade::verdict`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseReason {
    /// `slHit` — SL touché avant TP1.
    Sl,
    /// `beHit` — SL revenu à l'entrée (BE) après TP1, avant TP2.
    Be,
    /// `tp2SLHit` — après TP2 armé, prix repassé sous TP1 (banked TP2, exit à BE).
    Tp2Sl,
    /// `tp3Hit` — TP3 atteint.
    Tp3,
    /// `_expire` — expiration temporelle (age ou TP3-expire après TP2).
    Expire,
    /// `not _filled and _beForce` — ordre en attente annulé par BOS opposé.
    Cancel,
}

/// Verdict utilisateur (/10-style) pour les stats : bucket TP1/TP2/TP3/SL/BE/Expire.
///
/// Dérivé du meilleur milestone de prix touché (`best_milestone`) :
/// TP3 > TP2 > TP1 primaires ; sinon SL/BE/Expire selon `CloseReason`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Tp3,
    Tp2,
    Tp1,
    Sl,
    Be,
    Expire,
}

/// État d'un trade dans son cycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TradeState {
    /// En attente de retest (non rempli).
    Pending,
    /// Rempli, encore ouvert.
    Open,
    /// Clôturé (verdict + R fixés).
    Closed,
}

/// Un trade complet (Pine : ~13 arrays parallèles `stBull*`/`stBear*`).
#[derive(Debug, Clone)]
pub struct Trade {
    /// Identifiant unique (croissant).
    pub id: u64,
    pub side: Side,
    pub source: TradeSource,

    /// Niveaux figés à la création (après clamp SL).
    pub entry: f64,
    /// SL courant (mutable : passe à `entry` au TP1 / BE-forcé).
    pub sl: f64,
    pub tp1: f64,
    pub tp2: f64,
    pub tp3: f64,
    /// Score brut archivé (Pine `_scR` = `obScore[i]` ou `bsScore[i]`).
    pub score: i32,
    /// Risque clampé à la création (`_r = max(slMin, min(slMax, raw_r))`).
    /// Sert de dénominateur R pour tous les calculs de R-multiple.
    pub risk0: f64,

    /// `int(time)` à la création (secondes Unix).
    pub open_ts: i64,
    /// Index de la bar de création.
    pub bar_created: usize,
    /// `impulse_bar` de l'OB lié (source Ob uniquement) — pour scoreDeg.
    pub ob_key: Option<usize>,

    // --- État lifecycle (mutable bar après bar) ---
    pub filled: bool,
    /// `stBullTP1Hit` Pine — vrai si TP1 touché (prix) **ou** BE forcé.
    pub tp1_hit: bool,
    /// Vrai si le **prix** a réellement touché TP1 (high>=tp1 buy / low<=tp1 sell).
    /// Distinct de `tp1_hit` car le BE-forcé positionne `tp1_hit=true` sans toucher TP1.
    pub tp1_price_touched: bool,
    /// `stBullTP2HitTs` Pine — 0 = non touché, sinon timestamp (s) du TP2.
    pub tp2_ts: i64,
    /// Vrai si le prix a touché TP3 (high>=tp3 buy / low<=tp3 sell).
    pub tp3_touched: bool,
    /// BE forcé par BOS opposé ou score degradation (SL→entry sans TP1 prix).
    pub be_forced: bool,

    pub state: TradeState,
    pub fill_ts: Option<i64>,
    pub close_reason: Option<CloseReason>,
    pub close_ts: Option<i64>,
    pub close_bar: Option<usize>,
    /// R-multiple réalisé à la clôture (baseline risk0).
    pub close_r: Option<f64>,
}

impl Trade {
    /// Meilleur milestone de prix réellement touché (0/1/2/3).
    pub fn best_milestone(&self) -> u8 {
        if self.tp3_touched {
            3
        } else if self.tp2_ts > 0 {
            2
        } else if self.tp1_price_touched {
            1
        } else {
            0
        }
    }

    /// Verdict utilisateur (TP1/TP2/TP3/SL/BE/Expire) dérivé du meilleur milestone.
    ///
    /// Règle : si un TP a été touché en prix, le verdict = meilleur TP (win).
    /// Sinon : SL si `Sl`, BE si `Be`, Expire sinon (y compris `Cancel`/`Expire`
    /// sans TP — un ordre jamais rempli ou expiré sans confluence haussière).
    pub fn verdict(&self) -> Verdict {
        match self.best_milestone() {
            3 => Verdict::Tp3,
            2 => Verdict::Tp2,
            1 => Verdict::Tp1,
            _ => match self.close_reason {
                Some(CloseReason::Sl) => Verdict::Sl,
                Some(CloseReason::Be) | Some(CloseReason::Tp2Sl) => Verdict::Be,
                _ => Verdict::Expire,
            },
        }
    }

    /// R-multiple réalisé à la clôture (baseline = `risk0`).
    ///
    /// TP3 = distance réelle `(tp3-entry)/risk0` ; TP2 = +2 ; TP1 = +1 ;
    /// BE/Expire = 0 ; SL = -1.
    pub fn realized_r(&self) -> f64 {
        match self.verdict() {
            Verdict::Tp3 => {
                if self.risk0 > 0.0 {
                    let dist = match self.side {
                        Side::Buy => self.tp3 - self.entry,
                        Side::Sell => self.entry - self.tp3,
                    };
                    dist / self.risk0
                } else {
                    0.0
                }
            }
            Verdict::Tp2 => 2.0,
            Verdict::Tp1 => 1.0,
            Verdict::Be => 0.0,
            Verdict::Expire => 0.0,
            Verdict::Sl => -1.0,
        }
    }

    /// Vrai si le verdict est gagnant (R > 0).
    pub fn is_win(&self) -> bool {
        matches!(self.verdict(), Verdict::Tp1 | Verdict::Tp2 | Verdict::Tp3)
    }

    /// Le trade est-il « neutralisé » (TP1 prix touché) — n'a plus besoin de bloquer
    /// la génération (f_tradeBloquant). Pine : `not stBullTP1Hit` → bloque si pas TP1.
    /// Note : `tp1_hit` inclut le BE-forcé, ce qui neutralise aussi le blocage.
    pub fn neutralized(&self) -> bool {
        self.tp1_hit
    }

    /// Initialise un trade BUY à partir des niveaux calculés par le générateur.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new_buy(
        id: u64,
        source: TradeSource,
        entry: f64,
        sl: f64,
        tp1: f64,
        tp2: f64,
        tp3: f64,
        score: i32,
        risk0: f64,
        bar: &BarInput,
        bar_index: usize,
        ob_key: Option<usize>,
    ) -> Self {
        Self {
            id,
            side: Side::Buy,
            source,
            entry,
            sl,
            tp1,
            tp2,
            tp3,
            score,
            risk0,
            open_ts: bar.timestamp,
            bar_created: bar_index,
            ob_key,
            filled: false,
            tp1_hit: false,
            tp1_price_touched: false,
            tp2_ts: 0,
            tp3_touched: false,
            be_forced: false,
            state: TradeState::Pending,
            fill_ts: None,
            close_reason: None,
            close_ts: None,
            close_bar: None,
            close_r: None,
        }
    }

    /// Initialise un trade SELL.
    #[allow(dead_code, clippy::too_many_arguments)]
    pub(crate) fn new_sell(
        id: u64,
        source: TradeSource,
        entry: f64,
        sl: f64,
        tp1: f64,
        tp2: f64,
        tp3: f64,
        score: i32,
        risk0: f64,
        bar: &BarInput,
        bar_index: usize,
        ob_key: Option<usize>,
    ) -> Self {
        let mut t = Self::new_buy(
            id, source, entry, sl, tp1, tp2, tp3, score, risk0, bar, bar_index, ob_key,
        );
        t.side = Side::Sell;
        t
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bar(ts: i64) -> BarInput {
        BarInput {
            timestamp: ts,
            open: 0.0,
            high: 0.0,
            low: 0.0,
            close: 0.0,
            volume: 0.0,
        }
    }

    #[test]
    fn verdict_sl_avant_tp1() {
        let mut t = Trade::new_buy(1, TradeSource::Ob, 100.0, 98.0, 102.0, 104.0, 106.0, 10, 2.0, &bar(0), 0, None);
        t.state = TradeState::Closed;
        t.close_reason = Some(CloseReason::Sl);
        assert_eq!(t.verdict(), Verdict::Sl);
        assert!((t.realized_r() - (-1.0)).abs() < 1e-9);
        assert!(!t.is_win());
    }

    #[test]
    fn verdict_tp1_puis_be() {
        // TP1 touché en prix puis BE : verdict TP1 (win +1R), même si close_reason=Be.
        let mut t = Trade::new_buy(2, TradeSource::Ob, 100.0, 98.0, 102.0, 104.0, 106.0, 10, 2.0, &bar(0), 0, None);
        t.tp1_price_touched = true;
        t.tp1_hit = true;
        t.close_reason = Some(CloseReason::Be);
        assert_eq!(t.verdict(), Verdict::Tp1);
        assert!((t.realized_r() - 1.0).abs() < 1e-9);
        assert!(t.is_win());
    }

    #[test]
    fn verdict_tp3_distance_reelle() {
        let mut t = Trade::new_buy(3, TradeSource::Ob, 100.0, 98.0, 102.0, 104.0, 110.0, 10, 2.0, &bar(0), 0, None);
        t.tp3_touched = true;
        t.close_reason = Some(CloseReason::Tp3);
        assert_eq!(t.verdict(), Verdict::Tp3);
        // (110-100)/2 = 5R
        assert!((t.realized_r() - 5.0).abs() < 1e-9);
    }

    #[test]
    fn verdict_be_force_sans_tp1_prix() {
        // BE forcé par BOS opposé : tp1_hit=true mais tp1_price_touched=false.
        let mut t = Trade::new_buy(4, TradeSource::Ob, 100.0, 98.0, 102.0, 104.0, 106.0, 10, 2.0, &bar(0), 0, None);
        t.tp1_hit = true;
        t.be_forced = true;
        t.close_reason = Some(CloseReason::Be);
        assert_eq!(t.verdict(), Verdict::Be);
        assert!((t.realized_r() - 0.0).abs() < 1e-9);
        assert!(t.neutralized(), "BE-forcé neutralise le blocage");
    }
}
