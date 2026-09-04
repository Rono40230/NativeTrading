//! Cycle de vie des trades — évaluation INTRABAR (Pine lignes 3791-4118).
//!
//! Reproduit fidèlement la machine à états Pine, évaluée sur CHAQUE bar (sans
//! `barstate.isconfirmed`) :
//!
//! 1. **Fill** (retest) — `time > openTs && low <= entry` (buy) / `high >= entry` (sell).
//! 2. **Expiration** — `age > tradeMaxMins` OU (TP2 armé `&& age > tp3MaxMins`).
//! 3. **Sorties** (ordre strict de précédence) :
//!    `slHit` (SL avant TP1) → `beHit` (BE après TP1, avant TP2) → `tp2SLHit`
//!    (après TP2, prix repassé sous TP1) → `tp3Hit` (TP3 atteint) → `expire`
//!    → `cancel` (ordre en attente + BOS opposé).
//! 4. **BE forcé** — si `filled && (beForce || scoreDeg) && !tp1Hit` : SL→entry,
//!    `tp1Hit=true` (mais pas `tp1_price_touched`). Trade maintenu ouvert.
//! 5. **Progression normale** — TP1 touché → SL→entry (BE) ; TP2 touché → arme TP3.
//!
//! Les milestones de prix (TP1/TP2/TP3 réellement touchés) sont suivis séparément
//! pour produire le verdict (TP1/TP2/TP3/SL/BE/Expire) et le R-multiple.

use crate::barre::BarInput;
use crate::trade::{CloseReason, Trade, TradeState};

/// Mode de gestion du BE forcé sur BOS opposé (étude comparatif 26/08 —
/// « 95 % des trades fermés à BE »). Classique = production fidèle Pine v12 ;
/// les autres modes servent au binaire `comparatif_be` pour trancher par
/// les chiffres. La dégradation de score (scoreDeg) n'est PAS concernée :
/// seule la cause BOS opposé varie entre modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ModeBeForce {
    /// SL → entrée au BOS opposé BRUT (Pine v12 — production).
    #[default]
    Classique,
    /// Clôture immédiate au prix du tick du BOS opposé (R partiel réalisé,
    /// positif ou négatif — pas de plancher 0R).
    Marche,
    /// Aucune action sur trade rempli : il vit jusqu'à SL/TP (l'annulation
    /// des ordres EN ATTENTE reste active — règle distincte).
    Supprime,
    /// BE uniquement sur MSS opposé (cassure avec displacement) — un
    /// micro-BOS ne suffit plus.
    Qualifie,
}

/// Contexte structurel du contexte appelant — le BE forcé (BOS/MSS opposé)
/// et le hook de dé-marquage sont fournis par la stratégie hôte. La SMC
/// l'implémente sur `SmcOutput` + `ScoringV11` ; le straddle passe `false`
/// (mode Supprime) et un hook no-op.
pub trait HookStructure {
    /// BOS opposé BRUT (Pine `bosBaissier`/`bosHaussier`, jamais masqué MSS).
    fn bos_oppose(&self, is_buy: bool) -> bool;
    /// MSS opposé (cassure avec displacement).
    fn mss_oppose(&self, is_buy: bool) -> bool;
    /// Dé-marque le premier OB signalé (règle de l'un-signal — SMC seule).
    fn sur_be_force(&mut self, _is_buy: bool) {}
}

/// Hook no-op (straddle : pas de structure, pas de règle d'un-signal).
pub struct HookVide;
impl HookStructure for HookVide {
    fn bos_oppose(&self, _is_buy: bool) -> bool { false }
    fn mss_oppose(&self, _is_buy: bool) -> bool { false }
}

/// Gestionnaire du cycle de vie — opère sur le carnet de trades du `SignalGenerator`.
#[derive(Clone)]

pub struct TradeLifecycle {
    /// `i_tradeMaxMins × 60` en secondes (Pine 2374-2375).
    trade_max_secs: i64,
    /// `i_tp3MaxMins × 60` en secondes (Pine 2372).
    tp3_max_secs: i64,
    /// Mode du BE forcé (étude comparatif — défaut Classique = production).
    mode_be_force: ModeBeForce,
    /// Étude étape 4 — BE automatique : Some(seuil) arme le BE quand la MFE
    /// atteint seuil×r sans toucher TP1 ; retour à l'entrée → SL→entry.
    be_auto_seuil: Option<f64>,
    /// Décalage du BE après TP1 en ×risk0 (0.0 = BE à l'entrée, sémantique
    /// SMC/Pine ; 0.5 = tampon straddle — décision 27/08 anti-whipsaw).
    be_offset_r: f64,
    /// Trailing stop après TP2 (réglage propriétaire, inactif par défaut) :
    /// Some(k) → stop = extrême post-TP2 ∓ k×risk0, évalué par bar (l'extrême
    /// de la bar en cours ne compte pas pour son propre stop — conservateur).
    trailing_tp2_r: Option<f64>,
}

impl TradeLifecycle {
    pub fn new(trade_max_secs: i64, tp3_max_secs: i64) -> Self {
        Self {
            trade_max_secs,
            tp3_max_secs,
            mode_be_force: ModeBeForce::Classique,
            be_auto_seuil: None,
            be_offset_r: 0.0,
            trailing_tp2_r: None,
        }
    }

    /// Sélectionne le seuil du BE auto (étude étape 4). None = inactif.
    pub fn definir_be_auto(&mut self, seuil: Option<f64>) {
        self.be_auto_seuil = seuil;
    }

    /// Sélectionne le mode du BE forcé (étude comparatif).
    pub fn definir_mode_be_force(&mut self, mode: ModeBeForce) {
        self.mode_be_force = mode;
    }

    /// Trailing stop après TP2 : Some(k) = actif à k×R de l'extrême post-TP2.
    pub fn definir_trailing_tp2(&mut self, k: Option<f64>) {
        self.trailing_tp2_r = k;
    }

    /// Décalage du BE après TP1 (0.0 = entrée). Le niveau protecteur devient
    /// entry ∓ offset×risk0 — la sortie sur ce niveau vaut −offset R.
    pub fn definir_be_offset_r(&mut self, offset: f64) {
        self.be_offset_r = offset;
    }

    /// Évalue tous les trades non clôturés sur la bar courante (Pine 3797-3952 / 3963-4118).
    pub fn update(
        &self,
        trades: &mut [Trade],
        bar: &BarInput,
        bar_index: usize,
        hook: &mut dyn HookStructure,
    ) {
        for t in trades.iter_mut() {
            if t.state == TradeState::Closed {
                continue;
            }
            let is_buy = matches!(t.side, crate::trade::Side::Buy);
            self.update_trade(t, is_buy, bar, bar_index, hook);
        }
    }

    fn update_trade(
        &self,
        t: &mut Trade,
        is_buy: bool,
        bar: &BarInput,
        bar_index: usize,
        hook: &mut dyn HookStructure,
    ) {
        // Snapshot de l'état de contrôle (état en début de bar).
        let sl = t.sl;
        let entry = t.entry;
        let tp1 = t.tp1;
        let tp2 = t.tp2;
        let tp3 = t.tp3;
        let tp1_hit = t.tp1_hit;
        let tp2_ts = t.tp2_ts;
        let tp2_extremum = t.tp2_extremum;
        let open_ts = t.open_ts;
        let filled = t.filled;

        // --- 1. Fill (retest) — Pine 3811 / 3977 ---
        // Modèle "Retest (limite)" : fill sur bar strictement après création,
        // quand le prix touche l'entrée (low<=entry buy / high>=entry sell).
        if !filled && bar.timestamp > open_ts {
            let touch_entry = if is_buy {
                bar.low <= entry
            } else {
                bar.high >= entry
            };
            if touch_entry {
                t.filled = true;
                t.fill_ts = Some(bar.timestamp);
                t.state = TradeState::Open;
            }
        }
        let filled = t.filled;

        // --- 2. Expiration — Pine 3851-3852 / 4017-4018 ---
        let age_expire = (bar.timestamp - open_ts) > self.trade_max_secs;
        let expire = (tp2_ts > 0 && (bar.timestamp - tp2_ts) > self.tp3_max_secs) || age_expire;

        // Stop suivi (réglage propriétaire, inactif par défaut) : après TP2,
        // stop = extrême post-TP2 ∓ k×risk0. L'extrème de la bar EN COURS ne
        // protège pas cette même bar (ordre intrabar inconnu — conservateur).
        let trail_stop = self.trailing_tp2_r.filter(|_| tp2_ts > 0).map(|k| {
            match (is_buy, tp2_extremum) {
                (true, Some(haut)) => haut - k * t.risk0,
                (false, Some(bas)) => bas + k * t.risk0,
                _ => f64::NAN,
            }
        });
        let trail_hit = match trail_stop {
            Some(stop) if stop.is_finite() => {
                if is_buy { filled && bar.low <= stop } else { filled && bar.high >= stop }
            }
            _ => false,
        };

        // Niveau BE : entrée ± offset (0.0 = entrée — sémantique SMC).
        let niveau_be = if is_buy {
            entry - self.be_offset_r * t.risk0
        } else {
            entry + self.be_offset_r * t.risk0
        };
        // --- 3. Sorties (avec l'état de début de bar) — Pine 3853-3856 / 4019-4022 ---
        let (sl_hit, be_hit, tp2_sl_hit, tp3_hit) = if is_buy {
            (
                filled && bar.low < sl && !tp1_hit,
                filled && bar.low < niveau_be && tp1_hit && tp2_ts == 0,
                filled && bar.low < tp1 && tp1_hit && tp2_ts > 0,
                filled && bar.high >= tp3,
            )
        } else {
            (
                filled && bar.high > sl && !tp1_hit,
                filled && bar.high > niveau_be && tp1_hit && tp2_ts == 0,
                filled && bar.high > tp1 && tp1_hit && tp2_ts > 0,
                filled && bar.low <= tp3,
            )
        };

        // --- BOS opposé (beForce) + score degradation (scoreDeg) ---
        // beForce = !tp1_hit && BOS opposé BRUT (Pine `bosBaissier`/`bosHaussier`
        // lignes 457-458, jamais masqués par le filtre MSS — un BOS-MSS force
        // aussi le BE). Variantes de l'étude comparatif :
        // - Supprime : jamais (le trade vit jusqu'à SL/TP).
        // - Qualifie : MSS opposé uniquement (displacement), pas un micro-BOS.
        let bos_oppose = hook.bos_oppose(is_buy);
        let mss_oppose = hook.mss_oppose(is_buy);
        let be_force = match self.mode_be_force {
            ModeBeForce::Supprime => false,
            ModeBeForce::Qualifie => !tp1_hit && mss_oppose,
            ModeBeForce::Classique | ModeBeForce::Marche => !tp1_hit && bos_oppose,
        };
        // --- 4. Sortie si condition — précédence stricte Pine ---
        let close_reason = if sl_hit {
            Some(CloseReason::Sl)
        } else if be_hit {
            Some(CloseReason::Be)
        } else if trail_hit {
            // Sortie au stop suivi — meilleur que le retour sous TP1 (BE).
            t.ts_px = trail_stop;
            Some(CloseReason::Ts)
        } else if tp2_sl_hit {
            Some(CloseReason::Tp2Sl)
        } else if tp3_hit {
            // Verdict TP3 : on enregistre le milestone tp3 touché.
            t.tp3_touched = true;
            Some(CloseReason::Tp3)
        } else if expire {
            Some(CloseReason::Expire)
        } else if !filled && be_force {
            Some(CloseReason::Cancel)
        } else {
            None
        };

        if let Some(reason) = close_reason {
            t.state = TradeState::Closed;
            t.close_reason = Some(reason);
            t.close_ts = Some(bar.timestamp);
            t.close_bar = Some(bar_index);
            t.close_r = Some(match reason {
                // BE décalé (tampon straddle) : sortie au niveau, R = −offset.
                CloseReason::Be if self.be_offset_r > 0.0 => {
                    let dist = |prix: f64| match t.side {
                        crate::trade::Side::Buy => prix - t.entry,
                        crate::trade::Side::Sell => t.entry - prix,
                    };
                    if t.risk0 > 0.0 { dist(niveau_be) / t.risk0 } else { 0.0 }
                }
                _ => t.realized_r(),
            });
            return;
        }

        // --- 5. BE forcé — Pine 3908-3923 / 4074-4089 ---
        if filled && be_force && !tp1_hit {
            // Variante Marché : clôture immédiate au prix courant — le R
            // partiel est réalisé tel quel (souvent négatif : le BOS opposé
            // survient contre le trade).
            if self.mode_be_force == ModeBeForce::Marche && be_force {
                t.state = TradeState::Closed;
                t.close_reason = Some(CloseReason::Be);
                t.be_forced = true;
                t.close_ts = Some(bar.timestamp);
                t.close_bar = Some(bar_index);
                let r_marche = if t.risk0 > 0.0 {
                    if is_buy {
                        (bar.close - t.entry) / t.risk0
                    } else {
                        (t.entry - bar.close) / t.risk0
                    }
                } else {
                    0.0
                };
                t.close_r = Some(r_marche);
                return;
            }
            t.sl = entry; // SL → entry (BE).
            t.tp1_hit = true; // Neutralise (n'a plus besoin de TP1 gate).
            t.be_forced = true;
            t.state = TradeState::Open;
            // Pine 3936-3941 + 3987-3988 : un-signal le PREMIER OB signalé du
            // carneau (source OB uniquement — _srcBull == 0), PAS forcément
            // l'OB du trade. Sémantique exacte du Pine : limite les re-trades.
            if t.ob_key.is_some() {
                hook.sur_be_force(is_buy);
            }
            return;
        }

        // --- 6b. BE auto (étude étape 4) : MFE ≥ seuil×r sans TP1 → armé ;
        //     retour à l'entrée → SL→entry (même mécanique que le BE à TP1,
        //     sans toucher TP1 en prix — cf. BE-forcé).
        if let Some(seuil) = self.be_auto_seuil {
            if filled && !t.tp1_hit {
                let favorable = if is_buy { bar.high - entry } else { entry - bar.low };
                if !t.mfe_armed && t.risk0 > 0.0 && favorable >= seuil * t.risk0 {
                    t.mfe_armed = true;
                }
                if t.mfe_armed {
                    let retour = if is_buy { bar.low <= entry } else { bar.high >= entry };
                    if retour {
                        t.tp1_hit = true;
                        t.be_forced = true;
                        t.sl = entry;
                    }
                }
            }
        }

        // --- 6. Progression normale — Pine 3934-3947 / 4100-4113 ---
        if filled {
            // TP1 touché → BE (SL→entry).
            if !tp1_hit {
                let touch_tp1 = if is_buy {
                    bar.high >= tp1
                } else {
                    bar.low <= tp1
                };
                if touch_tp1 {
                    t.tp1_hit = true;
                    t.tp1_price_touched = true;
                    t.sl = niveau_be;
                }
            }
            // TP2 touché → arme TP3 (timestamp) + amorce l'extrême suivi.
            if t.tp1_hit && t.tp2_ts == 0 {
                let touch_tp2 = if is_buy {
                    bar.high >= tp2
                } else {
                    bar.low <= tp2
                };
                if touch_tp2 {
                    t.tp2_ts = bar.timestamp;
                    t.tp2_extremum = Some(if is_buy { bar.high } else { bar.low });
                }
            }
            // Extrême post-TP2 : alimente le trailing stop des barres suivantes.
            if t.tp2_ts > 0 && self.trailing_tp2_r.is_some() {
                let extremum = t.tp2_extremum.unwrap_or(if is_buy { bar.high } else { bar.low });
                t.tp2_extremum = Some(if is_buy {
                    extremum.max(bar.high)
                } else {
                    extremum.min(bar.low)
                });
            }
            // tp3 milestone (cas où tp3 touché sans déclencher tp3_hit car !filled
            // au début — rempli cette bar ; on l'enregistre pour stats).
            if !t.tp3_touched {
                let touch_tp3 = if is_buy {
                    bar.high >= tp3
                } else {
                    bar.low <= tp3
                };
                if touch_tp3 {
                    t.tp3_touched = true;
                }
            }
        }
        if filled {
            t.state = TradeState::Open;
        }
    }
}

#[cfg(test)]
#[path = "lifecycle_tests.rs"]
mod tests;
