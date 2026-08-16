//! Bus interne : les moteurs produisent, le runtime publie,
//! les consommateurs (API WS, notifications, journal d'observation)
//! s'abonnent. Trois flux : signaux, bougies clôturées, événements lifecycle.

use common::{Asset, Candle, Timeframe};
use tokio::sync::broadcast;

use crate::agregateur::ModeCloture;
use crate::types::{EvenementTrade, SignalBrut};

/// Capacité du canal signaux (les lecteurs lents sont dépassés
/// (`Lagged`) et doivent se resynchroniser — un signal n'est JAMAIS
/// rétracté ni rejoué pour rattraper un lecteur lent).
const CAPACITE_SIGNAUX: usize = 256;

/// Capacité du canal bougies (flux plus fréquent que les signaux).
const CAPACITE_BOUVIES: usize = 1024;

/// Capacité du canal événements lifecycle.
const CAPACITE_EVENEMENTS: usize = 1024;

/// Bougie clôturée par le runtime, avec son contexte (asset × TF) et son
/// mode de clôture. Consommée par le journal d'observation (Gate 1).
#[derive(Debug, Clone)]
pub struct BougieCloturee {
    pub asset: Asset,
    pub tf: Timeframe,
    pub bougie: Candle,
    pub mode: ModeCloture,
}

/// Poignée clonable de publication/abonnement aux signaux du runtime.
#[derive(Clone)]
pub struct BusSignaux {
    tx: broadcast::Sender<SignalBrut>,
}

impl BusSignaux {
    pub fn nouveau() -> Self {
        let (tx, _) = broadcast::channel(CAPACITE_SIGNAUX);
        Self { tx }
    }

    /// Publie un signal à tous les abonnés. Un signal publié n'est jamais
    /// retiré (règle R5) ; l'absence d'abonné n'est pas une erreur.
    pub fn publier(&self, signal: SignalBrut) {
        let _ = self.tx.send(signal);
    }

    /// S'abonne au flux de signaux.
    pub fn abonner(&self) -> broadcast::Receiver<SignalBrut> {
        self.tx.subscribe()
    }

    /// Nombre d'abonnés actuels.
    pub fn nb_abonnes(&self) -> usize {
        self.tx.receiver_count()
    }
}

impl Default for BusSignaux {
    fn default() -> Self {
        Self::nouveau()
    }
}

/// Poignée clonable de publication/abonnement aux bougies clôturées.
#[derive(Clone)]
pub struct BusBougies {
    tx: broadcast::Sender<BougieCloturee>,
}

impl BusBougies {
    pub fn nouveau() -> Self {
        let (tx, _) = broadcast::channel(CAPACITE_BOUVIES);
        Self { tx }
    }

    pub fn publier(&self, bougie: BougieCloturee) {
        let _ = self.tx.send(bougie);
    }

    pub fn abonner(&self) -> broadcast::Receiver<BougieCloturee> {
        self.tx.subscribe()
    }

    pub fn nb_abonnes(&self) -> usize {
        self.tx.receiver_count()
    }
}

impl Default for BusBougies {
    fn default() -> Self {
        Self::nouveau()
    }
}

/// Poignée clonable de publication/abonnement aux événements lifecycle.
#[derive(Clone)]
pub struct BusEvenements {
    tx: broadcast::Sender<EvenementTrade>,
}

impl BusEvenements {
    pub fn nouveau() -> Self {
        let (tx, _) = broadcast::channel(CAPACITE_EVENEMENTS);
        Self { tx }
    }

    pub fn publier(&self, evenement: EvenementTrade) {
        let _ = self.tx.send(evenement);
    }

    pub fn abonner(&self) -> broadcast::Receiver<EvenementTrade> {
        self.tx.subscribe()
    }

    pub fn nb_abonnes(&self) -> usize {
        self.tx.receiver_count()
    }
}

impl Default for BusEvenements {
    fn default() -> Self {
        Self::nouveau()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use common::{Asset, Direction, Timeframe};

    fn signal_exemple() -> SignalBrut {
        SignalBrut::nouveau(
            "test",
            Asset::from("BTC"),
            Timeframe::M15,
            Direction::Long,
            100.0,
            95.0,
            vec![110.0],
            80,
            "test".into(),
            0,
        )
    }

    fn bougie_exemple() -> BougieCloturee {
        BougieCloturee {
            asset: Asset::from("BTC"),
            tf: Timeframe::M15,
            bougie: Candle {
                timestamp: Utc::now(),
                open: 100.0,
                high: 101.0,
                low: 99.0,
                close: 100.5,
                volume: 10.0,
            },
            mode: crate::agregateur::ModeCloture::Confirmation,
        }
    }

    #[tokio::test]
    async fn publie_et_transmet_aux_abonnes() {
        let bus = BusSignaux::nouveau();
        let mut rx = bus.abonner();
        bus.publier(signal_exemple());
        let recu = rx.recv().await.unwrap();
        assert_eq!(recu.moteur, "test");
        assert_eq!(recu.asset, Asset::from("BTC"));
    }

    #[tokio::test]
    async fn publier_sans_abonne_ne_panique_pas() {
        let bus = BusSignaux::nouveau();
        bus.publier(signal_exemple()); // ne doit pas paniquer
        assert_eq!(bus.nb_abonnes(), 0);
    }

    #[tokio::test]
    async fn bougies_transmises_aux_abonnes() {
        let bus = BusBougies::nouveau();
        let mut rx = bus.abonner();
        bus.publier(bougie_exemple());
        let recu = rx.recv().await.unwrap();
        assert_eq!(recu.asset, Asset::from("BTC"));
        assert_eq!(recu.tf, Timeframe::M15);
        assert_eq!(recu.bougie.close, 100.5);
    }

    #[tokio::test]
    async fn lecteur_lent_est_depassé_mais_le_bus_tient() {
        let bus = BusSignaux::nouveau();
        let mut rx = bus.abonner();
        for _ in 0..CAPACITE_SIGNAUX + 10 {
            bus.publier(signal_exemple());
        }
        // Le lecteur a dépassé la capacité : il reçoit Lagged, pas d'erreur.
        match rx.try_recv() {
            Err(broadcast::error::TryRecvError::Lagged(n)) => assert!(n > 0),
            autre => panic!("attendu Lagged, reçu {:?}", autre.is_ok()),
        }
        // Le bus continue de fonctionner pour les nouveaux messages.
        let mut rx2 = bus.abonner();
        bus.publier(signal_exemple());
        assert!(rx2.try_recv().is_ok());
    }
}
