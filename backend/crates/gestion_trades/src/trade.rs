//! Trade struct + enums — cycle de vie des trades (Pine `stBull*`/`stBear*` arrays).
//!
//! Reproduit les arrays parallèles Pine (lignes 2387-2419) sous une seule struct.
//! Un `Trade` naît à la génération du signal (v11 OB ou BSZones), est REMPLI quand
//! le prix revient toucher l'entrée (retest, modèle "Retest (limite)"), puis évolue
//! via le lifecycle (SL → BE → TP2-SL → TP3 → TP1 → TP2) jusqu'à clôture.

use crate::barre::BarInput;

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
    /// `trailHit` — après TP2, trailing stop touché (réglage propriétaire) :
    /// sortie au stop suivi, au-dessus du BE.
    Ts,
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
    /// Trailing stop après TP2 (réglage propriétaire) : R = distance réelle
    /// du stop suivi (≥ TP2 si l'extrême a dépassé TP2 + k×R).
    Ts,
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
    /// Extrême atteint depuis TP2 (plus haut achat / plus bas vente) — base
    /// du trailing stop (réglage propriétaire, inactif par défaut).
    pub tp2_extremum: Option<f64>,
    /// Prix de sortie du trailing stop (verdict TS).
    pub ts_px: Option<f64>,
    /// Vrai si le prix a touché TP3 (high>=tp3 buy / low<=tp3 sell).
    pub tp3_touched: bool,
    /// BE forcé par BOS opposé ou score degradation (SL→entry sans TP1 prix).
    pub be_forced: bool,
    /// Étude étape 4 — BE auto : MFE ≥ seuil×r atteinte sans toucher TP1.
    pub mfe_armed: bool,

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
        if self.close_reason == Some(CloseReason::Ts) {
            return Verdict::Ts;
        }
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

    /// R-multiple réalisé à la clôture (baseline = `risk0`) — comptabilité
    /// propriétaire 24/08 : le TP touché est ACQUIS et comptabilisé
    /// (TP1+BE = 1R, TP2+BE = 2R) ; TP3 = distance réelle ; BE forcé
    /// (BOS opposé avant TP1, stop ramené à l'entrée) = 0R ; SL = -1R.
    pub fn realized_r(&self) -> f64 {
        if self.risk0 <= 0.0 {
            return 0.0;
        }
        let dist = |prix: f64| match self.side {
            Side::Buy => prix - self.entry,
            Side::Sell => self.entry - prix,
        };
        match self.verdict() {
            Verdict::Ts => dist(self.ts_px.unwrap_or(self.tp2)) / self.risk0,
            Verdict::Tp3 => dist(self.tp3) / self.risk0,
            // TP acquis = distance RÉELLE du niveau (étude étape 4 : TP1/TP2
            // devenus paramétrables — l'ancien 1.0/2.0 en dur supposait des
            // niveaux fixes et gonflait artificiellement les variantes).
            Verdict::Tp2 => dist(self.tp2) / self.risk0,
            Verdict::Tp1 => dist(self.tp1) / self.risk0,
            // BE forcé (jamais de TP touché) ou expiration : rien d'acquis.
            Verdict::Be | Verdict::Expire => 0.0,
            Verdict::Sl => -1.0,
        }
    }

    /// Vrai si le trade est gagnant (R réalisé > 0 — TP acquis compris).
    pub fn is_win(&self) -> bool {
        self.realized_r() > 0.0
    }

    /// Le trade est-il « neutralisé » (TP1 prix touché) — n'a plus besoin de bloquer
    /// la génération (f_tradeBloquant). Pine : `not stBullTP1Hit` → bloque si pas TP1.
    /// Note : `tp1_hit` inclut le BE-forcé, ce qui neutralise aussi le blocage.
    pub fn neutralized(&self) -> bool {
        self.tp1_hit
    }

    /// Initialise un trade BUY à partir des niveaux calculés par le générateur.
    #[allow(clippy::too_many_arguments)]
    pub fn new_buy(
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
            tp2_extremum: None,
            ts_px: None,
            tp3_touched: false,
            be_forced: false,
            mfe_armed: false,
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
    pub fn new_sell(
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
        let mut t = Trade::new_buy(
            1,
            TradeSource::Ob,
            100.0,
            98.0,
            102.0,
            104.0,
            106.0,
            10,
            2.0,
            &bar(0),
            0,
            None,
        );
        t.state = TradeState::Closed;
        t.close_reason = Some(CloseReason::Sl);
        assert_eq!(t.verdict(), Verdict::Sl);
        assert!((t.realized_r() - (-1.0)).abs() < 1e-9);
        assert!(!t.is_win());
    }

    #[test]
    fn verdict_tp1_puis_be() {
        // TP1 touché en prix puis BE : verdict TP1 (win +1R), même si close_reason=Be.
        let mut t = Trade::new_buy(
            2,
            TradeSource::Ob,
            100.0,
            98.0,
            102.0,
            104.0,
            106.0,
            10,
            2.0,
            &bar(0),
            0,
            None,
        );
        t.tp1_price_touched = true;
        t.tp1_hit = true;
        t.close_reason = Some(CloseReason::Be);
        assert_eq!(t.verdict(), Verdict::Tp1);
        // Comptabilité propriétaire : TP1 acquis = 1R.
        assert!((t.realized_r() - 1.0).abs() < 1e-9);
        assert!(t.is_win(), "TP1+BE = 1R acquis = gain");
    }

    #[test]
    fn verdict_tp3_distance_reelle() {
        let mut t = Trade::new_buy(
            3,
            TradeSource::Ob,
            100.0,
            98.0,
            102.0,
            104.0,
            110.0,
            10,
            2.0,
            &bar(0),
            0,
            None,
        );
        t.tp3_touched = true;
        t.close_reason = Some(CloseReason::Tp3);
        assert_eq!(t.verdict(), Verdict::Tp3);
        // (110-100)/2 = 5R
        assert!((t.realized_r() - 5.0).abs() < 1e-9);
    }

    #[test]
    fn verdict_be_force_sans_tp1_prix() {
        // BE forcé par BOS opposé : tp1_hit=true mais tp1_price_touched=false.
        let mut t = Trade::new_buy(
            4,
            TradeSource::Ob,
            100.0,
            98.0,
            102.0,
            104.0,
            106.0,
            10,
            2.0,
            &bar(0),
            0,
            None,
        );
        t.tp1_hit = true;
        t.be_forced = true;
        t.close_reason = Some(CloseReason::Be);
        assert_eq!(t.verdict(), Verdict::Be);
        assert!((t.realized_r() - 0.0).abs() < 1e-9);
        assert!(t.neutralized(), "BE-forcé neutralise le blocage");
    }
}
