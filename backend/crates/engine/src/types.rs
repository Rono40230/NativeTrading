//! Types fondamentaux du runtime tick.

use chrono::{DateTime, Utc};
use common::{Asset, Direction, Timeframe};
use serde::Serialize;

/// Événement prix entrant — deux formes selon la source :
///
/// - [`PrixEvent::Tick`] : prix isolé (trade public, prix spot) ;
/// - [`PrixEvent::Kline`] : snapshot de la bougie EN FORMATION poussé par
///   l'échange à chaque mise à jour (kline `confirm: false` de Bybit).
///   Source retenue par la Gate 0 (décision 1 : klines non confirmées).
///
/// Un même agrégateur (asset × TF) ne doit recevoir qu'UNE des deux formes
/// (source homogène), jamais un mélange.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PrixEvent {
    /// Prix isolé. `volume` = volume du tick si connu.
    Tick { prix: f64, volume: Option<f64> },
    /// Snapshot de la bougie en formation. `confirmee` = poussée de clôture
    /// officielle de l'échange (valeurs autoritaires — par ex. kline Bybit
    /// avec `confirm: true`).
    Kline {
        ouverture: f64,
        haut: f64,
        bas: f64,
        cloture: f64,
        volume: f64,
        confirmee: bool,
    },
}

impl PrixEvent {
    /// Dernier prix véhiculé par l'événement.
    pub fn prix(&self) -> f64 {
        match *self {
            PrixEvent::Tick { prix, .. } => prix,
            PrixEvent::Kline { cloture, .. } => cloture,
        }
    }
}

/// Tick brut horodaté à la réception.
#[derive(Debug, Clone, Copy)]
pub struct Tick {
    pub prix: f64,
    pub volume: Option<f64>,
    pub instant: DateTime<Utc>,
}

/// Événement routé vers un agrégateur (asset × TF).
///
/// Le runtime construit cet événement à partir d'un [`Tick`] (en calculant
/// `debut_bougie` depuis l'instant) ou directement depuis une kline poussée
/// par l'échange (le début de bougie est donné par le message).
#[derive(Debug, Clone)]
pub struct EvenementPrix {
    pub asset: Asset,
    pub tf: Timeframe,
    /// Début de la bougie visée (epoch secondes, aligné sur le TF).
    pub debut_bougie: i64,
    pub event: PrixEvent,
    pub recu_le: DateTime<Utc>,
}

/// Signal émis par un moteur du runtime.
///
/// Émis au PREMIER événement prix valide, puis **verrouillé, jamais rétracté**
/// (règle R5 — comportement alerte Pine `once_per_bar`). L'anti-ré-émission
/// par bougie est de la responsabilité du moteur (via sa clé de barre interne).
#[derive(Debug, Clone, Serialize)]
pub struct SignalBrut {
    /// Moteur émetteur (ex : `smc_v12`).
    pub moteur: String,
    pub asset: Asset,
    pub tf: Timeframe,
    pub direction: Direction,
    pub prix_entree: f64,
    pub stop_loss: f64,
    pub take_profits: Vec<f64>,
    /// Score/composite propre au moteur.
    pub score: i32,
    pub raison: String,
    /// Début (epoch sec) de la bougie au moment de l'émission.
    pub debut_barre: i64,
    pub emis_le: DateTime<Utc>,
    /// Clé stable du trade côté moteur (matching événements lifecycle — 2.8).
    pub cle: String,
}

impl SignalBrut {
    /// Constructeur minimal — les champs calculés (`debut_barre`, `emis_le`)
    /// sont dérivés du contexte.
    pub fn nouveau(
        moteur: &str,
        asset: Asset,
        tf: Timeframe,
        direction: Direction,
        prix_entree: f64,
        stop_loss: f64,
        take_profits: Vec<f64>,
        score: i32,
        raison: String,
        debut_barre: i64,
    ) -> Self {
        Self::avec_cle(
            moteur,
            asset,
            tf,
            direction,
            prix_entree,
            stop_loss,
            take_profits,
            score,
            raison,
            debut_barre,
            String::new(),
        )
    }

    /// Variante avec clé de trade (matching lifecycle — phase 2.8).
    #[allow(clippy::too_many_arguments)]
    pub fn avec_cle(
        moteur: &str,
        asset: Asset,
        tf: Timeframe,
        direction: Direction,
        prix_entree: f64,
        stop_loss: f64,
        take_profits: Vec<f64>,
        score: i32,
        raison: String,
        debut_barre: i64,
        cle: String,
    ) -> Self {
        Self {
            moteur: moteur.to_string(),
            asset,
            tf,
            direction,
            prix_entree,
            stop_loss,
            take_profits,
            score,
            raison,
            debut_barre,
            emis_le: Utc::now(),
            cle,
        }
    }
}

/// Type d'événement du cycle de vie d'un trade.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub enum TypeEvenementTrade {
    /// Ordre rempli (retest touché).
    Fill,
    /// Break-even armé (SL revenu à l'entrée).
    Be,
    /// TP1 touché.
    Tp1,
    /// TP2 touché.
    Tp2,
    /// TP3 touché.
    Tp3,
    /// Trade clôturé (SL/BE/TP2-SL/TP3/expiration/annulation).
    Cloture,
}

/// Événement de cycle de vie détecté AU TICK (intrabar) — le diff entre
/// deux évaluations successives du moteur. Comme un signal : émis au
/// premier tick valide, jamais rétracté.
#[derive(Debug, Clone, Serialize)]
pub struct EvenementTrade {
    pub moteur: String,
    pub asset: Asset,
    pub tf: Timeframe,
    /// Identité stable du trade côté moteur (clé d'anti-ré-émission).
    pub cle_trade: String,
    pub evenement: TypeEvenementTrade,
    /// Raison lisible (ex : `Sl`, `Tp2Sl`, `Expire`) — informative.
    pub detail: String,
    /// Prix associé à l'événement (niveau touché).
    pub prix: f64,
    /// Début (epoch sec) de la bougie au moment de l'événement.
    pub debut_barre: i64,
    pub emis_le: DateTime<Utc>,
}

/// Sortie d'une évaluation moteur : signaux (nouvelles entrées) et
/// événements de cycle de vie (fills, SL/TP, clôtures).
#[derive(Debug, Clone, Default)]
pub struct SortieMoteur {
    pub signaux: Vec<SignalBrut>,
    pub evenements: Vec<EvenementTrade>,
}

impl SortieMoteur {
    pub fn vide() -> Self {
        Self::default()
    }

    pub fn etend(&mut self, autre: SortieMoteur) {
        self.signaux.extend(autre.signaux);
        self.evenements.extend(autre.evenements);
    }
}
