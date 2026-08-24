//! Moteur straddle — machine à états autour d'une annonce (définition étape 4).
//!
//! ```text
//! Idle ──T-30──> Range ──T-10s──> Ordres(E, R) ──franchissement──> Position(OCO)
//!                                     └──T+expiration sans fill──> Idle
//! Position : SL = E∓1R ; TP1 = 1R → BE à E ; TP2 = 2R → BE à TP1 + TRAILING
//! au tick (jamais vers l'arrière) ; sorties SL / BE / TS / TimeStop.
//! ```

use common::{Asset, Direction, Timeframe};
use engine::types::{EvenementTrade, SignalBrut, SortieMoteur, TypeEvenementTrade};
use engine::{ContexteCloture, ContexteTick, Engine};

use crate::types::{Annonce, ParamsStraddle};

/// Nom du moteur (`SignalBrut.moteur`).
pub const NOM: &str = "straddle";

/// Phase courante de la machine (publique — diagnostic / affichage / tests).
#[derive(Debug, Clone, PartialEq)]
pub enum Phase {
    Idle,
    /// Fenêtre de préparation [T-30, T-10s] — ATR + range observé.
    Range { annonce_ts: i64 },
    /// Deux jambes posées au MÊME prix E, armées : premier franchissement
    /// remplit (buy si prix > E, sell si prix < E), l'autre est annulée.
    Ordres { annonce_ts: i64, entree: f64, r: f64 },
    /// Position en cours sur la jambe survivante.
    Position {
        annonce_ts: i64,
        long: bool,
        entree: f64,
        /// Risque unitaire figé au placement (distance SL initiale).
        r: f64,
        /// SL courant (BE à E après TP1, à TP1 après TP2, puis trailing).
        sl: f64,
        tp1: f64,
        tp2: f64,
        /// Meilleur prix atteint depuis TP2 — base du trailing au tick.
        meilleur_depuis_tp2: Option<f64>,
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

    /// Phase courante (diagnostic / tests / affichage).
    pub fn phase_courante(&self) -> &Phase {
        &self.phase
    }

    fn signal_fill(&self, long: bool, entree: f64, sl: f64, r: f64, cle: &str, ts: i64) -> SignalBrut {
        // TP1 = 1R, TP2 = 2R (canoniques) ; le 3e niveau sert d'affichage du
        // trailing — le writer n'insère que la jambe survivante.
        let tps = if long {
            [entree + r, entree + 2.0 * r, entree + 2.0 * r]
        } else {
            [entree - r, entree - 2.0 * r, entree - 2.0 * r]
        };
        SignalBrut::avec_cle(
            NOM,
            self.asset.clone(),
            self.tf,
            if long { Direction::Long } else { Direction::Short },
            entree,
            sl,
            tps.to_vec(),
            78,
            format!("straddle fill @ {:.5} R={:.5}", entree, r),
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

    /// R réalisé au prix de sortie (signé par la direction, baseline = r).
    fn r_realise(long: bool, entree: f64, prix: f64, r: f64) -> f64 {
        if r <= 0.0 {
            return 0.0;
        }
        if long {
            (prix - entree) / r
        } else {
            (entree - prix) / r
        }
    }
}

impl Engine for StraddleEngine {
    fn nom(&self) -> &str {
        NOM
    }

    /// Intrabar : range, armement T-10 s, fill OCO, SL/BE/TP, trailing AU TICK.
    fn on_tick(&mut self, ctx: &ContexteTick) -> SortieMoteur {
        let mut sortie = SortieMoteur::vide();
        let prix = ctx.bougie.prix();
        let ts = ctx.bougie.debut;
        let prochaine = self.annonces.first().cloned();

        match std::mem::replace(&mut self.phase, Phase::Idle) {
            Phase::Idle => {
                if let Some(a) = prochaine {
                    if ts >= a.ts - self.params.range_avant_min * 60 {
                        self.phase = Phase::Range { annonce_ts: a.ts };
                    } else {
                        self.phase = Phase::Idle;
                    }
                }
            }
            Phase::Range { annonce_ts } => {
                if ts >= annonce_ts - self.params.placement_avant_sec {
                    let atr = self.atr.get();
                    if atr > 0.0 {
                        // Pose des 2 jambes au MÊME prix E = prix courant.
                        // R = sl_atr × ATR (risque unitaire).
                        let r = self.params.sl_atr * atr;
                        if r > 0.0 {
                            self.phase = Phase::Ordres { annonce_ts, entree: prix, r };
                            return sortie;
                        }
                    }
                    self.phase = Phase::Range { annonce_ts };
                } else {
                    self.phase = Phase::Range { annonce_ts };
                }
            }
            Phase::Ordres { annonce_ts, entree, r } => {
                if prix > entree {
                    // FILL LONG — OCO : la jambe sell est annulée.
                    let cle = format!("straddle-{}-L", annonce_ts);
                    let sl = entree - r;
                    let s = self.signal_fill(true, entree, sl, r, &cle, ts);
                    sortie.signaux.push(s);
                    sortie.evenements.push(self.evenement(
                        &cle,
                        TypeEvenementTrade::Fill,
                        "OCO : jambe sell annulée",
                        entree,
                        ts,
                    ));
                    self.phase = Phase::Position {
                        annonce_ts,
                        long: true,
                        entree,
                        r,
                        sl,
                        tp1: entree + r,
                        tp2: entree + 2.0 * r,
                        meilleur_depuis_tp2: None,
                        fill_ts: ts,
                        cle,
                    };
                } else if prix < entree {
                    let cle = format!("straddle-{}-S", annonce_ts);
                    let sl = entree + r;
                    let s = self.signal_fill(false, entree, sl, r, &cle, ts);
                    sortie.signaux.push(s);
                    sortie.evenements.push(self.evenement(
                        &cle,
                        TypeEvenementTrade::Fill,
                        "OCO : jambe buy annulée",
                        entree,
                        ts,
                    ));
                    self.phase = Phase::Position {
                        annonce_ts,
                        long: false,
                        entree,
                        r,
                        sl,
                        tp1: entree - r,
                        tp2: entree - 2.0 * r,
                        meilleur_depuis_tp2: None,
                        fill_ts: ts,
                        cle,
                    };
                } else if ts >= annonce_ts + self.params.expiration_min * 60 {
                    // Aucun franchissement : les deux jambes expirent.
                    self.annonces.remove(0);
                    self.phase = Phase::Idle;
                } else {
                    self.phase = Phase::Ordres { annonce_ts, entree, r };
                }
            }
            Phase::Position { annonce_ts, long, entree, r, mut sl, tp1, tp2, meilleur_depuis_tp2: mut meilleur, fill_ts, cle } => {
                let mut reste_ouverte = true;
                let distance_trail = self.params.trailing_r * r;

                // Time-stop : sortie à l'heure, au prix courant.
                if ts - fill_ts >= self.params.time_stop_min * 60 {
                    let rr = Self::r_realise(long, entree, prix, r);
                    sortie.evenements.push(self.evenement(
                        &cle,
                        TypeEvenementTrade::Cloture,
                        &format!("TimeStop|{:.4}", rr),
                        prix,
                        ts,
                    ));
                    reste_ouverte = false;
                } else if (long && prix <= sl) || (!long && prix >= sl) {
                    // Sortie sur SL / BE / trailing stop — le verdict dépend
                    // du niveau touché (SL initial, E après TP1, TP1+trail).
                    let verdict = if (sl - entree).abs() < 1e-12 {
                        "BE"
                    } else if (long && sl > tp1) || (!long && sl < tp1) {
                        "TS"
                    } else {
                        "SL"
                    };
                    let rr = Self::r_realise(long, entree, sl, r);
                    sortie.evenements.push(self.evenement(
                        &cle,
                        TypeEvenementTrade::Cloture,
                        &format!("{}|{:.4}", verdict, rr),
                        sl,
                        ts,
                    ));
                    reste_ouverte = false;
                } else {
                    // Gestion des niveaux — TP1 : BE à l'entrée.
                    if (long && prix >= tp1) || (!long && prix <= tp1) {
                        if (long && sl < entree) || (!long && sl > entree) {
                            sl = entree;
                            sortie.evenements.push(self.evenement(
                                &cle,
                                TypeEvenementTrade::Tp1,
                                "TP1 — SL à l'entrée (BE)",
                                tp1,
                                ts,
                            ));
                            sortie.evenements.push(self.evenement(
                                &cle,
                                TypeEvenementTrade::Be,
                                "BE à l'entrée",
                                entree,
                                ts,
                            ));
                        }
                    }
                    // TP2 : BE à TP1 + DÉMARRAGE du trailing.
                    if (long && prix >= tp2) || (!long && prix <= tp2) {
                        if (long && sl < tp1) || (!long && sl > tp1) {
                            sl = tp1;
                            sortie.evenements.push(self.evenement(
                                &cle,
                                TypeEvenementTrade::Tp2,
                                "TP2 — SL à TP1 + trailing actif",
                                tp2,
                                ts,
                            ));
                        }
                        // Trailing AU TICK : le SL suit le meilleur prix à
                        // distance `trailing_r × R`, jamais vers l'arrière.
                        let meilleur_courant = match meilleur {
                            Some(m) if long => m.max(prix),
                            Some(m) => m.min(prix),
                            None => prix,
                        };
                        meilleur = Some(meilleur_courant);
                        let cible = if long {
                            meilleur_courant - distance_trail
                        } else {
                            meilleur_courant + distance_trail
                        };
                        let nouvelle = if long { sl.max(cible) } else { sl.min(cible) };
                        if (long && nouvelle > sl) || (!long && nouvelle < sl) {
                            sl = nouvelle;
                        }
                    }
                }
                if reste_ouverte {
                    self.phase = Phase::Position { annonce_ts, long, entree, r, sl, tp1, tp2, meilleur_depuis_tp2: meilleur, fill_ts, cle };
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
