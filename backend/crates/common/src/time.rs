//! Helper unifié de gestion des fuseaux horaires.
//!
//! Convention applicative : **stockage UTC, affichage Paris** (Europe/Paris,
//! CET/CEST gérés automatiquement par `chrono-tz` via la base IANA).
//!
//! Tous les calculs de jour (PDH/PDL, daily H/L, scheduler) doivent passer par
//! ces fonctions afin de garantir des frontières de jour cohérentes en heure
//! de Paris, indépendamment de l'heure d'été/hiver.

use chrono::{DateTime, Datelike, Offset, Timelike, Utc};
use chrono_tz::Europe::Paris;

/// `DateTime` ancré sur Europe/Paris (CET/CEST).
pub type DateTimeParis = DateTime<chrono_tz::Tz>;

/// Convertit un timestamp Unix UTC en `DateTime` Paris.
pub fn paris_from_unix(ts: i64) -> DateTimeParis {
    DateTime::<Utc>::from_timestamp(ts, 0)
        .unwrap_or_default()
        .with_timezone(&Paris)
}

/// Heure de Paris (0-23) depuis un timestamp Unix UTC.
pub fn heure_paris(ts: i64) -> u32 {
    paris_from_unix(ts).hour()
}

/// Jour de la semaine Paris (0=Lundi ... 6=Dimanche, convention app).
pub fn jour_semaine_paris(ts: i64) -> u32 {
    paris_from_unix(ts).weekday().num_days_from_monday()
}

/// Clé de jour Paris pour bucketing (PDH/PDL, daily H/L).
/// Deux timestamps ont la même clé s'ils tombent dans le même jour calendaire
/// Paris (00:00 → 23:59 Europe/Paris), DST comprise.
pub fn day_key_paris(ts: i64) -> i64 {
    paris_from_unix(ts).num_days_from_ce() as i64
}

/// Offset Paris en secondes (3600 hiver / CET, 7200 été / CEST) pour calculs
/// arithmétiques (ex. SQL, comparaisons UTC).
pub fn offset_paris_seconds(ts: i64) -> i64 {
    paris_from_unix(ts).offset().fix().local_minus_utc() as i64
}

/// Formate un timestamp Unix en "HH:MM Paris".
pub fn format_heure_paris(ts: i64) -> String {
    let p = paris_from_unix(ts);
    format!("{:02}:{:02} Paris", p.hour(), p.minute())
}

#[cfg(test)]
mod tests {
    use super::*;

    // 2024-07-15 (été) : UTC+2 (CEST). 2024-01-15 (hiver) : UTC+1 (CET).
    // Timestamps vérifiés via Europe/Paris.

    #[test]
    fn heure_paris_ete() {
        // 2024-07-15 10:00:00 UTC → 12:00:00 CEST (UTC+2)
        let ts = 1_721_037_600; // 2024-07-15T10:00:00Z
        assert_eq!(heure_paris(ts), 12);
        assert_eq!(offset_paris_seconds(ts), 7200);
    }

    #[test]
    fn heure_paris_hiver() {
        // 2024-01-15 10:00:00 UTC → 11:00:00 CET (UTC+1)
        let ts = 1_705_312_800; // 2024-01-15T10:00:00Z
        assert_eq!(heure_paris(ts), 11);
        assert_eq!(offset_paris_seconds(ts), 3600);
    }

    #[test]
    fn jour_semaine_paris_est_lundi_base() {
        // 2024-07-15 est un lundi Paris → 0 (convention 0=Lundi)
        let ts = 1_721_037_600;
        assert_eq!(jour_semaine_paris(ts), 0);
        // 2024-07-21 00:00:00 UTC = dimanche 02:00 CEST → 6
        let dimanche = 1_721_520_000;
        assert_eq!(jour_semaine_paris(dimanche), 6);
    }

    #[test]
    fn day_key_paris_change_a_minuit_paris() {
        // La clé de jour change à 00:00 Paris (et non à 00:00 UTC).
        // 2024-07-15 21:59:00 UTC = lundi 23:59:00 CEST → lundi.
        let avant = 1_721_080_740; // 2024-07-15T21:59:00Z (lundi 23:59 CEST)
        let apres = 1_721_080_800; // 2024-07-15T22:00:00Z = mardi 00:00:00 CEST
        assert_ne!(day_key_paris(avant), day_key_paris(apres));
    }

    #[test]
    fn format_heure_paris_format() {
        let ts = 1_721_037_600; // 2024-07-15 10:00 UTC → 12:00 CEST
        assert_eq!(format_heure_paris(ts), "12:00 Paris");
    }
}
