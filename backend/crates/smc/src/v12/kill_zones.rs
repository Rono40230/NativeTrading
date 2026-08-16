//! Kill Zones — plages horaires UTC (Pine lignes 124-161).
//!
//! Reproduit la logique Pine :
//!   `_gKzMins = int(time % 86400000) / 60000`  // Pine `time` est en millisecondes
//!   `inKZ = (_gKzMins >= KZ_ASIAN_START and _gKzMins < KZ_ASIAN_END)
//!        or (_gKzMins >= KZ_LONDON_START and _gKzMins < KZ_LONDON_END)
//!        or (_gKzMins >= KZ_NYAM_START and _gKzMins < KZ_NYAM_END)
//!        or (_gKzMins >= KZ_NYPM_START and _gKzMins < KZ_NYPM_END)`
//!
//! En Rust, le `timestamp` est en **secondes** : `_gKzMins = (timestamp % 86400) / 60`.
//!
//! Constantes UTC (Pine lignes 125-132) :
//!   Asian 0-180, London 420-570, NY AM 720-840, NY PM 1020-1140.

use super::types::{BarInput, KillZone, KillZoneEvent};

/// `KZ_ASIAN_START` (Pine ligne 125).
pub const KZ_ASIAN_START: i64 = 0;
pub const KZ_ASIAN_END: i64 = 180;
pub const KZ_LONDON_START: i64 = 420;
pub const KZ_LONDON_END: i64 = 570;
pub const KZ_NYAM_START: i64 = 720;
pub const KZ_NYAM_END: i64 = 840;
pub const KZ_NYPM_START: i64 = 1020;
pub const KZ_NYPM_END: i64 = 1140;

/// Détecteur Kill Zones (stateless par bar — basé sur le timestamp UTC).
#[derive(Clone)]
pub struct KillZoneDetector {
    last_event: KillZoneEvent,
}

impl KillZoneDetector {
    pub fn new() -> Self {
        Self {
            last_event: KillZoneEvent::default(),
        }
    }

    /// Calcule l'appartenance aux Kill Zones pour la bar courante.
    pub fn update(&mut self, bar: &BarInput) -> KillZoneEvent {
        let mins = bar.timestamp.rem_euclid(86_400) / 60;
        let zone = if (KZ_ASIAN_START..KZ_ASIAN_END).contains(&mins) {
            KillZone::Asian
        } else if (KZ_LONDON_START..KZ_LONDON_END).contains(&mins) {
            KillZone::London
        } else if (KZ_NYAM_START..KZ_NYAM_END).contains(&mins) {
            KillZone::NyAm
        } else if (KZ_NYPM_START..KZ_NYPM_END).contains(&mins) {
            KillZone::NyPm
        } else {
            KillZone::None
        };
        let ev = KillZoneEvent {
            in_kz: !matches!(zone, KillZone::None),
            zone,
            mins,
        };
        self.last_event = ev.clone();
        ev
    }

    pub fn last_event(&self) -> KillZoneEvent {
        self.last_event.clone()
    }
}

impl Default for KillZoneDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bar(ts: i64) -> BarInput {
        BarInput {
            timestamp: ts,
            open: 100.0,
            high: 101.0,
            low: 99.0,
            close: 100.0,
            volume: 0.0,
        }
    }

    #[test]
    fn asian_de_minuit_a_180min() {
        let mut det = KillZoneDetector::new();
        // 0 min UTC.
        assert_eq!(det.update(&bar(0)).zone, KillZone::Asian);
        // 179 min = 2h59 UTC (encore Asian, < 180).
        assert_eq!(det.update(&bar(179 * 60)).zone, KillZone::Asian);
        assert!(det.last_event().in_kz);
    }

    #[test]
    fn hors_kz_entre_asian_et_london() {
        let mut det = KillZoneDetector::new();
        // 181-419 min : hors zone.
        assert_eq!(det.update(&bar(300 * 60)).zone, KillZone::None);
        assert!(!det.last_event().in_kz);
    }

    #[test]
    fn london_420_570() {
        let mut det = KillZoneDetector::new();
        // 420 min = 7h00 UTC.
        assert_eq!(det.update(&bar(420 * 60)).zone, KillZone::London);
        // 569 min encore London.
        assert_eq!(det.update(&bar(569 * 60)).zone, KillZone::London);
        // 570 min = borne exclue.
        assert_eq!(det.update(&bar(570 * 60)).zone, KillZone::None);
    }

    #[test]
    fn ny_am_720_840() {
        let mut det = KillZoneDetector::new();
        assert_eq!(det.update(&bar(720 * 60)).zone, KillZone::NyAm);
        assert_eq!(det.update(&bar(839 * 60)).zone, KillZone::NyAm);
        assert_eq!(det.update(&bar(840 * 60)).zone, KillZone::None);
    }

    #[test]
    fn ny_pm_1020_1140() {
        let mut det = KillZoneDetector::new();
        assert_eq!(det.update(&bar(1020 * 60)).zone, KillZone::NyPm);
        assert_eq!(det.update(&bar(1139 * 60)).zone, KillZone::NyPm);
        assert_eq!(det.update(&bar(1140 * 60)).zone, KillZone::None);
    }

    #[test]
    fn modulo_jour_plusieurs_jours() {
        let mut det = KillZoneDetector::new();
        // 7h00 UTC le 2ᵉ jour (86400 + 420*60).
        let ts = 86_400 + 420 * 60;
        assert_eq!(det.update(&bar(ts)).zone, KillZone::London);
        assert_eq!(det.last_event().mins, 420);
    }

    #[test]
    fn timestamp_negatif_robuste() {
        let mut det = KillZoneDetector::new();
        // rem_euclid gère les timestamps négatifs sans panic.
        let ev = det.update(&bar(-3600));
        assert!(ev.mins >= 0 && ev.mins < 1440);
    }
}
