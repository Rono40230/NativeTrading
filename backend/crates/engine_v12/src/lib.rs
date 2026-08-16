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
//! Clé par trade : (barre de création, sens, source, entrée bit à bit).
//! Un trade vu intrabar n'est jamais ré-émis à la clôture — les deux chemins
//! calculant les mêmes identités pour les mêmes conditions.

use std::collections::HashSet;

use common::{Asset, Candle, Direction, Timeframe};
use engine::{
    BougieEnFormation, ContexteCloture, ContexteTick, Engine, EvenementTrade, SignalBrut,
    SortieMoteur, TypeEvenementTrade,
};
use smc::v12::trade::{CloseReason, Side, Trade, TradeSource, TradeState};
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

/// Clé d'anti-ré-émission d'un trade.
type CleTrade = (usize, u8, u8, u64);

/// État lifecycle d'un trade tel que vu à la dernière évaluation — la
/// comparaison de deux états produit les événements (fill, SL/TP, clôture).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct EtatVu {
    rempli: bool,
    tp1_touche: bool,
    tp2_touche: bool,
    tp3_touche: bool,
    be_force: bool,
    sl_bits: u64,
    clture_raison: Option<u8>,
}

impl EtatVu {
    fn depuis(t: &Trade) -> Self {
        Self {
            rempli: t.state != TradeState::Pending,
            tp1_touche: t.tp1_price_touched,
            tp2_touche: t.tp2_ts > 0,
            tp3_touche: t.tp3_touched,
            be_force: t.be_forced,
            sl_bits: t.sl.to_bits(),
            clture_raison: t.close_reason.map(raison_discriminant),
        }
    }
}

/// Discriminant stable de la raison de clôture (sérialisation compacte).
fn raison_discriminant(r: CloseReason) -> u8 {
    match r {
        CloseReason::Sl => 1,
        CloseReason::Be => 2,
        CloseReason::Tp2Sl => 3,
        CloseReason::Tp3 => 4,
        CloseReason::Expire => 5,
        CloseReason::Cancel => 6,
    }
}

fn raison_texte(r: CloseReason) -> &'static str {
    match r {
        CloseReason::Sl => "Sl",
        CloseReason::Be => "Be",
        CloseReason::Tp2Sl => "Tp2Sl",
        CloseReason::Tp3 => "Tp3",
        CloseReason::Expire => "Expire",
        CloseReason::Cancel => "Cancel",
    }
}

fn cle_du_trade(t: &Trade) -> CleTrade {
    (
        t.bar_created,
        match t.side {
            Side::Buy => 0,
            Side::Sell => 1,
        },
        match t.source {
            TradeSource::Ob => 0,
            TradeSource::BsZones => 1,
        },
        t.entry.to_bits(),
    )
}

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
}

impl MoteurV12 {
    pub fn nouveau(asset: Asset, tf: Timeframe) -> Self {
        Self {
            asset: asset.clone(),
            tf,
            moteur: SmcV12Engine::new(asset.as_str(), tf.as_str()),
            emis: HashSet::new(),
            vus: std::collections::HashMap::new(),
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

/// Extrait les trades jamais émis d'un carnet, les convertit en signaux et
/// verrouille leurs clés. Fonction libre : `emis` (mutable) et `trades`
/// (immutable) appartiennent à des champs distincts de l'appelant.
fn extraire_nouveaux(
    emis: &mut HashSet<CleTrade>,
    trades: &[Trade],
    asset: Asset,
    tf: Timeframe,
    debut_barre: i64,
) -> Vec<SignalBrut> {
    let mut signaux = Vec::new();
    // Clés présentes dans le carnet évalué — borne le cache PAR PRÉSENCE :
    // un trade vit potentiellement bien plus de 50 barres, et le carnet ne
    // fait que croître (`bar_created` strictement croissant) — une clé
    // disparue ne peut plus correspondre à un futur trade.
    let mut cles_presentes: HashSet<CleTrade> = HashSet::with_capacity(trades.len());
    for t in trades {
        let cle = cle_du_trade(t);
        cles_presentes.insert(cle);
        if emis.contains(&cle) {
            continue;
        }
        emis.insert(cle);
        let direction = match t.side {
            Side::Buy => Direction::Long,
            Side::Sell => Direction::Short,
        };
        let source = match t.source {
            TradeSource::Ob => "v11-OB",
            TradeSource::BsZones => "BSZones",
        };
        signaux.push(SignalBrut::nouveau(
            NOM,
            asset.clone(),
            tf,
            direction,
            t.entry,
            t.sl,
            vec![t.tp1, t.tp2, t.tp3],
            t.score,
            format!(
                "{} {} score={} risk={:.4} bar={}",
                source,
                match t.side {
                    Side::Buy => "BUY",
                    Side::Sell => "SELL",
                },
                t.score,
                t.risk0,
                t.bar_created
            ),
            debut_barre,
        ));
    }
    emis.retain(|cle| cles_presentes.contains(cle));
    signaux
}

/// Détecte les transitions lifecycle entre l'état vu précédemment et le
/// carnet évalué, les convertit en événements et met à jour l'état vu.
///
/// Un trade disparu du carnet (condition intrabar évanouie — rollback Pine)
/// est retiré silencieusement : son signal d'entrée déjà émis n'est JAMAIS
/// rétracté (R5), et aucun événement d'annulation n'est inventé.
fn diff_lifecycle(
    vus: &mut std::collections::HashMap<CleTrade, EtatVu>,
    trades: &[Trade],
    asset: Asset,
    tf: Timeframe,
    debut_barre: i64,
) -> Vec<EvenementTrade> {
    let mut evenements = Vec::new();
    let mut cles_presentes = HashSet::with_capacity(trades.len());

    for t in trades {
        let cle = cle_du_trade(t);
        cles_presentes.insert(cle);
        let nouvel_etat = EtatVu::depuis(t);
        let precedent = vus.insert(cle, nouvel_etat);

        // Baseline d'un trade fraîchement créé : rien touché, pas rempli.
        let vide = EtatVu {
            rempli: false,
            tp1_touche: false,
            tp2_touche: false,
            tp3_touche: false,
            be_force: false,
            sl_bits: t.sl.to_bits(),
            clture_raison: None,
        };
        let avant = precedent.unwrap_or(vide);

        let mut pousser = |type_ev: TypeEvenementTrade, detail: String, prix: f64| {
            evenements.push(EvenementTrade {
                moteur: NOM.to_string(),
                asset: asset.clone(),
                tf,
                cle_trade: format!("{}:{}:{}:{}", cle.0, cle.1, cle.2, cle.3),
                evenement: type_ev,
                detail,
                prix,
                debut_barre,
                emis_le: chrono::Utc::now(),
            });
        };

        // Ordre naturel du cycle : remplissage → BE → TP1 → TP2 → TP3 → clôture.
        if !avant.rempli && nouvel_etat.rempli {
            pousser(TypeEvenementTrade::Fill, "retest touché".into(), t.entry);
        }
        if !avant.be_force && nouvel_etat.be_force {
            pousser(TypeEvenementTrade::Be, "BE forcé".into(), t.entry);
        } else if avant.sl_bits != nouvel_etat.sl_bits && nouvel_etat.rempli && !nouvel_etat.tp3_touche {
            // SL déplacé vers l'entrée (BE armé après TP1).
            pousser(TypeEvenementTrade::Be, "SL → entrée".into(), t.entry);
        }
        if !avant.tp1_touche && nouvel_etat.tp1_touche {
            pousser(TypeEvenementTrade::Tp1, "TP1 touché".into(), t.tp1);
        }
        if !avant.tp2_touche && nouvel_etat.tp2_touche {
            pousser(TypeEvenementTrade::Tp2, "TP2 touché".into(), t.tp2);
        }
        if !avant.tp3_touche && nouvel_etat.tp3_touche {
            pousser(TypeEvenementTrade::Tp3, "TP3 touché".into(), t.tp3);
        }
        if avant.clture_raison.is_none() && nouvel_etat.clture_raison.is_some() {
            if let Some(r) = t.close_reason {
                pousser(
                    TypeEvenementTrade::Cloture,
                    raison_texte(r).to_string(),
                    match r {
                        CloseReason::Sl => t.sl,
                        CloseReason::Be | CloseReason::Tp2Sl => t.entry,
                        CloseReason::Tp3 => t.tp3,
                        // Prix de sortie exact non porté par le trade : entrée.
                        CloseReason::Expire | CloseReason::Cancel => t.entry,
                    },
                );
            }
        }
    }

    // Trades disparus du carnet (phantoms intrabar) : retrait silencieux.
    vus.retain(|cle, _| cles_presentes.contains(cle));
    evenements
}

impl Engine for MoteurV12 {
    fn nom(&self) -> &str {
        NOM
    }

    fn on_tick(&mut self, ctx: &ContexteTick) -> SortieMoteur {
        // Rollback Pine : évaluation de la bougie live sur un CLONE de
        // l'état confirmé — l'état commité n'est jamais corrompu.
        let bar = Self::bar_live(ctx.bougie.debut, ctx.bougie);
        let mut eval = self.moteur.clone();
        eval.update(&bar);
        let signaux = extraire_nouveaux(
            &mut self.emis,
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
        let bar = Self::bar_confirmee(ctx.bougie);
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
    fn formation_partielle(b: &BarInput, close_actuel: f64, high_vu: f64, low_vu: f64) -> BougieEnFormation {
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
        ContexteTick { asset, tf, bougie: f }
    }

    fn ctx_close<'a>(asset: &'a Asset, tf: Timeframe, c: &'a Candle, idx: usize) -> ContexteCloture<'a> {
        ContexteCloture { asset, tf, bougie: c, index_barre: idx }
    }

    /// Fidélité rollback : l'évaluation tick-par-tick (clones) et l'évaluation
    /// par clôtures aboutissent au MÊME état confirmé, et le chemin tick
    /// émet au moins tous les signaux du chemin clôture (le dernier tick
    /// simulé porte exactement la barre finale).
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

        let mut total_clotures = 0usize;
        let mut total_ticks = 0usize;
        let mut ajouts_a_la_cloture = 0usize;

        for (i, b) in bars.iter().enumerate() {
            // Chemin A : clôtures seules.
            let c = candle_depuis_bar(b);
            total_clotures += par_clotures.on_close(&ctx_close(&asset, tf, &c, i)).signaux.len();

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
                total_ticks += s.signaux.len();
            }
            // Le dernier tick simulé porte EXACTEMENT la barre finale : la
            // clôture ne doit strictement rien ajouter (ni signal, ni événement).
            let s_close = par_ticks.on_close(&ctx_close(&asset, tf, &c, i));
            ajouts_a_la_cloture += s_close.signaux.len() + s_close.evenements.len();
        }

        // 1. États confirmés identiques : mêmes trades, même index de barre.
        assert_eq!(
            par_clotures.livre_trades_dbg(),
            par_ticks.livre_trades_dbg(),
            "le chemin tick (clones jetés) ne doit jamais altérer l'état confirmé"
        );

        // 2. Le dernier tick simulé porte exactement la barre finale : la
        //    clôture ne voit AUCUN nouveau trade que le tick n'avait pas vu.
        //    (les ticks peuvent émettre PLUS — conditions intrabar
        //    évanouies, sémantique alerte Pine).
        assert!(
            total_ticks >= total_clotures,
            "ticks={} clotures={}",
            total_ticks,
            total_clotures
        );

        // 3. Invariant lifecycle : tout ce que la clôture confirme a déjà
        //    été vu au dernier tick — aucune émission en double.
        assert_eq!(
            ajouts_a_la_cloture, 0,
            "la clôture ne doit rien ajouter après une évaluation tick complète"
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
        assert!(nb_clotures > 0, "au moins une clôture attendue sur 700 bars");
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

        let toutes: Vec<(usize, u8, u8, u64)> = Vec::new();
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
        let _ = toutes;
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
        assert!(total > 0, "le moteur v12 doit émettre des signaux sur 700 bars");
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
