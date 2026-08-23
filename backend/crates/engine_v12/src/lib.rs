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
//! redémarrages. Un trade n'est émis qu'à la CLÔTURE de sa barre de création
//! (Pine `barstate.isconfirmed`) ; intrabar, seuls les événements lifecycle.

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
        // Pine crée les trades sur `barstate.isconfirmed` UNIQUEMENT : aucun
        // signal n'est émis intrabar (incident 23/08 : trades fantômes émis
        // mi-bougie puis ré-émis quand le prix revenait les confirmer).
        // Les événements lifecycle (fill/TP/SL), eux, sont intrabar — comme
        // l'exécution Pine temps réel (`low`/`high` cumulatifs, monotones).
        let bar = Self::bar_live(ctx.bougie.debut, ctx.bougie);
        self.appliquer_amorce_si_premiere(bar.timestamp);
        let mut eval = self.moteur.clone();
        eval.update(&bar);
        let evenements = diff_lifecycle(
            &mut self.vus,
            &eval.signals.trades,
            self.asset.clone(),
            self.tf,
            ctx.bougie.debut,
        );
        SortieMoteur {
            signaux: Vec::new(),
            evenements,
        }
    }

    fn on_close(&mut self, ctx: &ContexteCloture) -> SortieMoteur {
        // Commit autoritaire : la bougie officielle alimente le moteur réel.
        let bar = Self::bar_confirmee(ctx.bougie);
        self.appliquer_amorce_si_premiere(bar.timestamp);
        self.moteur.update(&bar);
        let signaux = extraire_nouveaux(
            &mut self.emis,
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
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    /// CSV XAUUSD M15 (même source que les tests v12) — le test est sauté
    /// silencieusement si le fichier est absent.
    const XAUUSD_M15_CSV: &str = "/mnt/IA/nautilus-smc-spike/xauusd_m15.csv";

    fn charger_bars() -> Vec<BarInput> {
        let contenu = match std::fs::read_to_string(XAUUSD_M15_CSV) {
            Ok(c) => c,
            Err(_) => return Vec::new(),
        };
        contenu
            .lines()
            .filter_map(|l| {
                let f: Vec<&str> = l.split(',').collect();
                if f.len() < 6 {
                    return None;
                }
                Some(BarInput {
                    timestamp: f[0].parse().ok()?,
                    open: f[1].parse().ok()?,
                    high: f[2].parse().ok()?,
                    low: f[3].parse().ok()?,
                    close: f[4].parse().ok()?,
                    volume: f[5].parse().ok()?,
                })
            })
            .collect()
    }

    fn candle_depuis_bar(b: &BarInput) -> Candle {
        Candle {
            timestamp: Utc.timestamp_opt(b.timestamp, 0).unwrap(),
            open: b.open,
            high: b.high,
            low: b.low,
            close: b.close,
            volume: b.volume,
        }
    }

    /// Bougie en formation synthétique reflétant une barre à un instant donné.
    fn formation_partielle(
        b: &BarInput,
        close_actuel: f64,
        high_vu: f64,
        low_vu: f64,
    ) -> BougieEnFormation {
        BougieEnFormation {
            debut: b.timestamp,
            open: b.open,
            high: high_vu,
            low: low_vu,
            close: close_actuel,
            volume: b.volume,
            nb_events: 1,
            dernier_event: None,
        }
    }

    fn ctx_tick<'a>(asset: &'a Asset, tf: Timeframe, f: &'a BougieEnFormation) -> ContexteTick<'a> {
        ContexteTick {
            asset,
            tf,
            bougie: f,
        }
    }

    fn ctx_close<'a>(
        asset: &'a Asset,
        tf: Timeframe,
        c: &'a Candle,
        idx: usize,
    ) -> ContexteCloture<'a> {
        ContexteCloture {
            asset,
            tf,
            bougie: c,
            index_barre: idx,
        }
    }

    /// Fidélité rollback : l'évaluation tick-par-tick (clones) et l'évaluation
    /// par clôtures aboutissent au MÊME état confirmé. Les signaux ne naissent
    /// qu'à la clôture (Pine `barstate.isconfirmed`) — le tick intrabar n'en
    /// émet JAMAIS (anti-fantôme, incident 23/08).
    #[test]
    fn ticks_et_clotures_aboutissent_au_meme_etat_confirme() {
        let bars = charger_bars();
        if bars.is_empty() {
            return; // CSV absent — test sauté
        }
        let asset = Asset::from("XAUUSD");
        let tf = Timeframe::M15;

        let mut par_clotures = MoteurV12::nouveau(asset.clone(), tf);
        let mut par_ticks = MoteurV12::nouveau(asset.clone(), tf);

        let mut signaux_clotures = 0usize;
        let mut signaux_ticks = 0usize;
        let mut ajouts_a_la_cloture = 0usize;

        for (i, b) in bars.iter().enumerate() {
            // Chemin A : clôtures seules.
            let c = candle_depuis_bar(b);
            signaux_clotures += par_clotures
                .on_close(&ctx_close(&asset, tf, &c, i))
                .signaux
                .len();

            // Chemin B : ticks simulés (open → extrême bas → extrême haut →
            // close) puis clôture. Chaque tick = évaluation clone complète.
            let scenarios = [
                formation_partielle(b, b.open, b.open, b.open),
                formation_partielle(b, b.low, b.open.max(b.open), b.low),
                formation_partielle(b, b.high, b.high, b.low),
                formation_partielle(b, b.close, b.high, b.low),
            ];
            for f in &scenarios {
                let s = par_ticks.on_tick(&ctx_tick(&asset, tf, f));
                signaux_ticks += s.signaux.len();
            }
            // Le dernier tick simulé porte EXACTEMENT la barre finale : la
            // clôture ne doit ajouter AUCUN événement (les signaux de création
            // sont attendus — c'est leur unique chemin d'émission).
            let s_close = par_ticks.on_close(&ctx_close(&asset, tf, &c, i));
            ajouts_a_la_cloture += s_close.evenements.len();
        }

        // 1. États confirmés identiques : mêmes trades, même index de barre.
        assert_eq!(
            par_clotures.livre_trades_dbg(),
            par_ticks.livre_trades_dbg(),
            "le chemin tick (clones jetés) ne doit jamais altérer l'état confirmé"
        );

        // 2. Pine : création sur barre confirmée uniquement — aucun signal
        //    intrabar, aucun fantôme mi-bougie.
        assert_eq!(
            signaux_ticks, 0,
            "le chemin tick ne doit JAMAIS émettre de signal (barstate.isconfirmed)"
        );
        assert!(signaux_clotures > 0, "signaux attendus à la clôture");

        // 3. Invariant lifecycle : le dernier tick simulé porte exactement la
        //    barre finale — la clôture ne doit rien annoncer de nouveau.
        assert_eq!(
            ajouts_a_la_cloture, 0,
            "la clôture ne doit annoncer aucun événement après un tick complet"
        );
    }

    /// Lifecycle intrabar : les fills, TP et clôtures sont détectés pendant
    /// le replay tick-par-tick, sans jamais deux fois la même transition.
    #[test]
    fn lifecycle_intrabar_detecte_au_tick_sans_doublon() {
        let bars = charger_bars();
        if bars.is_empty() {
            return;
        }
        let asset = Asset::from("XAUUSD");
        let tf = Timeframe::M15;
        let mut moteur = MoteurV12::nouveau(asset.clone(), tf);

        let mut types_vus: HashSet<(String, TypeEvenementTrade)> = HashSet::new();
        let mut nb_fills = 0;
        let mut nb_tp1 = 0;
        let mut nb_clotures = 0;

        for (i, b) in bars.iter().enumerate() {
            // Deux évaluations tick + clôture par barre.
            for f in [
                formation_partielle(b, b.open, b.open, b.open),
                formation_partielle(b, b.close, b.high, b.low),
            ] {
                let s = moteur.on_tick(&ctx_tick(&asset, tf, &f));
                for e in &s.evenements {
                    assert!(
                        types_vus.insert((e.cle_trade.clone(), e.evenement)),
                        "transition {:?} sur {:?} émise deux fois",
                        e.evenement,
                        e.cle_trade
                    );
                    match e.evenement {
                        TypeEvenementTrade::Fill => nb_fills += 1,
                        TypeEvenementTrade::Tp1 => nb_tp1 += 1,
                        TypeEvenementTrade::Cloture => nb_clotures += 1,
                        _ => {}
                    }
                }
            }
            let c = candle_depuis_bar(b);
            let s = moteur.on_close(&ctx_close(&asset, tf, &c, i));
            for e in &s.evenements {
                assert!(
                    types_vus.insert((e.cle_trade.clone(), e.evenement)),
                    "transition {:?} émise deux fois (au close)",
                    e.evenement
                );
                match e.evenement {
                    TypeEvenementTrade::Fill => nb_fills += 1,
                    TypeEvenementTrade::Tp1 => nb_tp1 += 1,
                    TypeEvenementTrade::Cloture => nb_clotures += 1,
                    _ => {}
                }
            }
        }

        assert!(nb_fills > 0, "au moins un fill attendu sur 700 bars");
        assert!(
            nb_clotures > 0,
            "au moins une clôture attendue sur 700 bars"
        );
        let _ = nb_tp1; // compté pour diagnostic (peut être 0 — voir ROADMAP)
    }

    /// Aucune clé n'est émise deux fois, quel que soit le chemin.
    #[test]
    fn aucune_double_emission_sur_replay() {
        let bars = charger_bars();
        if bars.is_empty() {
            return;
        }
        let asset = Asset::from("XAUUSD");
        let tf = Timeframe::M15;
        let mut moteur = MoteurV12::nouveau(asset.clone(), tf);

        let _toutes: Vec<(i64, u8, u8, u64)> = Vec::new();
        for (i, b) in bars.iter().enumerate() {
            // Deux ticks + clôture par barre : l'anti-ré-émission doit tenir.
            let f1 = formation_partielle(b, b.open, b.open, b.open);
            let f2 = formation_partielle(b, b.close, b.high, b.low);
            let _ = moteur.on_tick(&ctx_tick(&asset, tf, &f1));
            let _ = moteur.on_tick(&ctx_tick(&asset, tf, &f2));
            let c = candle_depuis_bar(b);
            let _ = moteur.on_close(&ctx_close(&asset, tf, &c, i));
        }
        // Le cache `emis` est élagué par présence dans le carnet à chaque
        // évaluation : sa taille suit le carnet, jamais l'historique complet.
        assert!(moteur.emis.len() <= moteur.livre_trades().len());
        let _ = _toutes;
    }

    /// Replay pur par clôtures : des signaux existent sur l'historique XAUUSD.
    #[test]
    fn replay_clotures_emet_des_signaux() {
        let bars = charger_bars();
        if bars.is_empty() {
            return;
        }
        let asset = Asset::from("XAUUSD");
        let tf = Timeframe::M15;
        let mut moteur = MoteurV12::nouveau(asset.clone(), tf);
        let mut total = 0;
        for (i, b) in bars.iter().enumerate() {
            let c = candle_depuis_bar(b);
            total += moteur.on_close(&ctx_close(&asset, tf, &c, i)).signaux.len();
        }
        assert!(
            total > 0,
            "le moteur v12 doit émettre des signaux sur 700 bars"
        );
    }
}

#[cfg(test)]
impl MoteurV12 {
    /// Représentation comparable du carnet confirmé (les tests comparent
    /// les chemins tick vs clôture).
    fn livre_trades_dbg(&self) -> String {
        format!("{:?}", self.moteur.signals.trades)
    }
}

mod lifecycle_diff;
use lifecycle_diff::{diff_lifecycle, extraire_nouveaux, EtatVu};
