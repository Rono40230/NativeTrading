//! Moteur straddle — machine à états autour d'une annonce (définition étape 4).
//!
//! ```text
//! Idle ──T-30──> Range ──T-10s──> Position(2 jambes ouvertes à E)
//! ```
//!
//! RÈGLE PROPRIÉTAIRE (correction 26/08) : c'est le TIMER qui décide de
//! l'entrée, pas le prix. À T-10 s, le straddle est OUVERT au prix courant
//! E, quelle qu'il soit : les DEUX jambes (LONG et SHORT au même prix E)
//! vivent en parallèle.
//!
//! GESTION UNIFIÉE (refonte) : dès l'ouverture, chaque jambe devient un
//! `Trade` du lifecycle commun (`gestion_trades`) — EXACTEMENT le même
//! moteur que la SMC : TP1 → SL au tampon E∓0,5R (be_offset, décision
//! 27/08 anti-whipsaw) ; TP2 → SL à TP1 + trailing au tick ; TP3 = +3R ;
//! expiration 60 min. Les deux jambes sont nourries au tick (barres
//! synthétiques une-tick). Le R net d'une passe = somme des R des jambes
//! (le SL de la perdante ≈ la TP1 de la gagnante : ±1R).
//!
//! R = sl_atr × ATR14(**H1**) — la volatilité HORAIRE normale de l'asset,
//! pas la compression M1 pré-annonce.

use common::{Asset, Direction, Timeframe};
use engine::types::{EvenementTrade, SignalBrut, SortieMoteur, TypeEvenementTrade};
use engine::{ContexteCloture, ContexteTick, Engine};
use gestion_trades::trade::{Side, Trade, TradeSource};
use gestion_trades::{BarInput, HookVide, TradeLifecycle};

use crate::types::{Annonce, ParamsStraddle};

/// Nom du moteur (`SignalBrut.moteur`).
pub const NOM: &str = "straddle";

/// Multiplicateurs des TP de la jambe (définition étape 4 — figés ici,
/// réglables à venir sur le rail commun).
const TP1_R: f64 = 1.0;
const TP2_R: f64 = 2.0;
const TP3_R: f64 = 3.0;

/// Tampon après TP1 : le SL passe de E∓1R à E∓0,5R (au lieu du BE à E).
/// Un whipsaw rebondissant à E ne tue plus la jambe gagnante ; une inversion
/// complète après TP1 coûte −0,5R (au lieu de 0R avec le BE).
const TAMPON_R: f64 = 0.5;

/// Nom d'une phase (diagnostic / affichage / tests).
#[derive(Debug, Clone)]
pub enum Phase {
    Idle,
    /// Fenêtre de préparation [T-30, T-10s] — ATR observé.
    Range { annonce_ts: i64 },
    /// Straddle OUVERT à T-10 s (timer) : 2 jambes au même prix E, chacune
    /// gérée par le lifecycle commun jusqu'à sa clôture.
    Position {
        annonce_ts: i64,
        /// Prix d'ouverture commun (prix courant à T-10 s).
        entree: f64,
        /// Risque unitaire figé à l'ouverture (distance SL initiale).
        r: f64,
        /// Les 2 jambes (LONG, SHORT) — trades du lifecycle commun, remplis
        /// d'emblée (le timer décide, pas le prix).
        jambes: [Trade; 2],
        ouverture_ts: i64,
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
    /// Machine de gestion commune (gestion_trades) — reconfigurée à chaque
    /// ouverture : tampon BE, trailing ×R dès TP2, expiration 60 min.
    lifecycle: TradeLifecycle,
    /// Compteur de ticks (bar_index du lifecycle).
    tick_index: usize,
    /// ATR M1 interne (repli si aucun ATR H1 disponible).
    atr: Atr14,
    /// Étalon du R : ATR14 **H1**. Injecté depuis la DB à l'armement
    /// (disponible immédiatement), puis auto-rafraîchi en live par les
    /// clôtures horaires reconstituées du flux M1.
    atr_h1_injecte: Option<f64>,
    /// RMA ATR sur barres H1 reconstituées du flux on_close.
    atr_h1_live: Atr14,
    /// Heure en cours d'agrégation : (ts_heure, high, low, close).
    heure_courante: Option<(i64, f64, f64, f64)>,
}

impl StraddleEngine {
    pub fn nouveau(asset: Asset, tf: Timeframe) -> Self {
        Self {
            asset,
            tf,
            params: ParamsStraddle::default(),
            annonces: Vec::new(),
            phase: Phase::Idle,
            lifecycle: Self::lifecycle_defaut(&ParamsStraddle::default()),
            tick_index: 0,
            atr: Atr14::new(),
            atr_h1_injecte: None,
            atr_h1_live: Atr14::new(),
            heure_courante: None,
        }
    }

    /// Lifecycle configuré pour le straddle : même moteur que la SMC, avec
    /// le tampon BE (E∓0,5R après TP1), le trailing ×R dès TP2 (nourri au
    /// tick), l'expiration = time-stop.
    fn lifecycle_defaut(params: &ParamsStraddle) -> TradeLifecycle {
        let exp = params.time_stop_min * 60;
        let mut lc = TradeLifecycle::new(exp, exp);
        lc.definir_be_offset_r(TAMPON_R);
        lc.definir_trailing_tp2(Some(params.trailing_r));
        lc
    }

    /// Injecte l'ATR14(H1) de l'asset (calculé par le runtime depuis la DB).
    pub fn avec_atr_h1(mut self, atr: Option<f64>) -> Self {
        self.atr_h1_injecte = atr.filter(|a| *a > 0.0);
        self
    }

    /// Étalon courant du R : RMA H1 live dès qu'elle a assez d'échantillons,
    /// sinon l'ATR H1 injectée, sinon l'ATR M1 (repli démarrage à froid).
    fn atr_h1(&self) -> Option<f64> {
        if self.atr_h1_live.n >= 3 && self.atr_h1_live.valeur > 0.0 {
            return Some(self.atr_h1_live.valeur);
        }
        self.atr_h1_injecte
    }

    /// Avec paramètres custom (calibrage).
    pub fn avec_params(mut self, params: ParamsStraddle) -> Self {
        self.lifecycle = Self::lifecycle_defaut(&params);
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

    /// Barre synthétique une-tick : chaque tick est une « bar » — le
    /// lifecycle évalue SL/TP/trailing à la granularité du tick.
    fn bar_tick(ts: i64, prix: f64) -> BarInput {
        BarInput {
            timestamp: ts,
            open: prix,
            high: prix,
            low: prix,
            close: prix,
            volume: 0.0,
        }
    }

    /// Construit la paire de jambes ouverte à E : LONG et SHORT remplis
    /// d'emblée (timer), SL = E∓1R, TP1/2/3 = ±1/2/3R, risque = r.
    fn jambes_nouvelles(entree: f64, r: f64, ts: i64, bar_index: usize) -> [Trade; 2] {
        let bar = Self::bar_tick(ts, entree);
        let mut long = Trade::new_buy(
            1, TradeSource::Ob, entree, entree - r,
            entree + TP1_R * r, entree + TP2_R * r, entree + TP3_R * r,
            78, r, &bar, bar_index, None,
        );
        long.filled = true;
        long.state = gestion_trades::trade::TradeState::Open;
        long.fill_ts = Some(ts);
        let mut short = Trade::new_sell(
            2, TradeSource::Ob, entree, entree + r,
            entree - TP1_R * r, entree - TP2_R * r, entree - TP3_R * r,
            78, r, &bar, bar_index, None,
        );
        short.filled = true;
        short.state = gestion_trades::trade::TradeState::Open;
        short.fill_ts = Some(ts);
        [long, short]
    }

    /// Signal d'ouverture du straddle : direction Both, niveaux de la jambe
    /// LONG (la jambe SHORT est symétrique autour de E — le writer la dérive
    /// en miroir pour l'insertion complète).
    fn signal_ouverture(
        &self,
        entree: f64,
        sl_long: f64,
        r: f64,
        cle: &str,
        ts: i64,
    ) -> SignalBrut {
        SignalBrut::avec_cle(
            NOM,
            self.asset.clone(),
            self.tf,
            Direction::Both,
            entree,
            sl_long,
            vec![entree + TP1_R * r, entree + TP2_R * r, entree + TP3_R * r],
            78,
            format!("straddle ouvert @ {:.5} R={:.5} (2 jambes, timer T-{}s)", entree, r, self.params.placement_avant_sec),
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

    /// Verdict net de la passe une fois les 2 jambes fermées — R = somme
    /// des R réalisés des jambes (comptabilité TP acquis du moteur commun).
    fn verdict_net(jambes: &[Trade; 2]) -> (String, f64) {
        let net: f64 = jambes
            .iter()
            .map(|t| t.close_r.unwrap_or(0.0))
            .sum();
        let un_tp1 = jambes.iter().any(|t| t.tp1_hit);
        if net > 1e-9 {
            ("tp2".into(), net)
        } else if net < -1e-9 {
            ("sl".into(), net)
        } else if un_tp1 {
            // TP1+BE acquis compensé par le SL de la perdante : ±1R net 0.
            ("be".into(), 0.0)
        } else {
            // Passe sans mouvement : 2 jambes refermées à E.
            ("expire".into(), 0.0)
        }
    }
}

impl Engine for StraddleEngine {
    fn nom(&self) -> &str {
        NOM
    }

    /// Intrabar : range, OUVERTURE à T-10 s par le timer (2 jambes à E),
    /// gestion UNIFIÉE au tick via le lifecycle commun, R net à la fin.
    fn on_tick(&mut self, ctx: &ContexteTick) -> SortieMoteur {
        let mut sortie = SortieMoteur::vide();
        let prix = ctx.bougie.prix();
        let ts = ctx.bougie.debut;
        let prochaine = self.annonces.first().cloned();
        self.tick_index += 1;

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
                    let atr = self.atr_h1().unwrap_or_else(|| self.atr.get());
                    if atr > 0.0 {
                        let r = self.params.sl_atr * atr;
                        if r > 0.0 {
                            let cle = format!("straddle-{}-{annonce_ts}-B", self.asset.as_str());
                            let jambes = Self::jambes_nouvelles(prix, r, ts, self.tick_index);
                            let s = self.signal_ouverture(prix, jambes[0].sl, r, &cle, ts);
                            sortie.signaux.push(s);
                            self.phase = Phase::Position {
                                annonce_ts,
                                entree: prix,
                                r,
                                jambes,
                                ouverture_ts: ts,
                                cle,
                            };
                            return sortie;
                        }
                    }
                    self.phase = Phase::Range { annonce_ts };
                } else {
                    self.phase = Phase::Range { annonce_ts };
                }
            }
            Phase::Position { annonce_ts, entree, r, mut jambes, ouverture_ts, cle } => {
                // Snapshot pré-update pour les transitions d'événements.
                let avant: Vec<(bool, i64, f64)> = jambes
                    .iter()
                    .map(|t| (t.tp1_hit, t.tp2_ts, t.sl))
                    .collect();

                // UNE évaluation du lifecycle commun par tick — les deux
                // jambes vivent dans le même carnet, nourries au tick.
                let bar = Self::bar_tick(ts, prix);
                self.lifecycle.update(&mut jambes, &bar, self.tick_index, &mut HookVide);

                // Événements de progression (diagnostic intrabar) : TP1
                // (tampon) et TP2 (SL à TP1 + trailing).
                for (i, t) in jambes.iter().enumerate() {
                    let (av_tp1, av_tp2, av_sl) = avant[i];
                    let nom_jambe = if matches!(t.side, Side::Buy) { "LONG" } else { "SHORT" };
                    if !av_tp1 && t.tp1_hit {
                        sortie.evenements.push(self.evenement(
                            &cle,
                            TypeEvenementTrade::Tp1,
                            &format!("jambe {nom_jambe} TP1 — SL au tampon E∓{TAMPON_R}R"),
                            t.tp1,
                            ts,
                        ));
                    }
                    if av_tp2 == 0 && t.tp2_ts > 0 {
                        sortie.evenements.push(self.evenement(
                            &cle,
                            TypeEvenementTrade::Tp2,
                            &format!("jambe {nom_jambe} TP2 — SL à TP1 + trailing actif"),
                            t.tp2,
                            ts,
                        ));
                    }
                    let _ = av_sl;
                }

                let toutes_fermees = jambes.iter().all(|t| t.close_reason.is_some());
                if toutes_fermees {
                    // Les 2 jambes sont fermées : verdict net de la passe.
                    let (verdict, net) = Self::verdict_net(&jambes);
                    sortie.evenements.push(self.evenement(
                        &cle,
                        TypeEvenementTrade::Cloture,
                        &format!("{verdict}|{net:.4}"),
                        prix,
                        ts,
                    ));
                    self.annonces.retain(|a| a.ts != annonce_ts);
                    self.phase = Phase::Idle;
                } else {
                    self.phase = Phase::Position { annonce_ts, entree, r, jambes, ouverture_ts, cle };
                }
            }
        }
        sortie
    }

    /// Clôture M1 : alimente l'ATR M1 (repli) et reconstitue les barres H1
    /// (high/low/close de l'heure en cours) pour auto-raffraîchir l'étalon.
    fn on_close(&mut self, ctx: &ContexteCloture) -> SortieMoteur {
        self.atr.update(ctx.bougie.high, ctx.bougie.low, ctx.bougie.close);
        let ts = ctx.bougie.timestamp.timestamp();
        let heure = ts - ts % 3600;
        match self.heure_courante {
            Some((h, ph, pl, _pc)) if h == heure => {
                self.heure_courante = Some((h, ph.max(ctx.bougie.high), pl.min(ctx.bougie.low), ctx.bougie.close));
            }
            Some((_h, ph, pl, pc)) => {
                // Nouvelle heure : la précédente est complète → barre H1.
                self.atr_h1_live.update(ph, pl, pc);
                self.heure_courante = Some((heure, ctx.bougie.high, ctx.bougie.low, ctx.bougie.close));
            }
            None => {
                self.heure_courante = Some((heure, ctx.bougie.high, ctx.bougie.low, ctx.bougie.close));
            }
        }
        SortieMoteur::vide()
    }
}

#[cfg(test)]
#[path = "moteur_tests.rs"]
mod tests;
