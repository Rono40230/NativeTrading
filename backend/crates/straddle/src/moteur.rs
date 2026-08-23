//! Moteur straddle — machine à états autour d'une annonce.
//!
//! ```text
//! Idle ──T-30──> Range ──T-5──> Ordres ──fill──> Position(OCO) ──SL/BE/TP/TS──> Idle
//!                                    └──T+expiration sans fill──> Idle
//! ```

use common::{Asset, Direction, Timeframe};
use engine::types::{EvenementTrade, SignalBrut, SortieMoteur, TypeEvenementTrade};
use engine::{ContexteCloture, ContexteTick, Engine};

use crate::types::{Annonce, ParamsStraddle};

/// Nom du moteur (`SignalBrut.moteur`).
pub const NOM: &str = "straddle";

/// Phase courante de la machine.
#[derive(Debug, Clone, PartialEq)]
enum Phase {
    Idle,
    /// Construction du range [T-30, T-5].
    Range { annonce_ts: i64, haut: f64, bas: f64 },
    /// Ordres stop posés (buy au-dessus, sell en-dessous).
    Ordres { annonce_ts: i64, buy_stop: f64, sell_stop: f64 },
    /// Position en cours (l'autre ordre est annulé — OCO).
    Position {
        annonce_ts: i64,
        long: bool,
        entree: f64,
        sl: f64,
        tp1: f64,
        tp2: f64,
        tp3: f64,
        fill_ts: i64,
        cle: String,
    },
}

/// ATR14 interne (RMA des true ranges, warm-up sur on_close).
#[derive(Debug, Clone)]
struct Atr14 {
    valeur: f64,
    precedent_close: Option<f64>,
    n: u32,
}

impl Atr14 {
    fn new() -> Self {
        Self { valeur: 0.0, precedent_close: None, n: 0 }
    }
    fn update(&mut self, high: f64, low: f64, close: f64) {
        let tr = match self.precedent_close {
            Some(pc) => (high - low).max((high - pc).abs()).max((low - pc).abs()),
            None => high - low,
        };
        self.precedent_close = Some(close);
        self.n += 1;
        self.valeur = if self.n <= 14 {
            self.valeur + (tr - self.valeur) / self.n as f64 // moyenne cumulée
        } else {
            (self.valeur * 13.0 + tr) / 14.0 // RMA
        };
    }
    fn get(&self) -> f64 {
        self.valeur
    }
}

/// Moteur straddle — une instance par couple (asset × TF).
pub struct StraddleEngine {
    asset: Asset,
    tf: Timeframe,
    params: ParamsStraddle,
    /// Annonces à venir, triées par ts (injectées hors DB).
    annonces: Vec<Annonce>,
    phase: Phase,
    atr: Atr14,
}

impl StraddleEngine {
    pub fn nouveau(asset: Asset, tf: Timeframe) -> Self {
        Self {
            asset,
            tf,
            params: ParamsStraddle::default(),
            annonces: Vec::new(),
            phase: Phase::Idle,
            atr: Atr14::new(),
        }
    }

    /// Avec paramètres custom (calibrage).
    pub fn avec_params(mut self, params: ParamsStraddle) -> Self {
        self.params = params;
        self
    }

    /// Injecte les annonces à venir (appelé par le runtime — jamais de DB ici).
    pub fn avec_annonces(mut self, annonces: Vec<Annonce>) -> Self {
        let mut a = annonces;
        a.sort_by_key(|x| x.ts);
        self.annonces = a;
        self
    }

    fn signal_fill(&self, long: bool, entree: f64, sl: f64, tps: [f64; 3], cle: &str, ts: i64) -> SignalBrut {
        SignalBrut::avec_cle(
            NOM,
            self.asset.clone(),
            self.tf,
            if long { Direction::Long } else { Direction::Short },
            entree,
            sl,
            tps.to_vec(),
            78,
            format!("straddle fill @ {:.5}", entree),
            ts,
            cle.to_string(),
        )
    }

    fn evenement(&self, cle: &str, e: TypeEvenementTrade, detail: &str, prix: f64, ts: i64) -> EvenementTrade {
        EvenementTrade {
            moteur: NOM.to_string(),
            asset: self.asset.clone(),
            tf: self.tf,
            cle_trade: cle.to_string(),
            evenement: e,
            detail: detail.to_string(),
            prix,
            debut_barre: ts,
            emis_le: chrono::Utc::now(),
        }
    }
}

impl Engine for StraddleEngine {
    fn nom(&self) -> &str {
        NOM
    }

    /// Intrabar : range, fills (OCO), SL/TP/BE, time-stop.
    fn on_tick(&mut self, ctx: &ContexteTick) -> SortieMoteur {
        let mut sortie = SortieMoteur::vide();
        let prix = ctx.bougie.prix();
        let ts = ctx.bougie.debut;

        // Annonce suivante pertinente.
        let prochaine = self.annonces.first().cloned();

        match std::mem::replace(&mut self.phase, Phase::Idle) {
            Phase::Idle => {
                if let Some(a) = prochaine {
                    if ts >= a.ts - self.params.range_avant_min * 60 {
                        // Entrée en fenêtre de range.
                        self.phase = Phase::Range { annonce_ts: a.ts, haut: prix, bas: prix };
                    } else {
                        self.phase = Phase::Idle;
                    }
                }
            }
            Phase::Range { annonce_ts, mut haut, mut bas } => {
                haut = haut.max(prix);
                bas = bas.min(prix);
                if ts >= annonce_ts - self.params.placement_avant_min * 60 {
                    let atr = self.atr.get();
                    if atr > 0.0 && haut > bas {
                        let buy_stop = haut + self.params.offset_atr * atr;
                        let sell_stop = bas - self.params.offset_atr * atr;
                        self.phase = Phase::Ordres { annonce_ts, buy_stop, sell_stop };
                    } else {
                        // ATR indisponible ou range plat : on garde le range.
                        self.phase = Phase::Range { annonce_ts, haut, bas };
                    }
                } else {
                    self.phase = Phase::Range { annonce_ts, haut, bas };
                }
            }
            Phase::Ordres { annonce_ts, buy_stop, sell_stop } => {
                if prix >= buy_stop {
                    // FILL LONG — OCO : le sell-stop est annulé.
                    let atr = self.atr.get().max(1e-12);
                    let entree = buy_stop;
                    let cle = format!("straddle-{}-{}", annonce_ts, "L");
                    let s = self.signal_fill(
                        true,
                        entree,
                        entree - self.params.sl_atr * atr,
                        [
                            entree + self.params.tp1_atr * atr,
                            entree + self.params.tp2_atr * atr,
                            entree + self.params.tp3_atr * atr,
                        ],
                        &cle,
                        ts,
                    );
                    sortie.signaux.push(s);
                    let ev = self.evenement(&cle, TypeEvenementTrade::Fill, "OCO : sell-stop annulé", entree, ts);
                    sortie.evenements.push(ev);
                    let (sl, tp1, tp2, tp3) = (
                        entree - self.params.sl_atr * atr,
                        entree + self.params.tp1_atr * atr,
                        entree + self.params.tp2_atr * atr,
                        entree + self.params.tp3_atr * atr,
                    );
                    self.phase = Phase::Position { annonce_ts, long: true, entree, sl, tp1, tp2, tp3, fill_ts: ts, cle };
                } else if prix <= sell_stop {
                    let atr = self.atr.get().max(1e-12);
                    let entree = sell_stop;
                    let cle = format!("straddle-{}-{}", annonce_ts, "S");
                    let s = self.signal_fill(
                        false,
                        entree,
                        entree + self.params.sl_atr * atr,
                        [
                            entree - self.params.tp1_atr * atr,
                            entree - self.params.tp2_atr * atr,
                            entree - self.params.tp3_atr * atr,
                        ],
                        &cle,
                        ts,
                    );
                    sortie.signaux.push(s);
                    let ev = self.evenement(&cle, TypeEvenementTrade::Fill, "OCO : buy-stop annulé", entree, ts);
                    sortie.evenements.push(ev);
                    let (sl, tp1, tp2, tp3) = (
                        entree + self.params.sl_atr * atr,
                        entree - self.params.tp1_atr * atr,
                        entree - self.params.tp2_atr * atr,
                        entree - self.params.tp3_atr * atr,
                    );
                    self.phase = Phase::Position { annonce_ts, long: false, entree, sl, tp1, tp2, tp3, fill_ts: ts, cle };
                } else if ts >= annonce_ts + self.params.expiration_min * 60 {
                    // Aucun fill : les deux ordres expirent.
                    self.annonces.remove(0);
                    self.phase = Phase::Idle;
                } else {
                    self.phase = Phase::Ordres { annonce_ts, buy_stop, sell_stop };
                }
            }
            Phase::Position { annonce_ts, long, entree, mut sl, tp1, tp2, tp3, fill_ts, cle } => {
                let mut reste_ouverte = true;
                // Time-stop.
                if ts - fill_ts >= self.params.time_stop_min * 60 {
                    let ev = self.evenement(&cle, TypeEvenementTrade::Cloture, "TimeStop", prix, ts);
                    sortie.evenements.push(ev);
                    reste_ouverte = false;
                } else if long {
                    if prix <= sl {
                        let raison = if sl == entree { "BE" } else { "SL" };
                        let ev = self.evenement(&cle, TypeEvenementTrade::Cloture, raison, prix, ts);
                        sortie.evenements.push(ev);
                        reste_ouverte = false;
                    } else if prix >= tp3 {
                        sortie.evenements.push(self.evenement(&cle, TypeEvenementTrade::Tp3, "TP3", prix, ts));
                        let ev = self.evenement(&cle, TypeEvenementTrade::Cloture, "TP3", prix, ts);
                        sortie.evenements.push(ev);
                        reste_ouverte = false;
                    } else if prix >= tp2 {
                        sortie.evenements.push(self.evenement(&cle, TypeEvenementTrade::Tp2, "TP2", prix, ts));
                        // BE : profit ≥ 2R ⇒ SL à l'entrée.
                        sl = sl.max(entree);
                        if sl == entree && (entree - (entree - self.params.sl_atr * self.atr.get())).abs() > 0.0 {
                            sortie.evenements.push(self.evenement(&cle, TypeEvenementTrade::Be, "SL → entrée", prix, ts));
                        }
                    } else if prix >= tp1 {
                        sortie.evenements.push(self.evenement(&cle, TypeEvenementTrade::Tp1, "TP1", prix, ts));
                        sl = sl.max(entree); // BE dès TP1 (profit ≥ 1R… TP1 = 1,5×SL).
                        if sl == entree {
                            sortie.evenements.push(self.evenement(&cle, TypeEvenementTrade::Be, "SL → entrée", prix, ts));
                        }
                    }
                } else {
                    if prix >= sl {
                        let raison = if sl == entree { "BE" } else { "SL" };
                        let ev = self.evenement(&cle, TypeEvenementTrade::Cloture, raison, prix, ts);
                        sortie.evenements.push(ev);
                        reste_ouverte = false;
                    } else if prix <= tp3 {
                        sortie.evenements.push(self.evenement(&cle, TypeEvenementTrade::Tp3, "TP3", prix, ts));
                        let ev = self.evenement(&cle, TypeEvenementTrade::Cloture, "TP3", prix, ts);
                        sortie.evenements.push(ev);
                        reste_ouverte = false;
                    } else if prix <= tp2 {
                        sortie.evenements.push(self.evenement(&cle, TypeEvenementTrade::Tp2, "TP2", prix, ts));
                        sl = sl.min(entree);
                        if sl == entree {
                            sortie.evenements.push(self.evenement(&cle, TypeEvenementTrade::Be, "SL → entrée", prix, ts));
                        }
                    } else if prix <= tp1 {
                        sortie.evenements.push(self.evenement(&cle, TypeEvenementTrade::Tp1, "TP1", prix, ts));
                        sl = sl.min(entree);
                        if sl == entree {
                            sortie.evenements.push(self.evenement(&cle, TypeEvenementTrade::Be, "SL → entrée", prix, ts));
                        }
                    }
                }
                if reste_ouverte {
                    self.phase = Phase::Position { annonce_ts, long, entree, sl, tp1, tp2, tp3, fill_ts, cle };
                } else {
                    self.annonces.retain(|a| a.ts != annonce_ts);
                    self.phase = Phase::Idle;
                }
            }
        }
        sortie
    }

    /// Clôture : alimente l'ATR14 interne.
    fn on_close(&mut self, ctx: &ContexteCloture) -> SortieMoteur {
        self.atr.update(ctx.bougie.high, ctx.bougie.low, ctx.bougie.close);
        SortieMoteur::vide()
    }
}

#[cfg(test)]
#[path = "moteur_tests.rs"]
mod tests;
