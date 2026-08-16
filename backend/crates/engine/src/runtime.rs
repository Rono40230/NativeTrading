//! Le runtime : orchestrateur des états (asset × TF), cœur du chemin
//! critique. Reçoit les événements prix, les agrège, évalue les moteurs,
//! publie signaux et bougies clôturées.
//!
//! ## Cycle de vie
//!
//! 1. **Enregistrement** : un couple (asset × TF) est déclaré avec ses
//!    moteurs (zéro moteur en phase 1 — pas de stratégie avant la phase 2).
//! 2. **Cold start** ([`Runtime::rejouer`]) : l'historique des bougies
//!    clôturées est rejoué dans les moteurs via `on_close` — équivalent
//!    TradingView chargeant l'historique avant le live. Les signaux
//!    historiques éventuels sont retournés à l'appelant mais **jamais
//!    publiés** sur le bus (ce ne sont pas des signaux live).
//! 3. **Live** ([`Runtime::traiter_evenement`]) : chaque événement prix
//!    déclenche l'agrégation, les clôtures éventuelles (`on_close`) puis
//!    l'évaluation intrabar (`on_tick`). Les signaux sont publiés et
//!    retournés.
//!
//! ## Garde anti-recouvrement
//!
//! Tout événement pour une bougie déjà clôturée (replay ou live) est
//! ignoré : un push de confirmation en retard ou un recouvrement
//! replay/live ne peut jamais provoquer de double `on_close`.

use std::collections::HashMap;

use common::{Asset, Candle, Timeframe};

use crate::agregateur::{AgregateurBougie, ModeCloture};
use crate::bus::{BougieCloturee, BusBougies, BusEvenements, BusSignaux};
use crate::engine::{ContexteCloture, ContexteTick, Engine};
use crate::types::{EvenementPrix, SortieMoteur};

/// État d'un couple (asset × TF) : agrégateur + moteurs + compteur de barres.
struct EtatAsset {
    agg: AgregateurBougie,
    moteurs: Vec<Box<dyn Engine>>,
    /// Index de la prochaine barre à clôturer (équivalent `bar_index` Pine,
    /// compté depuis le premier replay).
    prochain_index: usize,
    /// Début (epoch sec) de la dernière bougie clôturée (replay ou live).
    dernier_debut_cloture: Option<i64>,
    /// Événements prix ignorés par la garde anti-recouvrement.
    evenements_ignores: u64,
}

/// Le runtime — UNE instance par process, détient tout l'état en mémoire.
/// Détenu par la boucle événementielle (voir `boucle`) ; les consultations
/// externes passent par les bus ou des instantanés.
pub struct Runtime {
    etats: HashMap<(Asset, Timeframe), EtatAsset>,
    bus: BusSignaux,
    bus_bougies: BusBougies,
    bus_evenements: BusEvenements,
}

impl Runtime {
    pub fn nouveau() -> Self {
        Self {
            etats: HashMap::new(),
            bus: BusSignaux::nouveau(),
            bus_bougies: BusBougies::nouveau(),
            bus_evenements: BusEvenements::nouveau(),
        }
    }

    /// Déclare un couple (asset × TF) avec ses moteurs. Un couple déjà
    /// déclaré est remplacé (les moteurs repartent de zéro).
    pub fn enregistrer(&mut self, asset: Asset, tf: Timeframe, moteurs: Vec<Box<dyn Engine>>) {
        self.etats.insert(
            (asset, tf),
            EtatAsset {
                agg: AgregateurBougie::nouveau(tf),
                moteurs,
                prochain_index: 0,
                dernier_debut_cloture: None,
                evenements_ignores: 0,
            },
        );
    }

    /// Retire un couple (asset × TF) du runtime (config UI modifiée).
    /// L'état (agrégateur, moteurs) est perdu — un ré-enregistrement
    /// repart de zéro avec cold start.
    pub fn retirer(&mut self, asset: Asset, tf: Timeframe) {
        self.etats.remove(&(asset, tf));
    }

    /// Couples (asset × TF) enregistrés.
    pub fn cles(&self) -> Vec<(Asset, Timeframe)> {
        self.etats.keys().cloned().collect()
    }

    /// Bus des signaux (abonnement API WS, notifications, journal).
    pub fn bus(&self) -> &BusSignaux {
        &self.bus
    }

    /// Bus des bougies clôturées (journal d'observation, archivage).
    pub fn bus_bougies(&self) -> &BusBougies {
        &self.bus_bougies
    }

    /// Bus des événements lifecycle (fills, SL/TP, clôtures).
    pub fn bus_evenements(&self) -> &BusEvenements {
        &self.bus_evenements
    }

    /// Statistiques d'un couple (observabilité de la garde anti-recouvrement).
    pub fn stats(&self, asset: Asset, tf: Timeframe) -> Option<(usize, Option<i64>, u64)> {
        self.etats
            .get(&(asset, tf))
            .map(|e| (e.prochain_index, e.dernier_debut_cloture, e.evenements_ignores))
    }

    /// Cold start : rejoue des bougies clôturées dans les moteurs.
    ///
    /// Retourne les sorties historiques éventuelles (signaux + événements,
    /// à disposition de l'appelant — journal de replay) **sans les
    /// publier** sur les bus : un signal de replay n'est pas un signal live.
    pub fn rejouer(&mut self, asset: Asset, tf: Timeframe, bougies: &[Candle]) -> SortieMoteur {
        let mut historique = SortieMoteur::vide();
        let Some(etat) = self.etats.get_mut(&(asset.clone(), tf)) else {
            tracing::warn!(
                asset = %asset.as_str(),
                tf = tf.as_str(),
                "rejouer : couple non enregistré"
            );
            return historique;
        };
        for (i, b) in bougies.iter().enumerate() {
            let ctx = ContexteCloture {
                asset: &asset,
                tf,
                bougie: b,
                index_barre: i,
            };
            for moteur in &mut etat.moteurs {
                historique.etend(moteur.on_close(&ctx));
            }
            etat.dernier_debut_cloture = Some(b.timestamp.timestamp());
        }
        etat.prochain_index = bougies.len();
        historique
    }

    /// Traite un événement prix live : agrégation → clôture éventuelle
    /// (`on_close`) → évaluation intrabar (`on_tick`) → publication.
    ///
    /// Retourne les sorties émises (également publiées sur les bus).
    pub fn traiter_evenement(&mut self, ev: EvenementPrix) -> SortieMoteur {
        let Some(etat) = self.etats.get_mut(&(ev.asset.clone(), ev.tf)) else {
            return SortieMoteur::vide();
        };

        // Garde anti-recouvrement : bougie déjà clôturée (replay ou live)
        // → événement en retard, ignoré définitivement.
        if let Some(d) = etat.dernier_debut_cloture {
            if ev.debut_bougie <= d {
                etat.evenements_ignores += 1;
                tracing::debug!(
                    asset = %ev.asset.as_str(),
                    tf = ev.tf.as_str(),
                    debut = ev.debut_bougie,
                    "événement prix ignoré (bougie déjà clôturée)"
                );
                return SortieMoteur::vide();
            }
        }

        let bougie_cloturee = etat.agg.traiter(ev.debut_bougie, ev.event, ev.recu_le);

        let mut sortie = SortieMoteur::vide();

        if let Some((c, _)) = &bougie_cloturee {
            let ctx = ContexteCloture {
                asset: &ev.asset,
                tf: ev.tf,
                bougie: c,
                index_barre: etat.prochain_index,
            };
            etat.prochain_index += 1;
            etat.dernier_debut_cloture = Some(c.timestamp.timestamp());
            for moteur in &mut etat.moteurs {
                sortie.etend(moteur.on_close(&ctx));
            }
        }

        // Évaluation intrabar sur la formation mise à jour. Après une
        // clôture par confirmation, la formation est vide : pas de tick.
        if etat.agg.en_formation().map(|f| f.debut) == Some(ev.debut_bougie) {
            let f = etat
                .agg
                .en_formation()
                .expect("formation vérifiée à l'instant");
            let ctx = ContexteTick {
                asset: &ev.asset,
                tf: ev.tf,
                bougie: f,
            };
            for moteur in &mut etat.moteurs {
                sortie.etend(moteur.on_tick(&ctx));
            }
        }

        for s in &sortie.signaux {
            self.bus.publier(s.clone());
        }
        for e in &sortie.evenements {
            self.bus_evenements.publier(e.clone());
        }
        if let Some((c, mode)) = bougie_cloturee {
            self.bus_bougies.publier(BougieCloturee {
                asset: ev.asset,
                tf: ev.tf,
                bougie: c,
                mode,
            });
        }
        sortie
    }

    /// Clôture forcée de toutes les formations en cours (arrêt propre du
    /// process). Les moteurs ne sont PAS évalués — une bougie non confirmée
    /// officiellement ne doit produire ni clôture moteur ni signal ; les
    /// bougies flushées sont retournées pour le journal d'observation.
    pub fn clore_tout(&mut self) -> Vec<BougieCloturee> {
        let mut cloturees = Vec::new();
        for ((asset, tf), etat) in &mut self.etats {
            if let Some(b) = etat.agg.clore_courante() {
                cloturees.push(BougieCloturee {
                    asset: asset.clone(),
                    tf: *tf,
                    bougie: b,
                    mode: ModeCloture::Forcee,
                });
            }
        }
        cloturees
    }
}

impl Default for Runtime {
    fn default() -> Self {
        Self::nouveau()
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{EvenementTrade, SignalBrut};
    use chrono::{TimeZone, Utc};
    use common::Direction;
    use std::sync::{Arc, Mutex};

    /// Moteur de test : compte ticks/clôtures, émet un signal par tick et un
    /// événement par clôture si demandé (via compteur partagé observable
    /// depuis le test).
    #[derive(Default)]
    struct Compteurs {
        ticks: u32,
        clotures: u32,
        index_clotures: Vec<usize>,
    }

    struct MoteurCompteur {
        compteurs: Arc<Mutex<Compteurs>>,
        signal_a_chaque_tick: bool,
        evenement_a_chaque_cloture: bool,
    }

    impl Engine for MoteurCompteur {
        fn nom(&self) -> &str {
            "compteur"
        }

        fn on_tick(&mut self, ctx: &ContexteTick) -> SortieMoteur {
            self.compteurs.lock().unwrap().ticks += 1;
            if self.signal_a_chaque_tick {
                SortieMoteur {
                    signaux: vec![SignalBrut::nouveau(
                        "compteur",
                        ctx.asset.clone(),
                        ctx.tf,
                        Direction::Long,
                        ctx.bougie.close,
                        0.0,
                        vec![],
                        1,
                        "tick".into(),
                        ctx.bougie.debut,
                    )],
                    evenements: vec![],
                }
            } else {
                SortieMoteur::vide()
            }
        }

        fn on_close(&mut self, ctx: &ContexteCloture) -> SortieMoteur {
            let mut c = self.compteurs.lock().unwrap();
            c.clotures += 1;
            c.index_clotures.push(ctx.index_barre);
            if self.evenement_a_chaque_cloture {
                SortieMoteur {
                    signaux: vec![],
                    evenements: vec![EvenementTrade {
                        moteur: "compteur".into(),
                        asset: ctx.asset.clone(),
                        tf: ctx.tf,
                        cle_trade: format!("bar-{}", ctx.index_barre),
                        evenement: crate::types::TypeEvenementTrade::Fill,
                        detail: "clôture".into(),
                        prix: ctx.bougie.close,
                        debut_barre: ctx.bougie.timestamp.timestamp(),
                        emis_le: chrono::Utc::now(),
                    }],
                }
            } else {
                SortieMoteur::vide()
            }
        }
    }

    fn bougie_a(ts_sec: i64, close: f64) -> Candle {
        Candle {
            timestamp: Utc.timestamp_opt(ts_sec, 0).unwrap(),
            open: close,
            high: close,
            low: close,
            close,
            volume: 1.0,
        }
    }

    fn kline(o: f64, h: f64, b: f64, c: f64, confirmee: bool) -> crate::types::PrixEvent {
        crate::types::PrixEvent::Kline {
            ouverture: o,
            haut: h,
            bas: b,
            cloture: c,
            volume: 10.0,
            confirmee,
        }
    }

    fn evenement(asset: Asset, tf: Timeframe, debut: i64, event: crate::types::PrixEvent) -> EvenementPrix {
        EvenementPrix {
            asset,
            tf,
            debut_bougie: debut,
            event,
            recu_le: Utc::now(),
        }
    }

    #[test]
    fn tick_et_cloture_evalue_les_moteurs_dans_l_ordre() {
        let compteurs = Arc::new(Mutex::new(Compteurs::default()));
        let mut runtime = Runtime::nouveau();
        runtime.enregistrer(
            Asset::from("BTC"),
            Timeframe::M5,
            vec![Box::new(MoteurCompteur {
                compteurs: compteurs.clone(),
                signal_a_chaque_tick: false,
                evenement_a_chaque_cloture: false,
            })],
        );

        // Deux updates non confirmées de la bougie @0
        runtime.traiter_evenement(evenement(Asset::from("BTC"), Timeframe::M5, 0, kline(100.0, 101.0, 99.0, 100.5, false)));
        runtime.traiter_evenement(evenement(Asset::from("BTC"), Timeframe::M5, 0, kline(100.0, 102.0, 99.0, 101.5, false)));
        // Confirmation officielle
        runtime.traiter_evenement(evenement(Asset::from("BTC"), Timeframe::M5, 0, kline(100.0, 102.0, 99.0, 101.0, true)));

        let c = compteurs.lock().unwrap();
        assert_eq!(c.ticks, 2); // les updates non confirmées
        assert_eq!(c.clotures, 1); // la confirmation
        assert_eq!(c.index_clotures, vec![0]);
    }

    #[test]
    fn passage_de_periode_declenche_cloture_puis_tick() {
        let compteurs = Arc::new(Mutex::new(Compteurs::default()));
        let mut runtime = Runtime::nouveau();
        runtime.enregistrer(
            Asset::from("XAUUSD"),
            Timeframe::M15,
            vec![Box::new(MoteurCompteur {
                compteurs: compteurs.clone(),
                signal_a_chaque_tick: false,
                evenement_a_chaque_cloture: false,
            })],
        );

        // Bougie @0 en formation, puis premier événement de @900 (M15)
        runtime.traiter_evenement(evenement(Asset::from("XAUUSD"), Timeframe::M15, 0, kline(50.0, 51.0, 49.0, 50.5, false)));
        runtime.traiter_evenement(evenement(Asset::from("XAUUSD"), Timeframe::M15, 900, kline(50.0, 51.0, 49.0, 50.0, false)));

        let c = compteurs.lock().unwrap();
        assert_eq!(c.clotures, 1); // @0 clôturée par passage de période
        assert_eq!(c.ticks, 2); // tick sur @0 + premier tick sur @900
        assert_eq!(c.index_clotures, vec![0]);
    }

    #[test]
    fn rejouer_alimente_les_moteurs_et_fixe_les_index() {
        let compteurs = Arc::new(Mutex::new(Compteurs::default()));
        let mut runtime = Runtime::nouveau();
        runtime.enregistrer(
            Asset::from("BTC"),
            Timeframe::M5,
            vec![Box::new(MoteurCompteur {
                compteurs: compteurs.clone(),
                signal_a_chaque_tick: false,
                evenement_a_chaque_cloture: false,
            })],
        );

        let historique = vec![bougie_a(0, 100.0), bougie_a(300, 101.0), bougie_a(600, 102.0)];
        runtime.rejouer(Asset::from("BTC"), Timeframe::M5, &historique);

        {
            let c = compteurs.lock().unwrap();
            assert_eq!(c.clotures, 3);
            assert_eq!(c.index_clotures, vec![0, 1, 2]);
        }

        // La prochaine clôture live poursuit la numérotation Pine
        runtime.traiter_evenement(evenement(Asset::from("BTC"), Timeframe::M5, 900, kline(102.0, 103.0, 101.0, 102.5, true)));
        let c = compteurs.lock().unwrap();
        assert_eq!(c.index_clotures, vec![0, 1, 2, 3]);
    }

    #[test]
    fn evenement_sur_bougie_deja_rejouee_est_ignore() {
        let compteurs = Arc::new(Mutex::new(Compteurs::default()));
        let mut runtime = Runtime::nouveau();
        runtime.enregistrer(
            Asset::from("BTC"),
            Timeframe::M5,
            vec![Box::new(MoteurCompteur {
                compteurs: compteurs.clone(),
                signal_a_chaque_tick: false,
                evenement_a_chaque_cloture: false,
            })],
        );

        runtime.rejouer(Asset::from("BTC"), Timeframe::M5, &[bougie_a(0, 100.0), bougie_a(300, 101.0)]);

        // Push en retard pour la bougie @300 déjà clôturée (replay)
        runtime.traiter_evenement(evenement(Asset::from("BTC"), Timeframe::M5, 300, kline(101.0, 101.5, 100.5, 101.0, true)));
        let c = compteurs.lock().unwrap();
        assert_eq!(c.clotures, 2); // inchangé — pas de double clôture
        assert_eq!(c.ticks, 0);

        // Statistiques : 1 événement ignoré par la garde
        let (_, _, ignores) = runtime.stats(Asset::from("BTC"), Timeframe::M5).unwrap();
        assert_eq!(ignores, 1);
    }

    #[test]
    fn signaux_live_publies_sur_le_bus() {
        let compteurs = Arc::new(Mutex::new(Compteurs::default()));
        let mut runtime = Runtime::nouveau();
        runtime.enregistrer(
            Asset::from("BTC"),
            Timeframe::M5,
            vec![Box::new(MoteurCompteur {
                compteurs: compteurs,
                signal_a_chaque_tick: true,
                evenement_a_chaque_cloture: false,
            })],
        );
        let mut rx = runtime.bus().abonner();

        let sortie = runtime.traiter_evenement(evenement(Asset::from("BTC"), Timeframe::M5, 0, kline(100.0, 101.0, 99.0, 100.5, false)));
        assert_eq!(sortie.signaux.len(), 1);
        let recu = rx.try_recv().unwrap();
        assert_eq!(recu.moteur, "compteur");
        assert_eq!(recu.debut_barre, 0);
    }

    #[test]
    fn rejouer_ne_publie_rien_sur_les_bus() {
        let compteurs = Arc::new(Mutex::new(Compteurs::default()));
        let mut runtime = Runtime::nouveau();
        runtime.enregistrer(
            Asset::from("BTC"),
            Timeframe::M5,
            vec![Box::new(MoteurCompteur {
                compteurs,
                signal_a_chaque_tick: true, // émettrait à chaque tick… mais
                evenement_a_chaque_cloture: false,
            })],
        );
        // …le replay n'appelle QUE on_close, qui n'émet rien ici.
        // Pour tester la non-publication des signaux de replay, on utilise
        // un moteur qui émet à la clôture.
        drop(runtime);

        struct MoteurClotureEmettrice;
        impl Engine for MoteurClotureEmettrice {
            fn nom(&self) -> &str {
                "cloture_emettrice"
            }
            fn on_close(&mut self, ctx: &ContexteCloture) -> SortieMoteur {
                SortieMoteur {
                    signaux: vec![SignalBrut::nouveau(
                        "cloture_emettrice",
                        ctx.asset.clone(),
                        ctx.tf,
                        Direction::Long,
                        ctx.bougie.close,
                        0.0,
                        vec![],
                        1,
                        "replay".into(),
                        ctx.bougie.timestamp.timestamp(),
                    )],
                    evenements: vec![],
                }
            }
        }

        let mut runtime2 = Runtime::nouveau();
        runtime2.enregistrer(Asset::from("BTC"), Timeframe::M5, vec![Box::new(MoteurClotureEmettrice)]);
        let mut rx = runtime2.bus().abonner();

        let historique = runtime2.rejouer(Asset::from("BTC"), Timeframe::M5, &[bougie_a(0, 100.0)]);
        assert_eq!(historique.signaux.len(), 1); // signal historique retourné…
        assert!(rx.try_recv().is_err()); // …mais PAS publié sur le bus
    }

    #[test]
    fn bougies_cloturees_publiees_sur_le_bus_bougies() {
        let mut runtime = Runtime::nouveau();
        runtime.enregistrer(Asset::from("BTC"), Timeframe::M5, vec![]);
        let mut rx = runtime.bus_bougies().abonner();

        runtime.traiter_evenement(evenement(Asset::from("BTC"), Timeframe::M5, 0, kline(100.0, 102.0, 99.0, 101.0, true)));
        let recu = rx.try_recv().unwrap();
        assert_eq!(recu.asset, Asset::from("BTC"));
        assert_eq!(recu.tf, Timeframe::M5);
        assert_eq!(recu.bougie.timestamp.timestamp(), 0);
        assert_eq!(recu.bougie.close, 101.0);
    }

    #[test]
    fn clore_tout_ne_declenche_pas_les_moteurs() {
        let compteurs = Arc::new(Mutex::new(Compteurs::default()));
        let mut runtime = Runtime::nouveau();
        runtime.enregistrer(
            Asset::from("BTC"),
            Timeframe::M5,
            vec![Box::new(MoteurCompteur {
                compteurs: compteurs.clone(),
                signal_a_chaque_tick: false,
                evenement_a_chaque_cloture: false,
            })],
        );

        runtime.traiter_evenement(evenement(Asset::from("BTC"), Timeframe::M5, 0, kline(100.0, 101.0, 99.0, 100.5, false)));
        let cloturees = runtime.clore_tout();
        assert_eq!(cloturees.len(), 1); // bougie flushée pour le journal
        assert_eq!(cloturees[0].bougie.close, 100.5);

        let c = compteurs.lock().unwrap();
        assert_eq!(c.clotures, 0); // PAS d'on_close : bougie non confirmée
    }

    #[test]
    fn evenement_non_enregistre_ignore_sans_panique() {
        let mut runtime = Runtime::nouveau();
        let sortie = runtime.traiter_evenement(evenement(Asset::from("ETH"), Timeframe::M1, 0, kline(1.0, 1.0, 1.0, 1.0, false)));
        assert!(sortie.signaux.is_empty());
        assert!(sortie.evenements.is_empty());
        assert!(runtime.cles().is_empty());
    }
}
