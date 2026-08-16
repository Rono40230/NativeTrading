//! Agrégation prix → bougies : formation de l'OHLCV en continu et clôture
//! au passage de période. Brique pure (aucune I/O, testable sans réseau).
//!
//! ## Stratégie de clôture
//!
//! Une bougie est clôturée dans trois cas :
//! 1. **Confirmation officielle** : poussée `Kline { confirmee: true }` de
//!    l'échange pour la bougie en formation → clôture immédiate avec les
//!    valeurs autoritaires (garantit l'identité avec la bougie officielle).
//! 2. **Passage de période** : événement pour une bougie postérieure →
//!    clôture de l'accumulation en cours, ouverture de la nouvelle.
//! 3. **Clôture forcée** ([`AgregateurBougie::clore_courante`]) : arrêt
//!    propre du runtime.
//!
//! Les événements en retard (bougie antérieure à la formation en cours)
//! sont ignorés — la protection anti-ordre-inversé.

use chrono::{DateTime, TimeZone, Utc};
use common::{Candle, Timeframe};

use crate::types::PrixEvent;

/// Comment une bougie a été clôturée par l'agrégateur — axe diagnostique de
/// la Gate 1 (une divergence sur un mode précis oriente la correction).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModeCloture {
    /// Poussée officielle de confirmation de l'échange (valeurs
    /// autoritaires — doit correspondre à 100 % à la bougie en DB).
    Confirmation,
    /// Premier événement de la bougie suivante (la confirmation officielle
    /// n'est pas encore arrivée — divergence possible si des updates ont
    /// manqué).
    PassagePeriode,
    /// Clôture forcée à l'arrêt du runtime (bougie incomplète : jamais
    /// comparée aux officielles).
    Forcee,
}

impl ModeCloture {
    /// Libellé persisté dans le journal d'observation.
    pub fn libelle(&self) -> &'static str {
        match self {
            ModeCloture::Confirmation => "confirmation",
            ModeCloture::PassagePeriode => "passage",
            ModeCloture::Forcee => "forcee",
        }
    }
}

/// Bougie en formation — l'état intrabar du runtime.
#[derive(Debug, Clone, PartialEq)]
pub struct BougieEnFormation {
    /// Début de la période (epoch secondes, aligné sur le TF).
    pub debut: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    /// Nombre d'événements prix intégrés depuis l'ouverture.
    pub nb_events: u64,
    /// Instant de réception du dernier événement.
    pub dernier_event: Option<DateTime<Utc>>,
}

impl BougieEnFormation {
    /// Dernier prix connu (alias de `close`).
    pub fn prix(&self) -> f64 {
        self.close
    }
}

/// Agrégateur d'UN couple (asset × timeframe). Pur, sans I/O — le runtime
/// en détient un par (asset, TF) surveillé.
///
/// Note d'alignement : les bornes sont calculées modulo l'epoch — exact
/// pour M1→H4 et D1 (minuit UTC). W1 via ce chemin est approximatif
/// (l'epoch démarre un jeudi) : les bougies W1 proviennent du replay DB.
pub struct AgregateurBougie {
    tf: Timeframe,
    formation: Option<BougieEnFormation>,
}

impl AgregateurBougie {
    pub fn nouveau(tf: Timeframe) -> Self {
        Self {
            tf,
            formation: None,
        }
    }

    pub fn tf(&self) -> Timeframe {
        self.tf
    }

    /// Bougie en formation (si aucun événement reçu, `None`).
    pub fn en_formation(&self) -> Option<&BougieEnFormation> {
        self.formation.as_ref()
    }

    /// Début de bougie aligné sur le TF pour un epoch donné (secondes).
    pub fn aligner(tf: Timeframe, ts_sec: i64) -> i64 {
        let secs = tf.minutes() as i64 * 60;
        ts_sec.div_euclid(secs) * secs
    }

    /// Traite un événement prix pour la bougie débutant à `debut_bougie`.
    ///
    /// Retourne `Some((candle, mode))` si cet événement provoque la clôture
    /// d'une bougie (confirmation officielle ou passage de période), `None`
    /// sinon. Les événements en retard sont ignorés.
    pub fn traiter(
        &mut self,
        debut_bougie: i64,
        event: PrixEvent,
        recu_le: DateTime<Utc>,
    ) -> Option<(Candle, ModeCloture)> {
        let cloture_officielle = matches!(event, PrixEvent::Kline { confirmee: true, .. });

        match self.formation.as_ref().map(|f| f.debut) {
            // Rien en formation : ouvrir (et clôturer d'office si l'échange
            // nous confirme directement une bougie — cas post-reconnexion).
            None => {
                self.ouvrir(debut_bougie, &event, recu_le);
                if cloture_officielle {
                    self.clore().map(|c| (c, ModeCloture::Confirmation))
                } else {
                    None
                }
            }
            // Événement en retard : ignoré.
            Some(d) if debut_bougie < d => None,
            // Passage de période : clôturer l'ancienne, ouvrir la nouvelle.
            Some(d) if debut_bougie > d => {
                let cloture = self.clore().map(|c| (c, ModeCloture::PassagePeriode));
                self.ouvrir(debut_bougie, &event, recu_le);
                cloture
            }
            // Bougie courante : mise à jour, ou clôture si confirmation.
            Some(_) => {
                if let PrixEvent::Kline { confirmee: true, .. } = event {
                    self.remplacer_et_clore(event)
                        .map(|c| (c, ModeCloture::Confirmation))
                } else {
                    self.mettre_a_jour(event, recu_le);
                    None
                }
            }
        }
    }

    /// Force la clôture de la bougie en cours (arrêt propre du runtime).
    pub fn clore_courante(&mut self) -> Option<Candle> {
        self.clore()
    }

    // ── Internes ─────────────────────────────────────────────────────────────

    fn ouvrir(&mut self, debut: i64, event: &PrixEvent, recu_le: DateTime<Utc>) {
        let f = match *event {
            PrixEvent::Tick { prix, volume } => BougieEnFormation {
                debut,
                open: prix,
                high: prix,
                low: prix,
                close: prix,
                volume: volume.unwrap_or(0.0),
                nb_events: 1,
                dernier_event: Some(recu_le),
            },
            PrixEvent::Kline {
                ouverture,
                haut,
                bas,
                cloture,
                volume,
                ..
            } => BougieEnFormation {
                debut,
                open: ouverture,
                high: haut,
                low: bas,
                close: cloture,
                volume,
                nb_events: 1,
                dernier_event: Some(recu_le),
            },
        };
        self.formation = Some(f);
    }

    fn mettre_a_jour(&mut self, event: PrixEvent, recu_le: DateTime<Utc>) {
        let Some(f) = self.formation.as_mut() else {
            return;
        };
        match event {
            PrixEvent::Tick { prix, volume } => {
                if prix > f.high {
                    f.high = prix;
                }
                if prix < f.low {
                    f.low = prix;
                }
                f.close = prix;
                if let Some(v) = volume {
                    f.volume += v;
                }
            }
            PrixEvent::Kline {
                ouverture,
                haut,
                bas,
                cloture,
                volume,
                ..
            } => {
                // Snapshot cumulatif officiel : open/close/volume autoritaires,
                // high/low fusionnés par sécurité (ordre réseau).
                f.open = ouverture;
                if haut > f.high {
                    f.high = haut;
                }
                if bas < f.low {
                    f.low = bas;
                }
                f.close = cloture;
                f.volume = volume;
            }
        }
        f.nb_events += 1;
        f.dernier_event = Some(recu_le);
    }

    /// Remplace la formation par les valeurs autoritaires de la confirmation
    /// puis clôture. La bougie officielle publiée par l'échange (celle écrite
    /// en DB par le worker) fait foi : remplacer — et non fusionner — garantit
    /// la concordance 100 % exigée par la Gate 1.
    fn remplacer_et_clore(&mut self, event: PrixEvent) -> Option<Candle> {
        let PrixEvent::Kline {
            ouverture,
            haut,
            bas,
            cloture,
            volume,
            ..
        } = event
        else {
            return self.clore();
        };
        let Some(f) = self.formation.as_mut() else {
            return None;
        };
        f.open = ouverture;
        f.high = haut;
        f.low = bas;
        f.close = cloture;
        f.volume = volume;
        f.nb_events += 1;
        self.clore()
    }

    fn clore(&mut self) -> Option<Candle> {
        let f = self.formation.take()?;
        Some(Candle {
            timestamp: epoch_vers_utc(f.debut),
            open: f.open,
            high: f.high,
            low: f.low,
            close: f.close,
            volume: f.volume,
        })
    }
}

fn epoch_vers_utc(ts: i64) -> DateTime<Utc> {
    Utc.timestamp_opt(ts, 0)
        .single()
        .unwrap_or_else(Utc::now)
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn maintenant() -> DateTime<Utc> {
        Utc::now()
    }

    fn tick(prix: f64) -> PrixEvent {
        PrixEvent::Tick {
            prix,
            volume: Some(1.0),
        }
    }

    fn kline(o: f64, h: f64, b: f64, c: f64, v: f64, confirmee: bool) -> PrixEvent {
        PrixEvent::Kline {
            ouverture: o,
            haut: h,
            bas: b,
            cloture: c,
            volume: v,
            confirmee,
        }
    }

    #[test]
    fn ticks_forment_ohlc_exact() {
        let mut agg = AgregateurBougie::nouveau(Timeframe::M5);
        // Bougie débutant à t=300 (M5)
        assert!(agg
            .traiter(300, tick(100.0), maintenant())
            .is_none());
        agg.traiter(300, tick(105.0), maintenant());
        agg.traiter(300, tick(95.0), maintenant());
        agg.traiter(300, tick(102.0), maintenant());

        let f = agg.en_formation().unwrap();
        assert_eq!(f.debut, 300);
        assert_eq!(f.open, 100.0);
        assert_eq!(f.high, 105.0);
        assert_eq!(f.low, 95.0);
        assert_eq!(f.close, 102.0);
        assert_eq!(f.volume, 4.0); // 4 ticks × 1.0
        assert_eq!(f.nb_events, 4);
    }

    #[test]
    fn passage_de_periode_cloture_puis_ouvre() {
        let mut agg = AgregateurBougie::nouveau(Timeframe::M5);
        agg.traiter(300, tick(100.0), maintenant());
        agg.traiter(300, tick(110.0), maintenant());
        // Premier événement de la période suivante (t=600)
        let (bougie, mode) = agg.traiter(600, tick(108.0), maintenant()).unwrap();
        assert_eq!(mode, ModeCloture::PassagePeriode);

        assert_eq!(bougie.timestamp.timestamp(), 300);
        assert_eq!(bougie.open, 100.0);
        assert_eq!(bougie.high, 110.0);
        assert_eq!(bougie.close, 110.0);
        assert!((bougie.volume - 2.0).abs() < 1e-9);

        // La nouvelle période est ouverte avec le prix courant
        let f = agg.en_formation().unwrap();
        assert_eq!(f.debut, 600);
        assert_eq!(f.open, 108.0);
        assert_eq!(f.close, 108.0);
    }

    #[test]
    fn kline_snapshots_accumulent_correctement() {
        let mut agg = AgregateurBougie::nouveau(Timeframe::M15);
        agg.traiter(0, kline(50.0, 51.0, 49.5, 50.5, 120.0, false), maintenant());
        agg.traiter(0, kline(50.0, 52.0, 49.0, 51.5, 300.0, false), maintenant());
        agg.traiter(0, kline(50.0, 51.8, 49.2, 51.0, 450.0, false), maintenant());

        let f = agg.en_formation().unwrap();
        assert_eq!(f.open, 50.0);
        assert_eq!(f.high, 52.0); // max des snapshots
        assert_eq!(f.low, 49.0); // min des snapshots
        assert_eq!(f.close, 51.0);
        assert_eq!(f.volume, 450.0); // cumul officiel : remplace
        assert_eq!(f.nb_events, 3);
    }

    #[test]
    fn confirmation_officielle_cloture_immédiatement() {
        let mut agg = AgregateurBougie::nouveau(Timeframe::M15);
        agg.traiter(0, kline(50.0, 51.0, 49.5, 50.5, 120.0, false), maintenant());
        agg.traiter(0, kline(50.0, 52.0, 49.0, 51.5, 300.0, false), maintenant());

        // Poussée de clôture officielle
        let (bougie, mode) = agg
            .traiter(0, kline(50.0, 52.0, 49.0, 51.0, 480.0, true), maintenant())
            .unwrap();
        assert_eq!(mode, ModeCloture::Confirmation);
        assert_eq!(bougie.timestamp.timestamp(), 0);
        assert_eq!(bougie.high, 52.0);
        assert_eq!(bougie.close, 51.0);
        assert_eq!(bougie.volume, 480.0);

        // Plus rien en formation
        assert!(agg.en_formation().is_none());
    }

    #[test]
    fn premier_message_est_une_confirmation_post_reconnexion() {
        let mut agg = AgregateurBougie::nouveau(Timeframe::M5);
        // Après reconnexion, le premier message peut confirmer une bougie
        // jamais vue en formation.
        let (bougie, mode) = agg
            .traiter(600, kline(99.0, 101.0, 98.0, 100.0, 500.0, true), maintenant())
            .unwrap();
        assert_eq!(mode, ModeCloture::Confirmation);
        assert_eq!(bougie.timestamp.timestamp(), 600);
        assert_eq!(bougie.open, 99.0);
        assert_eq!(bougie.close, 100.0);
        assert!(agg.en_formation().is_none());
    }

    #[test]
    fn evenement_en_retard_est_ignore() {
        let mut agg = AgregateurBougie::nouveau(Timeframe::M5);
        agg.traiter(600, tick(100.0), maintenant());
        agg.traiter(600, tick(102.0), maintenant());

        // Événement pour une bougie antérieure (rejouée en retard)
        assert!(agg.traiter(300, tick(50.0), maintenant()).is_none());
        let f = agg.en_formation().unwrap();
        assert_eq!(f.debut, 600);
        assert_eq!(f.close, 102.0); // inchangé
        assert_eq!(f.nb_events, 2); // l'événement en retard n'a pas compté
    }

    #[test]
    fn clore_courante_force_la_cloture() {
        let mut agg = AgregateurBougie::nouveau(Timeframe::M30);
        agg.traiter(0, tick(10.0), maintenant());
        agg.traiter(0, tick(12.0), maintenant());

        let bougie = agg.clore_courante().unwrap();
        assert_eq!(bougie.open, 10.0);
        assert_eq!(bougie.high, 12.0);
        assert!(agg.en_formation().is_none());
        // Deuxième appel : plus rien à clôturer
        assert!(agg.clore_courante().is_none());
    }

    #[test]
    fn alignement_arrondit_au_debut_de_periode() {
        assert_eq!(AgregateurBougie::aligner(Timeframe::M5, 267), 0);
        assert_eq!(AgregateurBougie::aligner(Timeframe::M5, 300), 300);
        assert_eq!(AgregateurBougie::aligner(Timeframe::M5, 314), 300);
        assert_eq!(AgregateurBougie::aligner(Timeframe::M30, 3615), 3600);
        assert_eq!(AgregateurBougie::aligner(Timeframe::H1, 3599), 0);
        assert_eq!(AgregateurBougie::aligner(Timeframe::D1, 86399 + 86400), 86400);
    }

    #[test]
    fn gaps_multiples_ne_fabriquent_rien() {
        let mut agg = AgregateurBougie::nouveau(Timeframe::M5);
        agg.traiter(0, tick(100.0), maintenant());
        // Saut direct de 3 périodes (coupure WS) : clôture de la 1re,
        // PAS de fabrication des bougies intermédiaires.
        let (bougie, _) = agg.traiter(900, tick(90.0), maintenant()).unwrap();
        assert_eq!(bougie.timestamp.timestamp(), 0);
        let f = agg.en_formation().unwrap();
        assert_eq!(f.debut, 900);
    }

    #[test]
    fn prixFinalEstLeClose() {
        let mut agg = AgregateurBougie::nouveau(Timeframe::M5);
        agg.traiter(0, tick(100.0), maintenant());
        agg.traiter(0, tick(104.0), maintenant());
        assert_eq!(agg.en_formation().unwrap().prix(), 104.0);
    }
}
