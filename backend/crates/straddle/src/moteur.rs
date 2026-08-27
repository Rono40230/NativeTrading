//! Moteur straddle — machine à états autour d'une annonce (définition étape 4).
//!
//! ```text
//! Idle ──T-30──> Range ──T-10s──> Position(2 jambes ouvertes à E)
//! ```
//!
//! RÈGLE PROPRIÉTAIRE (correction 26/08) : c'est le TIMER qui décide de
//! l'entrée, pas le prix. À T-10 s, le straddle est OUVERT au prix courant
//! E, quelle que soit sa valeur : les DEUX jambes (LONG et SHORT au même
//! prix E) vivent en parallèle, chacune avec ses niveaux symétriques.
//!
//! Gestion par jambe : SL = E∓1R ; TP1 = ±1R → SL resserré à E∓0,5R
//! (TAMPON anti-whipsaw, décision 27/08 : le rebond à E tuait la gagnante
//! juste avant le vrai mouvement — DAX 9h, +4R manqués) ; TP2 = ±2R →
//! SL à TP1 + TRAILING au tick ; sorties SL / TS / TimeStop. Le R réalisé d'une passe = SOMME NETTE des deux jambes (le SL
//! de la jambe perdante = la TP1 de la gagnante : ±1R). Une annonce sans
//! mouvement se ferme au TimeStop à E → passe journalisée à 0R.
//!
//! R = sl_atr × ATR14(**H1**) — la volatilité HORAIRE normale de l'asset,
//! pas la compression M1 pré-annonce (qui rendait le R microscopique face
//! aux spikes des annonces HIGH impact).

use common::{Asset, Direction, Timeframe};
use engine::types::{EvenementTrade, SignalBrut, SortieMoteur, TypeEvenementTrade};
use engine::{ContexteCloture, ContexteTick, Engine};

use crate::types::{Annonce, ParamsStraddle};

/// Nom du moteur (`SignalBrut.moteur`).
pub const NOM: &str = "straddle";

/// Tampon après TP1 : le SL passe de E∓1R à E∓0,5R (au lieu du BE à E).
/// Un whipsaw rebondissant à E ne tue plus la jambe gagnante ; une inversion
/// complète après TP1 coûte −0,5R (au lieu de 0R avec le BE).
const TAMPON_R: f64 = 0.5;

/// Nom d'une phase (diagnostic / affichage / tests).
#[derive(Debug, Clone, PartialEq)]
pub enum Phase {
    Idle,
    /// Fenêtre de préparation [T-30, T-10s] — ATR observé.
    Range { annonce_ts: i64 },
    /// Straddle OUVERT à T-10 s (timer) : 2 jambes au même prix E, chacune
    /// gérée indépendamment jusqu'à sa clôture.
    Position {
        annonce_ts: i64,
        /// Prix d'ouverture commun (prix courant à T-10 s).
        entree: f64,
        /// Risque unitaire figé à l'ouverture (distance SL initiale).
        r: f64,
        jambes: [Jambe; 2],
        ouverture_ts: i64,
        cle: String,
    },
}

/// Une jambe du straddle (LONG ou SHORT), ouverte à E à T-10 s.
#[derive(Debug, Clone, PartialEq)]
pub struct Jambe {
    /// Vrai = jambe LONG, faux = jambe SHORT.
    pub long: bool,
    /// SL courant (BE à E après TP1, à TP1 après TP2, puis trailing).
    pub sl: f64,
    pub tp1: f64,
    pub tp2: f64,
    /// Meilleur prix atteint depuis TP2 — base du trailing au tick.
    pub meilleur_depuis_tp2: Option<f64>,
    /// Clôture de la jambe : (verdict, R réalisé de CETTE jambe).
    pub fermee: Option<(String, f64)>,
}

impl Jambe {
    fn nouvelle(long: bool, entree: f64, r: f64) -> Self {
        Self {
            long,
            sl: if long { entree - r } else { entree + r },
            tp1: if long { entree + r } else { entree - r },
            tp2: if long { entree + 2.0 * r } else { entree - 2.0 * r },
            meilleur_depuis_tp2: None,
            fermee: None,
        }
    }

    fn ouverte(&self) -> bool {
        self.fermee.is_none()
    }

    /// R réalisé de la jambe au prix donné (baseline = r).
    fn r_a(&self, entree: f64, prix: f64, r: f64) -> f64 {
        if r <= 0.0 {
            return 0.0;
        }
        if self.long {
            (prix - entree) / r
        } else {
            (entree - prix) / r
        }
    }
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
    /// ATR M1 interne (repli si aucun ATR H1 disponible).
    atr: Atr14,
    /// Étalon du R : ATR14 **H1**. Injecté depuis la DB à l'armement
    /// (disponible immédiatement), puis auto-rafraîchi en live par les
    /// clôtures horaires reconstituées du flux M1 — la compression
    /// pré-annonce rendait l'ATR M1 microscopique face aux spikes
    /// (constat Gate 3 26/08 : R = 1,15 pt XAU vs spike 16 R).
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
            atr: Atr14::new(),
            atr_h1_injecte: None,
            atr_h1_live: Atr14::new(),
            heure_courante: None,
        }
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
            vec![entree + r, entree + 2.0 * r, entree + 2.0 * r],
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

    /// Gère UNE jambe sur un tick : SL/BE/TS, TP1, TP2 + trailing.
    /// Retourne les événements émis pour cette jambe.
    fn gerer_jambe(
        &self,
        jambe: &mut Jambe,
        entree: f64,
        r: f64,
        prix: f64,
        ts: i64,
        ouverture_ts: i64,
        cle: &str,
        sortie: &mut SortieMoteur,
    ) {
        let distance_trail = self.params.trailing_r * r;
        let nom_jambe = if jambe.long { "LONG" } else { "SHORT" };

        // Time-stop : sortie à l'heure, au prix courant. (Clôture de jambe
        // SILENCIEUSE — seule la Cloture finale de la passe ferme la ligne :
        // le writer signaux ferme au premier événement Cloture de la clé.)
        if ts - ouverture_ts >= self.params.time_stop_min * 60 {
            let rr = jambe.r_a(entree, prix, r);
            jambe.fermee = Some(("TimeStop".into(), rr));
            return;
        }

        // Sortie sur SL / trailing stop — verdict selon le niveau : TS
        // (au-delà de TP1, trailing armé) · SL (initial −1R ou tampon −0,5R —
        // le R réalisé fait la différence).
        if (jambe.long && prix <= jambe.sl) || (!jambe.long && prix >= jambe.sl) {
            let verdict = if (jambe.long && jambe.sl > jambe.tp1) || (!jambe.long && jambe.sl < jambe.tp1) {
                "TS"
            } else {
                "SL"
            };
            let rr = jambe.r_a(entree, jambe.sl, r);
            jambe.fermee = Some((verdict.to_string(), rr));
            return;
        }

        // TP1 : SL resserré au TAMPON E∓0,5R (pas le BE à E — décision 27/08 :
        // le rebond typique de l'ouverture/annonce touche E et tuait la
        // gagnante avant le vrai mouvement ; à −0,5R elle survit).
        if (jambe.long && prix >= jambe.tp1) || (!jambe.long && prix <= jambe.tp1) {
            let tampon = if jambe.long { entree - r * TAMPON_R } else { entree + r * TAMPON_R };
            if (jambe.long && jambe.sl < tampon) || (!jambe.long && jambe.sl > tampon) {
                jambe.sl = tampon;
                sortie.evenements.push(self.evenement(
                    cle,
                    TypeEvenementTrade::Tp1,
                    &format!("jambe {nom_jambe} TP1 — SL resserré à E∓{TAMPON_R}R (tampon)"),
                    jambe.tp1,
                    ts,
                ));
            }
        }
        // TP2 : BE à TP1 + DÉMARRAGE du trailing.
        if (jambe.long && prix >= jambe.tp2) || (!jambe.long && prix <= jambe.tp2) {
            if (jambe.long && jambe.sl < jambe.tp1) || (!jambe.long && jambe.sl > jambe.tp1) {
                jambe.sl = jambe.tp1;
                sortie.evenements.push(self.evenement(
                    cle,
                    TypeEvenementTrade::Tp2,
                    &format!("jambe {nom_jambe} TP2 — SL à TP1 + trailing actif"),
                    jambe.tp2,
                    ts,
                ));
            }
            // Trailing AU TICK : le SL suit le meilleur prix à distance
            // `trailing_r × R`, jamais vers l'arrière.
            let meilleur_courant = match jambe.meilleur_depuis_tp2 {
                Some(m) if jambe.long => m.max(prix),
                Some(m) => m.min(prix),
                None => prix,
            };
            jambe.meilleur_depuis_tp2 = Some(meilleur_courant);
            let cible = if jambe.long {
                meilleur_courant - distance_trail
            } else {
                meilleur_courant + distance_trail
            };
            let nouvelle = if jambe.long { jambe.sl.max(cible) } else { jambe.sl.min(cible) };
            if (jambe.long && nouvelle > jambe.sl) || (!jambe.long && nouvelle < jambe.sl) {
                jambe.sl = nouvelle;
            }
        }
    }

    /// Verdict net de la passe une fois les 2 jambes fermées.
    fn verdict_net(entree: f64, jambes: &[Jambe; 2]) -> (String, f64) {
        let net: f64 = jambes
            .iter()
            .map(|j| j.fermee.as_ref().map(|(_, r)| *r).unwrap_or(0.0))
            .sum();
        let _ = entree;
        let un_tp1 = jambes.iter().any(|j| j.meilleur_depuis_tp2.is_some() || {
            // TP1 touché si le SL a quitté sa position initiale vers E (BE)
            // ou si la jambe s'est fermée en TS/BE après TP1.
            matches!(j.fermee.as_ref().map(|(v, _)| v.as_str()), Some("TS") | Some("BE"))
        });
        if net > 1e-9 {
            ("tp2".into(), net)
        } else if net < -1e-9 {
            ("sl".into(), net)
        } else if un_tp1 {
            // TP1+BE (1R) moins SL perdante (-1R) = 0R net.
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
    /// gestion SL/BE/TP/trailing AU TICK de chaque jambe, R net à la fin.
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
                    let atr = self.atr_h1().unwrap_or_else(|| self.atr.get());
                    if atr > 0.0 {
                        // OUVERTURE PAR LE TIMER au prix courant E, quelle
                        // que soit sa valeur. R = sl_atr × ATR H1 (étalon de
                        // volatilité normale — pas la compression pré-annonce) ;
                        // repli ATR M1 si aucun étalon H1 disponible.
                        let r = self.params.sl_atr * atr;
                        if r > 0.0 {
                            let cle = format!("straddle-{}-{annonce_ts}-B", self.asset.as_str());
                            let jambes = [Jambe::nouvelle(true, prix, r), Jambe::nouvelle(false, prix, r)];
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
                for jambe in jambes.iter_mut() {
                    if jambe.ouverte() {
                        self.gerer_jambe(jambe, entree, r, prix, ts, ouverture_ts, &cle, &mut sortie);
                    }
                }
                if jambes.iter().all(|j| !j.ouverte()) {
                    // Les 2 jambes sont fermées : verdict net de la passe.
                    let (verdict, net) = Self::verdict_net(entree, &jambes);
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
            Some((h, ph, pl, pc)) if h == heure => {
                self.heure_courante = Some((h, ph.max(ctx.bougie.high), pl.min(ctx.bougie.low), ctx.bougie.close));
            }
            Some((h, ph, pl, pc)) => {
                // Nouvelle heure : la précédente est complète → barre H1.
                let _ = h;
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
