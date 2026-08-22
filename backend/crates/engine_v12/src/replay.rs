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

/// Rejoue `bougies` (ordre chronologique) dans un `MoteurV12` neuf et
/// compare l'état final à un moteur nu de référence.
pub fn rejouer_bougies(
    asset: Asset,
    tf: Timeframe,
    bougies: &[Candle],
    simuler_ticks: bool,
    amorce: smc::v12::AmorceMtf,
) -> ResultatReplay {
    let debut = std::time::Instant::now();
    let mut plugin = MoteurV12::nouveau(asset.clone(), tf).avec_amorce(amorce.clone());
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
    // amorçage MTF que le plugin (comparaison conforme_reference valide).
    let mut reference = SmcV12Engine::new(asset.as_str(), tf.as_str());
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
