//! Plugin SMC v12 pour le runtime tick (Phase 2.1 ROADMAP).
//!
//! La logique v12 (`smc::v12` — 16 composantes, scoring, lifecycle) est
//! **rebranchée, pas réécrite**, derrière le trait [`engine::Engine`].
//!
//! ## Modèle d'exécution = rollback Pine
//!
//! TradingView ré-exécute le script sur la bougie live à chaque tick, en
//! repartant de l'état confirmé de la bougie précédente. Le plugin reproduit
//! exactement ce modèle :
//!
//! - **`on_tick`** (chaque prix) : clone du moteur confirmé → évaluation
//!   complète de la bougie en formation (`close` = dernier prix) → les
//!   nouveaux trades sont émis comme des **alertes Pine `once_per_bar`** :
//!   au premier tick valide, **verrouillés, jamais rétractés** (R5) — même
//!   si la condition disparaît ensuite sur le graphique. Le clone est
//!   ensuite jeté : l'état confirmé reste intact.
//! - **`on_close`** (clôture officielle) : la bougie finale alimente le
//!   moteur RÉEL — c'est le commit autoritaire. Un trade né de conditions
//!   de clôture (BOS confirmé, displacement…) est émis ici s'il n'avait pas
//!   été vu intrabar.
//!
//! ## Anti-ré-émission
//!
//! Clé par trade : (open_ts, sens, source, entrée bit à bit) — stable entre
//! redémarrages. Intrabar : ANNONCE d'imminence (Telegram, une par trade).
//! À la clôture (Pine `barstate.isconfirmed`) : création du trade + ligne
//! officielle en base ; intrabar, seuls les événements lifecycle.

use std::collections::HashSet;

use common::{Asset, Candle, Direction, Timeframe};
use engine::{
    BougieEnFormation, ContexteCloture, ContexteTick, Engine, EvenementTrade, SignalBrut,
    SortieMoteur, TypeEvenementTrade,
};
use smc::v12::trade::{Side, Trade, TradeSource, TradeState};
use smc::v12::{BarInput, SmcV12Engine};

/// Nom du moteur (identifiant stable dans `SignalBrut.moteur`).
pub const NOM: &str = "smc_v12";

pub mod replay;

/// Convertit une bougie clôturée en entrée moteur (partagé avec le replay).
pub(crate) fn bar_input_depuis_bougie(c: &Candle) -> BarInput {
    BarInput {
        timestamp: c.timestamp.timestamp(),
        open: c.open,
        high: c.high,
        low: c.low,
        close: c.close,
        volume: c.volume,
    }
}

/// Bougie en formation synthétique à un instant donné d'une barre (replay
/// intrabar simulé : `close` = prix courant, `high_vu`/`low_vu` = extrêmes
/// déjà parcourus).
pub(crate) fn formation_depuis_bar(
    b: &BarInput,
    close: f64,
    high_vu: f64,
    low_vu: f64,
) -> BougieEnFormation {
    BougieEnFormation {
        debut: b.timestamp,
        open: b.open,
        high: high_vu,
        low: low_vu,
        close,
        volume: b.volume,
        nb_events: 1,
        dernier_event: None,
    }
}

/// Clé d'anti-ré-émission d'un trade — `(open_ts, side, source, entry_bits)`.
/// `open_ts` (horodatage de la barre de création) plutôt que l'index de barre :
/// stable d'un redémarrage à l'autre malgré la fenêtre de replay qui glisse.
type CleTrade = (i64, u8, u8, u64);

/// État lifecycle d'un trade tel que vu à la dernière évaluation — la
/// comparaison de deux états produit les événements (fill, SL/TP, clôture).
/// Le moteur SMC v12 en plugin du runtime — une instance par (asset × TF),
/// comme un indicateur Pine par graphique.
pub struct MoteurV12 {
    asset: Asset,
    tf: Timeframe,
    /// Moteur confirmé (état commité à chaque clôture).
    moteur: SmcV12Engine,
    /// Clés des trades déjà émis (alertes verrouillées).
    emis: HashSet<CleTrade>,
    /// Clés des setups déjà ANNONCÉS intrabar (message d'imminence) —
    /// jamais élagué : un setup qui oscille au bord de zone n'est annoncé
    /// qu'une fois par bougie.
    annonces: HashSet<CleTrade>,
    /// Dernier état lifecycle vu par trade — le diff produit les événements.
    vus: std::collections::HashMap<CleTrade, EtatVu>,
    /// Amorce MTF (H1/H4/W1/MN de la DB) — appliquée paresseusement à la
    /// 1re bar (t0 connu), sinon les confluences W1 (+5) / MN (+6) du
    /// scoring ne verraient que la fenêtre de replay (Pine/TV : années).
    amorce: Option<smc::v12::AmorceMtf>,
    amorce_appliquee: bool,
}

impl MoteurV12 {
    pub fn nouveau(asset: Asset, tf: Timeframe) -> Self {
        Self {
            asset: asset.clone(),
            tf,
            moteur: SmcV12Engine::new(asset.as_str(), tf.as_str()),
            emis: HashSet::new(),
            annonces: HashSet::new(),
            vus: std::collections::HashMap::new(),
            amorce: None,
            amorce_appliquee: false,
        }
    }

    /// Attache l'amorce MTF (H1/H4/W1/MN de la DB) — appliquée à la 1re bar.
    pub fn avec_amorce(mut self, amorce: smc::v12::AmorceMtf) -> Self {
        self.amorce = Some(amorce);
        self
    }

    /// Mode du BE forcé sur BOS opposé (étude comparatif — défaut
    /// Classique = production).
    pub fn avec_mode_be_force(mut self, mode: smc::v12::lifecycle::ModeBeForce) -> Self {
        self.moteur = self.moteur.avec_mode_be_force(mode);
        self
    }

    /// Mode TP3 (étude Module G — défaut Dol = production).
    pub fn avec_mode_tp3(mut self, mode: smc::v12::signals::ModeTp3) -> Self {
        self.moteur = self.moteur.avec_mode_tp3(mode);
        self
    }

    /// TP1 réglable (Paramètres › stratégies › SMC, défaut 0.6) — passe au
    /// moteur : les nouveaux signaux partent avec ce niveau.
    pub fn avec_tp1(mut self, tp1: f64) -> Self {
        self.moteur = self.moteur.avec_tp1(tp1);
        self
    }

    /// TP2 réglable (Paramètres › stratégies › SMC, défaut 2.0).
    pub fn avec_tp2(mut self, tp2: f64) -> Self {
        self.moteur = self.moteur.avec_tp2(tp2);
        self
    }

    /// TP3 réglable propriétaire (liquidité lointaine / R fixe + repli).
    pub fn avec_tp3_reglage(mut self, reglage: smc::v12::signals::Tp3Reglage) -> Self {
        self.moteur = self.moteur.avec_tp3_reglage(reglage);
        self
    }

    /// Trailing stop après TP2 (Paramètres › SMC, inactif par défaut).
    pub fn avec_trailing_tp2(mut self, k: Option<f64>) -> Self {
        self.moteur = self.moteur.avec_trailing_tp2(k);
        self
    }

    /// Bonus de scoring BPR (étude Module A — défaut actif = étalon Pine).
    pub fn avec_scoring_bpr(mut self, actif: bool) -> Self {
        self.moteur = self.moteur.avec_scoring_bpr(actif);
        self
    }

    /// Bonus Module F — sessions H/L (étude Phase 4).
    pub fn avec_scoring_sessions(mut self, actif: bool) -> Self {
        self.moteur = self.moteur.avec_scoring_sessions(actif);
        self
    }

    /// Bonus Module H — mega-orders volume (étude Phase 5).
    pub fn avec_scoring_mega_volume(mut self, actif: bool) -> Self {
        self.moteur = self.moteur.avec_scoring_mega_volume(actif);
        self
    }

    /// R1 — sweep requis en qualification v11 (étude étape 3).
    pub fn avec_sweep_requis(mut self, actif: bool) -> Self {
        self.moteur = self.moteur.avec_sweep_requis(actif);
        self
    }

    /// R2 — porte P/D directionnel en qualification v11 (étude étape 3).
    pub fn avec_pd_requis(mut self, actif: bool) -> Self {
        self.moteur = self.moteur.avec_pd_requis(actif);
        self
    }

    /// R4 — confluences MTF sur HTF clôturé seul (étude étape 3).
    pub fn avec_mtf_cloture(mut self, actif: bool) -> Self {
        self.moteur = self.moteur.avec_mtf_cloture(actif);
        self
    }

    /// R5 — confluences MTF à containment directionnel (étude étape 3).
    pub fn avec_mtf_directionnel(mut self, actif: bool) -> Self {
        self.moteur = self.moteur.avec_mtf_directionnel(actif);
        self
    }

    /// Étape 4 — multiplicateurs de niveaux (SL offset, TP1, TP2).
    pub fn avec_multiplicateurs(mut self, sl: f64, tp1: f64, tp2: f64) -> Self {
        self.moteur = self.moteur.avec_multiplicateurs(sl, tp1, tp2);
        self
    }

    /// Étape 4 — BE automatique à seuil de MFE.
    pub fn avec_be_auto(mut self, seuil: Option<f64>) -> Self {
        self.moteur = self.moteur.avec_be_auto(seuil);
        self
    }

    /// Applique l'amorce une seule fois, au premier bar vu (t0 = son ts).
    fn appliquer_amorce_si_premiere(&mut self, ts: i64) {
        if self.amorce_appliquee {
            return;
        }
        self.amorce_appliquee = true;
        if let Some(a) = self.amorce.take() {
            self.moteur.primer_mtf_amorce(&a, ts);
        }
    }

    /// Carnet de trades confirmé (diagnostic / tests).
    pub fn livre_trades(&self) -> &[Trade] {
        &self.moteur.signals.trades
    }

    /// Barre live depuis la bougie en formation (`close` = dernier prix).
    fn bar_live(debut: i64, f: &BougieEnFormation) -> BarInput {
        BarInput {
            timestamp: debut,
            open: f.open,
            high: f.high,
            low: f.low,
            close: f.close,
            volume: f.volume,
        }
    }

    /// Barre confirmée depuis une bougie clôturée.
    fn bar_confirmee(c: &Candle) -> BarInput {
        BarInput {
            timestamp: c.timestamp.timestamp(),
            open: c.open,
            high: c.high,
            low: c.low,
            close: c.close,
            volume: c.volume,
        }
    }
}

impl Engine for MoteurV12 {
    fn nom(&self) -> &str {
        NOM
    }

    fn on_tick(&mut self, ctx: &ContexteTick) -> SortieMoteur {
        // Rollback Pine : évaluation de la bougie live sur un CLONE de
        // l'état confirmé — l'état commité n'est jamais corrompu.
        // ANNONCES d'imminence : dès que l'évaluation live crée le trade,
        // le message part (décision propriétaire 23/08 — sans attendre la
        // clôture ; le Pine, lui, crée le trade sur `barstate.isconfirmed`
        // et l'insertion en base suit cette règle). Une seule annonce par
        // trade, même si le prix oscillait au bord de la zone.
        // Les événements lifecycle (fill/TP/SL), eux, restent intrabar —
        // comme l'exécution Pine temps réel (`low`/`high` cumulatifs).
        let bar = Self::bar_live(ctx.bougie.debut, ctx.bougie);
        self.appliquer_amorce_si_premiere(bar.timestamp);
        let mut eval = self.moteur.clone();
        eval.update(&bar);
        let signaux = extraire_annonces(
            &mut self.annonces,
            &self.emis,
            &eval.signals.trades,
            self.asset.clone(),
            self.tf,
            ctx.bougie.debut,
        );
        let evenements = diff_lifecycle(
            &mut self.vus,
            &eval.signals.trades,
            self.asset.clone(),
            self.tf,
            ctx.bougie.debut,
        );
        SortieMoteur {
            signaux,
            evenements,
        }
    }

    fn on_close(&mut self, ctx: &ContexteCloture) -> SortieMoteur {
        // Commit autoritaire : la bougie officielle alimente le moteur réel.
        // Création du trade à la confirmation (Pine `barstate.isconfirmed`)
        // → signal officiel (ligne en base). `deja_annonce` distingue ceux
        // dont le message d'imminence est déjà parti intrabar.
        let bar = Self::bar_confirmee(ctx.bougie);
        self.appliquer_amorce_si_premiere(bar.timestamp);
        self.moteur.update(&bar);
        let signaux = extraire_nouveaux(
            &mut self.emis,
            &self.annonces,
            &self.moteur.signals.trades,
            self.asset.clone(),
            self.tf,
            ctx.bougie.timestamp.timestamp(),
        );
        let evenements = diff_lifecycle(
            &mut self.vus,
            &self.moteur.signals.trades,
            self.asset.clone(),
            self.tf,
            ctx.bougie.timestamp.timestamp(),
        );
        SortieMoteur {
            signaux,
            evenements,
        }
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[path = "engine_tests.rs"]
mod tests;

#[cfg(test)]
impl MoteurV12 {
    /// Représentation comparable du carnet confirmé (les tests comparent
    /// les chemins tick vs clôture).
    fn livre_trades_dbg(&self) -> String {
        format!("{:?}", self.moteur.signals.trades)
    }
}

mod lifecycle_diff;
use lifecycle_diff::{diff_lifecycle, extraire_annonces, extraire_nouveaux, EtatVu};
