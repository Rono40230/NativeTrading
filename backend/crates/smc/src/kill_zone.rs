use chrono::{DateTime, Datelike, Timelike, Utc, Weekday};

/// Fenêtres de trading institutionnel actives (Kill Zones ICT/SMC).
///
/// London  : 07:00–10:00 UTC
/// New York : 13:30–16:30 UTC
/// Macros ICT (fenêtres 20 min à haute activité algorithmique) :
///   02:33–03:00, 04:03–04:30, 08:50–09:10, 09:50–10:10,
///   10:50–11:10, 13:10–13:40, 15:15–15:45 UTC
///
/// Retourne `false` le weekend (marché fermé samedi et dimanche UTC).
pub fn est_en_kill_zone(ts: DateTime<Utc>) -> bool {
    match ts.weekday() {
        Weekday::Sat | Weekday::Sun => return false,
        _ => {}
    }

    let hm = ts.hour() * 60 + ts.minute();

    let london = (7 * 60..10 * 60).contains(&hm);
    let new_york = (13 * 60 + 30..16 * 60 + 30).contains(&hm);
    let macro_london1 = (2 * 60 + 33..3 * 60).contains(&hm);
    let macro_london2 = (4 * 60 + 3..4 * 60 + 30).contains(&hm);
    let macro_ny1 = (8 * 60 + 50..9 * 60 + 10).contains(&hm);
    let macro_ny2 = (9 * 60 + 50..10 * 60 + 10).contains(&hm);
    let macro_silver = (10 * 60 + 50..11 * 60 + 10).contains(&hm);
    let macro_ny_pm = (13 * 60 + 10..13 * 60 + 40).contains(&hm);
    let macro_close = (15 * 60 + 15..15 * 60 + 45).contains(&hm);

    london
        || new_york
        || macro_london1
        || macro_london2
        || macro_ny1
        || macro_ny2
        || macro_silver
        || macro_ny_pm
        || macro_close
}

/// Retourne le libellé de la Kill Zone active, ou `None` si hors fenêtre.
pub fn nom_kill_zone(ts: DateTime<Utc>) -> Option<&'static str> {
    match ts.weekday() {
        Weekday::Sat | Weekday::Sun => return None,
        _ => {}
    }

    let hm = ts.hour() * 60 + ts.minute();

    if (7 * 60..10 * 60).contains(&hm) {
        Some("London")
    } else if (13 * 60 + 30..16 * 60 + 30).contains(&hm) {
        Some("New York")
    } else if (2 * 60 + 33..3 * 60).contains(&hm) || (4 * 60 + 3..4 * 60 + 30).contains(&hm) {
        Some("Macro London")
    } else if (8 * 60 + 50..11 * 60 + 10).contains(&hm) {
        Some("Macro NY AM")
    } else if (13 * 60 + 10..13 * 60 + 40).contains(&hm)
        || (15 * 60 + 15..15 * 60 + 45).contains(&hm)
    {
        Some("Macro NY PM")
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn london_open_est_actif() {
        let ts = Utc.with_ymd_and_hms(2026, 3, 20, 8, 30, 0).unwrap();
        assert!(est_en_kill_zone(ts));
        assert_eq!(nom_kill_zone(ts), Some("London"));
    }

    #[test]
    fn new_york_open_est_actif() {
        let ts = Utc.with_ymd_and_hms(2026, 3, 20, 14, 0, 0).unwrap();
        assert!(est_en_kill_zone(ts));
        assert_eq!(nom_kill_zone(ts), Some("New York"));
    }

    #[test]
    fn hors_session_inactif() {
        let ts = Utc.with_ymd_and_hms(2026, 3, 20, 20, 0, 0).unwrap();
        assert!(!est_en_kill_zone(ts));
        assert_eq!(nom_kill_zone(ts), None);
    }

    #[test]
    fn weekend_inactif() {
        let ts = Utc.with_ymd_and_hms(2026, 3, 21, 9, 0, 0).unwrap(); // Samedi
        assert!(!est_en_kill_zone(ts));
    }
}
