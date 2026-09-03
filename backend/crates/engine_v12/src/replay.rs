//! Replay harness (Phase 2.5 ROADMAP) — rejouer des semaines d'historique
//! par le chemin du moteur v12 et journaliser signaux + événements.
//!
//! Deux modes :
//! - **clôtures seules** (parité) : c'est le mode bar-replay de TradingView —
//!   le journal doit correspondre à la référence (moteur nu nourri en
//!   bar-replay). C'est le critère de la Gate 2 ;
//! - **avec ticks simulés** (alertes) : chaque bougie est parcourue en
//!   4 évaluations intrabar — le journal montre ce que le LIVE émettrait
//!   (sur-ensemble : conditions intrabar évanouies, sémantique alerte).
//!
//! Dans les deux modes, un moteur nu de référence rejoue les mêmes bougies
//! en bar-replay : `conforme_reference` vérifie que le chemin plugin
//! n'altère JAMAIS l'état confirmé (même carnet final bit à bit).

use common::{Asset, Candle, Timeframe};
use engine::{Engine, SortieMoteur};
use smc::v12::SmcV12Engine;

use crate::{bar_input_depuis_bougie, formation_depuis_bar, MoteurV12};

/// Résultat complet d'un replay.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ResultatReplay {
    pub asset: String,
    pub timeframe: String,
    pub nb_bougies: usize,
    pub periode_de: i64,
    pub periode_a: i64,
    /// Mode avec évaluations intrabar simulées.
    pub simule_ticks: bool,
    pub signaux: Vec<engine::SignalBrut>,
    pub evenements: Vec<engine::EvenementTrade>,
    /// Le carnet final du plugin == carnet du moteur nu de référence.
    pub conforme_reference: bool,
    pub nb_trades_reference: usize,
    pub duree_ms: u128,
}

/// Modes d'étude du replay — chaque champ = un levier A/B testé par une
/// campagne (valeur par défaut = production au moment de la refonte 29/08).
/// Un wrapper public par étude ne mute QUE son levier : plus de paramètres
/// positionnels qui s'empilent (source d'erreurs de patch).
#[derive(Clone)]
pub struct ModesEtude {
    /// BE forcé sur BOS opposé (Classique = fidèle Pine ; Supprime = décision 26/08 des études).
    pub be: smc::v12::lifecycle::ModeBeForce,
    /// TP3 (DolCappe3R = production, décision DoL≤3R 28/08).
    pub tp3: smc::v12::signals::ModeTp3,
    /// Bonus BPR (inactif — étude 28/08 : +1.0R = bruit).
    pub scoring_bpr: bool,
    /// Bonus sessions H/L (inactif — étude 28/08 : ON ≡ OFF bit-à-bit).
    pub scoring_sessions: bool,
    /// Bonus mega-orders volume ≥ 2× SMA20 (actif — étude 28/08 : +21.3R).
    pub scoring_mega: bool,
    /// Porte sweep frais requis (inactif — étude 29/08 : −577.3R).
    pub sweep_requis: bool,
    /// Porte P/D directionnel (R2 — rejetée 29/08 : −47.9R).
    pub pd_requis: bool,
    /// Confluences MTF sur HTF clôturé seul (R4, étude en cours).
    pub mtf_cloture: bool,
    /// Confluences MTF à containment directionnel (R5, étude en cours).
    pub mtf_directionnel: bool,
    /// Étape 4 — multiplicateur de l'offset SL (1.0 = production).
    pub sl_mult: f64,
    /// Étape 4 — TP1 = entry ± tp1_mult × r (1.0 = production).
    pub tp1_mult: f64,
    /// Étape 4 — TP2 = entry ± tp2_mult × r (2.0 = production).
    pub tp2_mult: f64,
    /// Étape 4 — BE auto à seuil de MFE (None = production).
    pub be_auto: Option<f64>,
}

impl Default for ModesEtude {
    fn default() -> Self {
        Self {
            be: smc::v12::lifecycle::ModeBeForce::Classique,
            tp3: smc::v12::signals::ModeTp3::DolCappe3R,
            scoring_bpr: false,
            scoring_sessions: false,
            scoring_mega: true,
            sweep_requis: false,
            pd_requis: false,
            mtf_cloture: false,
            mtf_directionnel: false,
            sl_mult: 0.75, // étape 4 29/08 : production
            tp1_mult: 0.6,  // étape 4 29/08 : production
            tp2_mult: 2.0,
            be_auto: None,
        }
    }
}

/// Base des études « production » : BE Supprimé + TP3 DoL≤3R + défauts actuels.
fn modes_production() -> ModesEtude {
    ModesEtude {
        be: smc::v12::lifecycle::ModeBeForce::Supprime,
        ..Default::default()
    }
}

/// Rejoue `bougies` (ordre chronologique) dans un `MoteurV12` neuf et
/// compare l'état final à un moteur nu de référence.
pub fn rejouer_bougies(
    asset: Asset,
    tf: Timeframe,
    bougies: &[Candle],
    simuler_ticks: bool,
    amorce: smc::v12::AmorceMtf,
) -> ResultatReplay {
    rejouer_bougies_modes(asset, tf, bougies, simuler_ticks, amorce, &ModesEtude::default())
}

/// Variante paramétrée (étude comparatif du BE forcé) : rejoue l'historique
/// avec un mode de gestion du BOS opposé donné.
pub fn rejouer_bougies_mode(
    asset: Asset,
    tf: Timeframe,
    bougies: &[Candle],
    simuler_ticks: bool,
    amorce: smc::v12::AmorceMtf,
    mode: smc::v12::lifecycle::ModeBeForce,
) -> ResultatReplay {
    let modes = ModesEtude {
        be: mode,
        tp3: smc::v12::signals::ModeTp3::Dol,
        ..Default::default()
    };
    rejouer_bougies_modes(asset, tf, bougies, simuler_ticks, amorce, &modes)
}

/// Variante Module G (étude TP3 DoL vs 3R fixe) — BE = production (Supprimé).
pub fn rejouer_bougies_tp3(
    asset: Asset,
    tf: Timeframe,
    bougies: &[Candle],
    simuler_ticks: bool,
    amorce: smc::v12::AmorceMtf,
    mode_tp3: smc::v12::signals::ModeTp3,
) -> ResultatReplay {
    let modes = ModesEtude {
        tp3: mode_tp3,
        ..modes_production()
    };
    rejouer_bougies_modes(asset, tf, bougies, simuler_ticks, amorce, &modes)
}

/// Variante Module A (étude BPR) — BE = Supprimé + TP3 = DolCappe3R
/// (production), seul le bonus de scoring BPR diffère.
pub fn rejouer_bougies_bpr(
    asset: Asset,
    tf: Timeframe,
    bougies: &[Candle],
    simuler_ticks: bool,
    amorce: smc::v12::AmorceMtf,
    scoring_bpr: bool,
) -> ResultatReplay {
    let modes = ModesEtude {
        scoring_bpr,
        ..modes_production()
    };
    rejouer_bougies_modes(asset, tf, bougies, simuler_ticks, amorce, &modes)
}

/// Variante Module F (étude sessions H/L) — BE = Supprimé + TP3 = DoL≤3R
/// (production), seul le bonus Sessions H/L diffère.
pub fn rejouer_bougies_sessions(
    asset: Asset,
    tf: Timeframe,
    bougies: &[Candle],
    simuler_ticks: bool,
    amorce: smc::v12::AmorceMtf,
    scoring_sessions: bool,
) -> ResultatReplay {
    let modes = ModesEtude {
        scoring_sessions,
        ..modes_production()
    };
    rejouer_bougies_modes(asset, tf, bougies, simuler_ticks, amorce, &modes)
}

/// Variante Module H (étude mega-orders) — production + seul le bonus
/// volume ≥ 2× SMA20 diffère.
pub fn rejouer_bougies_mega(
    asset: Asset,
    tf: Timeframe,
    bougies: &[Candle],
    simuler_ticks: bool,
    amorce: smc::v12::AmorceMtf,
    scoring_mega: bool,
) -> ResultatReplay {
    let modes = ModesEtude {
        scoring_mega,
        ..modes_production()
    };
    rejouer_bougies_modes(asset, tf, bougies, simuler_ticks, amorce, &modes)
}

/// Variante R1 (étude sweep requis) — production + seule la porte sweep diffère.
pub fn rejouer_bougies_sweep(
    asset: Asset,
    tf: Timeframe,
    bougies: &[Candle],
    simuler_ticks: bool,
    amorce: smc::v12::AmorceMtf,
    sweep_requis: bool,
) -> ResultatReplay {
    let modes = ModesEtude {
        sweep_requis,
        ..modes_production()
    };
    rejouer_bougies_modes(asset, tf, bougies, simuler_ticks, amorce, &modes)
}

/// Variante R2 (étude P/D directionnel) — production + seule la porte P/D diffère.
pub fn rejouer_bougies_pd(
    asset: Asset,
    tf: Timeframe,
    bougies: &[Candle],
    simuler_ticks: bool,
    amorce: smc::v12::AmorceMtf,
    pd_requis: bool,
) -> ResultatReplay {
    let modes = ModesEtude {
        pd_requis,
        ..modes_production()
    };
    rejouer_bougies_modes(asset, tf, bougies, simuler_ticks, amorce, &modes)
}

/// Variante TP1 réglable (Paramètres › stratégies › SMC) — production, seul
/// TP1 diffère. Clôtures seules (bar-replay), comme les études A-B.
pub fn rejouer_bougies_tp1(
    asset: Asset,
    tf: Timeframe,
    bougies: &[Candle],
    amorce: smc::v12::AmorceMtf,
    tp1: f64,
) -> ResultatReplay {
    let modes = ModesEtude {
        tp1_mult: tp1,
        ..modes_production()
    };
    rejouer_bougies_modes(asset, tf, bougies, false, amorce, &modes)
}

/// Chemin commun : tous les leviers d'étude portés par [`ModesEtude`].
pub fn rejouer_bougies_modes(
    asset: Asset,
    tf: Timeframe,
    bougies: &[Candle],
    simuler_ticks: bool,
    amorce: smc::v12::AmorceMtf,
    modes: &ModesEtude,
) -> ResultatReplay {
    let debut = std::time::Instant::now();
    let mut plugin = MoteurV12::nouveau(asset.clone(), tf)
        .avec_amorce(amorce.clone())
        .avec_mode_be_force(modes.be)
        .avec_mode_tp3(modes.tp3)
        .avec_scoring_bpr(modes.scoring_bpr)
        .avec_scoring_sessions(modes.scoring_sessions)
        .avec_scoring_mega_volume(modes.scoring_mega)
        .avec_sweep_requis(modes.sweep_requis)
        .avec_pd_requis(modes.pd_requis)
        .avec_mtf_cloture(modes.mtf_cloture)
        .avec_mtf_directionnel(modes.mtf_directionnel)
        .avec_multiplicateurs(modes.sl_mult, modes.tp1_mult, modes.tp2_mult)
        .avec_be_auto(modes.be_auto);
    let mut journal = SortieMoteur::vide();

    for (i, b) in bougies.iter().enumerate() {
        if simuler_ticks {
            // 4 évaluations intrabar : open → extrême bas → extrême haut →
            // clôture (parcours réaliste d'une bougie).
            let bar = bar_input_depuis_bougie(b);
            for f in [
                formation_depuis_bar(&bar, bar.open, bar.open, bar.open),
                formation_depuis_bar(&bar, bar.low, bar.open, bar.low),
                formation_depuis_bar(&bar, bar.high, bar.high, bar.low),
                formation_depuis_bar(&bar, bar.close, bar.high, bar.low),
            ] {
                let ctx = engine::ContexteTick {
                    asset: &asset,
                    tf,
                    bougie: &f,
                };
                journal.etend(plugin.on_tick(&ctx));
            }
        }
        let ctx = engine::ContexteCloture {
            asset: &asset,
            tf,
            bougie: b,
            index_barre: i,
        };
        journal.etend(plugin.on_close(&ctx));
    }

    // Référence : moteur nu, bar-replay pur (le mode TradingView) — même
    // amorçage MTF et MÊMES MODES (BE forcé + TP3 + scoring BPR + sessions
    // H/L) que le plugin, sinon la comparaison conforme_reference compare
    // deux stratégies différentes.
    let mut reference = SmcV12Engine::new(asset.as_str(), tf.as_str())
        .avec_mode_be_force(modes.be)
        .avec_mode_tp3(modes.tp3)
        .avec_scoring_bpr(modes.scoring_bpr)
        .avec_scoring_sessions(modes.scoring_sessions)
        .avec_scoring_mega_volume(modes.scoring_mega)
        .avec_sweep_requis(modes.sweep_requis)
        .avec_pd_requis(modes.pd_requis)
        .avec_mtf_cloture(modes.mtf_cloture)
        .avec_mtf_directionnel(modes.mtf_directionnel)
        .avec_multiplicateurs(modes.sl_mult, modes.tp1_mult, modes.tp2_mult)
        .avec_be_auto(modes.be_auto);
    if let Some(premiere) = bougies.first() {
        reference.primer_mtf_amorce(&amorce, premiere.timestamp.timestamp());
    }

    for b in bougies {
        reference.update(&bar_input_depuis_bougie(b));
    }

    ResultatReplay {
        asset: asset.as_str().to_string(),
        timeframe: tf.as_str().to_string(),
        nb_bougies: bougies.len(),
        periode_de: bougies
            .first()
            .map(|b| b.timestamp.timestamp())
            .unwrap_or(0),
        periode_a: bougies.last().map(|b| b.timestamp.timestamp()).unwrap_or(0),
        simule_ticks: simuler_ticks,
        signaux: journal.signaux,
        evenements: journal.evenements,
        conforme_reference: format!("{:?}", plugin.livre_trades())
            == format!("{:?}", reference.signals.trades),
        nb_trades_reference: reference.signals.trades.len(),
        duree_ms: debut.elapsed().as_millis(),
    }
}

/// Résumé compact d'un replay (comptages par type) — pour logs et API.
pub fn resume(r: &ResultatReplay) -> String {
    use engine::TypeEvenementTrade as T;
    let n = |t: T| r.evenements.iter().filter(|e| e.evenement == t).count();
    format!(
        "{} {} : {} bougies ({}→{}) | signaux={} | fills={} tp1={} tp2={} tp3={} be={} clotures={} | référence={} trades | conforme={} | {} ms [{}]",
        r.asset,
        r.timeframe,
        r.nb_bougies,
        r.periode_de,
        r.periode_a,
        r.signaux.len(),
        n(T::Fill),
        n(T::Tp1),
        n(T::Tp2),
        n(T::Tp3),
        n(T::Be),
        n(T::Cloture),
        r.nb_trades_reference,
        r.conforme_reference,
        r.duree_ms,
        if r.simule_ticks { "ticks simulés" } else { "clôtures" },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::NOM;
    use chrono::TimeZone;

    /// Parité sur des bougies synthétiques en tendance haussière marquée.
    #[test]
    fn replay_clotures_conforme_a_la_reference() {
        let asset = Asset::from("XAUUSD");
        let tf = Timeframe::M15;
        let mut bougies = Vec::new();
        let mut prix = 4000.0;
        for i in 0..600 {
            // Tendance haussière + oscillation pour créer pivots et retests.
            let osc = (i as f64 / 9.0).sin() * 8.0;
            let close = prix + osc;
            let open = prix + osc * 0.6;
            let high = open.max(close) + 3.0;
            let low = open.min(close) - 3.0;
            bougies.push(Candle {
                timestamp: chrono::Utc
                    .timestamp_opt(1_750_000_000 + (i as i64) * 900, 0)
                    .unwrap(),
                open,
                high,
                low,
                close,
                volume: 100.0,
            });
            prix += 0.8;
        }

        let r = rejouer_bougies(asset, tf, &bougies, false, Default::default());
        assert!(
            r.conforme_reference,
            "plugin == moteur nu (chemin clôtures)"
        );
        assert!(r.nb_bougies == 600);
        let s = resume(&r);
        assert!(s.contains("conforme=true"), "{}", s);
    }

    /// CSV XAUUSD M15 réel (même source que les tests v12) — le replay
    /// intrabar détecte des événements lifecycle ET reste conforme.
    #[test]
    fn replay_avec_ticks_emet_des_evenements_intrabar() {
        let bars = charger_bars_csv();
        if bars.is_empty() {
            return; // CSV absent — test sauté
        }
        let asset = Asset::from("XAUUSD");
        let tf = Timeframe::M15;
        let r = rejouer_bougies(asset, tf, &bars, true, Default::default());
        // Intrabar : le journal contient des événements lifecycle.
        assert!(!r.evenements.is_empty(), "fills/TP détectés intrabar");
        // ET l'état final reste conforme (clones jetés).
        assert!(
            r.conforme_reference,
            "les clones n'altèrent pas l'état confirmé"
        );
        assert_eq!(plugin_nom(&r), NOM);
    }

    fn plugin_nom(r: &ResultatReplay) -> &str {
        r.signaux.first().map(|s| s.moteur.as_str()).unwrap_or(NOM)
    }

    /// CSV `timestamp,open,high,low,close,volume` (XAUUSD M15 de référence).
    fn charger_bars_csv() -> Vec<Candle> {
        let contenu = match std::fs::read_to_string("/mnt/IA/nautilus-smc-spike/xauusd_m15.csv") {
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
                Some(Candle {
                    timestamp: chrono::Utc.timestamp_opt(f[0].parse().ok()?, 0).unwrap(),
                    open: f[1].parse().ok()?,
                    high: f[2].parse().ok()?,
                    low: f[3].parse().ok()?,
                    close: f[4].parse().ok()?,
                    volume: f[5].parse().ok()?,
                })
            })
            .collect()
    }
}
