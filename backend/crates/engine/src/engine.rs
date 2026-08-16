//! Le trait [`Engine`] — contrat unique de toutes les stratégies du runtime.
//!
//! « Une stratégie = un crate » (ROADMAP, décision 5) : chaque moteur
//! implémente ce trait et est branché sur le runtime, une instance par
//! couple (asset × timeframe) — exactement comme un indicateur Pine tourne
//! par graphique. Les moteurs ne se connaissent pas entre eux.

use common::{Asset, Candle, Timeframe};

use crate::agregateur::BougieEnFormation;
use crate::types::SortieMoteur;

/// Contexte d'un événement prix — évaluation intrabar.
pub struct ContexteTick<'a> {
    pub asset: &'a Asset,
    pub tf: Timeframe,
    /// Bougie en formation, mise à jour de l'événement courant.
    /// `bougie.close` = dernier prix reçu.
    pub bougie: &'a BougieEnFormation,
}

/// Contexte d'une clôture de bougie — confirmations.
pub struct ContexteCloture<'a> {
    pub asset: &'a Asset,
    pub tf: Timeframe,
    /// Bougie confirmée (finalisée par l'agrégateur).
    pub bougie: &'a Candle,
    /// Index de la barre clôturée (`bar_index` Pine), compté par le runtime
    /// depuis le démarrage de l'instance du moteur.
    pub index_barre: usize,
}

/// Contrat d'un moteur de signaux.
///
/// - [`Engine::on_tick`] : appelé à CHAQUE événement prix (tick ou update de
///   la bougie en formation). C'est ici que vivent les détections intrabar
///   (prix dans une OB, sweep, OTE, SL/TP du lifecycle).
/// - [`Engine::on_close`] : appelé uniquement à la clôture d'une bougie
///   (équivalent `barstate.isconfirmed` du Pine). C'est ici que vivent les
///   confirmations (BOS, displacement, création de zones).
///
/// Les sorties ([`SortieMoteur`]) distinguent les **signaux** (nouvelles
/// entrées) des **événements** (fills, SL/TP, clôtures du lifecycle) —
/// tous émis au premier tick valide, jamais rétractés (R5).
///
/// Les implémentations doivent être synchrones et rapides : elles tournent
/// dans la boucle événementielle du runtime, sur le chemin critique.
pub trait Engine: Send {
    /// Nom unique du moteur (identifiant stable, ex : `smc_v12`).
    fn nom(&self) -> &str;

    /// Évaluation intrabar — appelée à chaque événement prix.
    fn on_tick(&mut self, _ctx: &ContexteTick) -> SortieMoteur {
        SortieMoteur::vide()
    }

    /// Évaluation à la clôture d'une bougie — confirmations Pine.
    fn on_close(&mut self, _ctx: &ContexteCloture) -> SortieMoteur {
        SortieMoteur::vide()
    }
}
