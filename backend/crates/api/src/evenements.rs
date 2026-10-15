//! §Straddle événements (28/09) — taxonomie + conversion DST.
//!
//! Question propriétaire : « quels événements PRÉVISIBLES génèrent le plus de
//! liquidité, et quels assets y réagissent le mieux ? » Chaque créneau à
//! l'horloge est défini dans SON fuseau d'origine : la conversion en heure de
//! Paris suit automatiquement les bascules d'heure d'été — y compris les
//! fenêtres de l'année où l'Europe et les États-Unis ne sont plus alignés.
//!
//! DOCTRINE (owner 14/10 puis 15/10) — deux familles, deux sources :
//!   • STRUCTURELS (calendrier = false) : créneaux récurrents (ouvertures,
//!     fixes, clôtures, réouvertures). Depuis le 15/10 ils vivent dans la
//!     TABLE `evenements_catalogue` (migration 0122) — éditables par la
//!     modale « Gérer les créneaux fixes » (CRUD), la matrice suit. Le seed
//!     initial = les 8 structurels du code d'avant + « Réouverture or »
//!     (owner 15/10 : lundi 00h00 Paris, pause Axi 22h58→00h00).
//!   • CALENDRIER (calendrier = true) : les 9 types d'annonces 📅 (NFP, CPI,
//!     FOMC…, filet cal_autres) restent EN CODE — leur heure au quotidien
//!     relève du calendrier économique SEUL (annonces High ; les Medium ne
//!     se tradent pas, décision owner 14/10). Ils ne sont PAS éditables.
//!
//! La matrice mesure, sur 120 jours de M1, l'ATR des 3 premières minutes de
//! chaque occurrence rapporté à l'habitude de l'asset (×N). Elle ne décide
//! RIEN : c'est la matière première de l'armement (propriétaire seul).


use chrono::{DateTime, Datelike, Duration, TimeZone, Utc};
use chrono_tz::Tz;


/// Un créneau de la taxonomie — struct POSSÉDÉE (les structurels viennent de
/// la base, les types 📅 du code). Les jours sont ISO lundi=1..dimanche=7
/// **dans le fuseau de l'événement**.
#[derive(Clone, Debug)]
pub(crate) struct Evenement {
    pub(crate) ident: String,
    pub(crate) nom: String,
    pub(crate) detail: String,
    pub(crate) tz: Tz,
    pub(crate) heure: u32,
    pub(crate) minute: u32,
    pub(crate) jours: Vec<u32>,
    /// true = TYPE CALENDAIRE : pas d'heure fixe — l'heure vient de la VRAIE
    /// annonce du calendrier, classée par titre. tz/heure/jours sont des
    /// placeholders (New York 8:30) pour ces lignes.
    pub(crate) calendrier: bool,
    /// false = en pause (owner 15/10) : le runtime l'ignore, les compteurs
    /// de la matrice sont conservés. Toujours true pour les types 📅.
    pub(crate) actif: bool,
}

/// Les 9 TYPES CALENDAIRE 📅 (code, non éditables — l'heure vient de
/// l'annonce réelle). Structurels : voir table evenements_catalogue.
pub(crate) const CAL_TYPES: &[(&str, &str, &str)] = &[
    ("cal_nfp", "NFP — emploi US (1er vendredi)",
     "Non-Farm Payrolls et taux de chômage : la plus grosse volatilité mensuelle du calendrier."),
    ("cal_cpi", "CPI US — inflation",
     "Indice des prix à la consommation (CPI/cœur) : le chiffre qui décide des attentes de taux."),
    ("cal_fomc", "FOMC — taux + conférence",
     "Décision de taux et conférence de la Fed : 14h00 puis 14h30 New York les jours de réunion."),
    ("cal_pce", "PCE US — inflation Fed",
     "Déflateur PCE (mesure d'inflation préférée de la Fed), 8:30 New York."),
    ("cal_gdp", "PIB US (GDP)",
     "Croissance trimestrielle (advance/second/third estimate), 8:30 New York."),
    ("cal_retail", "Retail Sales US",
     "Ventes au détail (et core retail), 8:30 New York — pouls du consommateur."),
    ("cal_jobless", "Jobless Claims US (jeudi)",
     "Inscriptions hebdo au chômage, 8:30 New York chaque jeudi."),
    ("cal_perso", "Dépenses personnelles US",
     "Personal Spending / Personal Income, 8:30 New York."),
    ("cal_autres", "Autres annonces US High",
     "Filet : toute annonce High non classée dans un type ci-dessus (ISM, confiance, ventes de logements…) — armée par défaut pour ne perdre aucune passe."),
];

/// Les types 📅 en `Evenement` (placeholder New York 8:30 — l'heure réelle
/// vient de l'annonce classée).
pub(crate) fn types_calendrier() -> Vec<Evenement> {
    CAL_TYPES
        .iter()
        .map(|(ident, nom, detail)| Evenement {
            ident: (*ident).to_string(),
            nom: (*nom).to_string(),
            detail: (*detail).to_string(),
            tz: chrono_tz::America::New_York,
            heure: 8,
            minute: 30,
            jours: vec![1, 2, 3, 4, 5],
            calendrier: true,
            actif: true,
        })
        .collect()
}

/// Fenêtre mesurée après le début de l'événement (3 premières minutes M1).
pub(crate) const FENETRE_MINUTES: i64 = 3;
/// Historique mesuré pour la matrice (jours) — couvre plusieurs bascules DST.
pub(crate) const PERIODE_JOURS: i64 = 120;
/// Minimum de minutes d'événement observées pour publier un ratio.
pub(crate) const MIN_MINUTES: i64 = 20;
/// Minimum de bougies M1 pour qu'un asset ait une habitude digne de ce nom.
pub(crate) const MIN_BOUGIES: usize = 20_000;

// ── Conversion DST ───────────────────────────────────────────────────────────

/// Instants (timestamps M1) couverts par un événement entre deux bornes UTC :
/// pour chaque jour du fuseau d'origine qui matche, l'heure locale est
/// convertie en absolu — chrono-tz applique la bonne bascule d'été selon la
/// DATE, pas selon la saison courante.
pub(crate) fn fenetres_evenement(ev: &Evenement, debut: i64, fin: i64) -> Vec<i64> {
    let mut out = Vec::new();
    let Some(debut_utc) = DateTime::from_timestamp(debut, 0) else {
        return out;
    };
    let Some(fin_utc) = DateTime::from_timestamp(fin, 0) else {
        return out;
    };
    // Bornes en dates du fuseau événement : la veille du début (un événement
    // déjà commencé à `debut` compte quand même) jusqu'à dépasser `fin`.
    let premier_jour = debut_utc.with_timezone(&ev.tz).date_naive() - Duration::days(1);
    let dernier_jour = fin_utc.with_timezone(&ev.tz).date_naive() + Duration::days(1);
    let mut jour = premier_jour;
    while jour <= dernier_jour {
        let naive = jour.and_hms_opt(ev.heure, ev.minute, 0);
        // Heure inexistante (bascule de printemps) → single() = None : ignorée.
        if let Some(ts) = naive
            .and_then(|n| ev.tz.from_local_datetime(&n).single())
            .map(|a| a.timestamp())
        {
            if ev.jours.contains(&jour.weekday().number_from_monday())
                && ts + FENETRE_MINUTES * 60 >= debut
                && ts <= fin
            {
                for k in 0..FENETRE_MINUTES {
                    out.push(ts + k * 60);
                }
            }
        }
        jour += Duration::days(1);
    }
    out
}

/// Prochaine occurrence strictement future, en absolu UTC.
pub(crate) fn prochaine_occurrence(
    ev: &Evenement,
    maintenant: DateTime<Utc>,
) -> Option<DateTime<Utc>> {
    let local = maintenant.with_timezone(&ev.tz);
    for delta in 0..=8i64 {
        let jour = (local + Duration::days(delta)).date_naive();
        if !ev.jours.contains(&jour.weekday().number_from_monday()) {
            continue;
        }
        let naive = jour.and_hms_opt(ev.heure, ev.minute, 0)?;
        if let Some(absolu) = ev.tz.from_local_datetime(&naive).single() {
            if absolu > local {
                return Some(absolu.with_timezone(&Utc));
            }
        }
    }
    None
}

/// Classe le TITRE d'une annonce réelle en ident de type calendaire
/// (owner 10/10). None = non classée → filet « cal_autres » si armé.
/// Ordre = spécificité décroissante (PCE avant « Personal »).
pub(crate) fn classifie_annonce(titre: &str) -> Option<&'static str> {
    let t = titre.to_lowercase();
    let dans = |mots: &[&str]| mots.iter().all(|m| t.contains(m));
    if dans(&["non-farm"]) || dans(&["nonfarm"]) || (dans(&["employment"]) && t.contains("change")) {
        return Some("cal_nfp");
    }
    if t.contains("cpi") || (t.contains("consumer") && t.contains("price")) {
        return Some("cal_cpi");
    }
    if t.contains("fomc") || t.contains("federal funds rate") || t.contains("fed chair") {
        return Some("cal_fomc");
    }
    if t.contains("pce") {
        return Some("cal_pce");
    }
    if t.contains("gdp") || t.contains("gross domestic") {
        return Some("cal_gdp");
    }
    if t.contains("retail sales") || (t.contains("core retail") ) {
        return Some("cal_retail");
    }
    if t.contains("jobless") || t.contains("unemployment claims") || t.contains("initial claims") {
        return Some("cal_jobless");
    }
    if t.contains("personal spending") || t.contains("personal income") {
        return Some("cal_perso");
    }
    None
}

// ── Tests : conversion DST et fenêtres (même sémantique qu'avant la
//    migration 0122 — les structurels se construisent ici en mémoire) ────────

#[cfg(test)]
pub(crate) mod tests_outils {
    use super::*;

    /// Constructeur de test : un créneau possédé, calendrier=false.
    pub(crate) fn crene(tz: Tz, heure: u32, minute: u32, jours: &[u32]) -> Evenement {
        Evenement {
            ident: "test".into(),
            nom: "Test".into(),
            detail: String::new(),
            tz,
            heure,
            minute,
            jours: jours.to_vec(),
            calendrier: false,
            actif: true,
        }
    }

    pub(crate) fn paris_hhmm(ev: &Evenement, utc: DateTime<Utc>) -> String {
        use chrono::Timelike;
        let p = prochaine_occurrence(ev, utc - Duration::hours(12))
            .expect("occurrence")
            .with_timezone(&chrono_tz::Europe::Paris);
        format!("{:02}:{:02}", p.hour(), p.minute())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evenements::tests_outils::{crene, paris_hhmm};
    use chrono::Timelike;
    use chrono_tz::America::New_York;

    /// 9:30 New York = 15:30 Paris toute l'année hors entre-bascules :
    /// l'écart New York↔Paris est structurellement de 6 h.
    #[test]
    fn nyse_ouverture_stable_entre_ete_et_hiver() {
        let ev = crene(New_York, 9, 30, &[1, 2, 3, 4, 5]);
        let ete = Utc.with_ymd_and_hms(2026, 7, 15, 0, 0, 0).unwrap();
        let hiver = Utc.with_ymd_and_hms(2026, 1, 15, 0, 0, 0).unwrap();
        assert_eq!(paris_hhmm(&ev, ete), "15:30");
        assert_eq!(paris_hhmm(&ev, hiver), "15:30");
    }

    /// LA fenêtre piège : l'Europe bascule le dernier dimanche d'octobre,
    /// les États-Unis le 1ᵉʳ novembre — entre les deux, l'écart passe à 5 h
    /// et l'ouverture NYSE tombe à 14:30 Paris.
    #[test]
    fn entre_bascules_octobre_nyse_a_1430_paris() {
        let ev = crene(New_York, 9, 30, &[1, 2, 3, 4, 5]);
        let lundi_oct = Utc.with_ymd_and_hms(2026, 10, 26, 0, 0, 0).unwrap();
        assert_eq!(paris_hhmm(&ev, lundi_oct), "14:30");
        let mi_mars = Utc.with_ymd_and_hms(2026, 3, 15, 0, 0, 0).unwrap();
        assert_eq!(paris_hhmm(&ev, mi_mars), "14:30");
    }

    /// La réouverture CME 18:00 New York tombe toujours à 00:00 Paris —
    /// c'est la réconciliation or mesurée le 28/09.
    #[test]
    fn reouverture_cme_toujours_minuit_paris() {
        let ev = crene(New_York, 18, 0, &[1, 2, 3, 4, 5]);
        let ete = Utc.with_ymd_and_hms(2026, 7, 15, 0, 0, 0).unwrap();
        let hiver = Utc.with_ymd_and_hms(2026, 1, 15, 0, 0, 0).unwrap();
        assert_eq!(paris_hhmm(&ev, ete), "00:00");
        assert_eq!(paris_hhmm(&ev, hiver), "00:00");
    }

    /// Londres et Paris basculent ensemble : le fix du matin reste à 11:30.
    #[test]
    fn fix_lbma_am_stable() {
        let ev = crene(chrono_tz::Europe::London, 10, 30, &[1, 2, 3, 4, 5]);
        let ete = Utc.with_ymd_and_hms(2026, 7, 15, 0, 0, 0).unwrap();
        let hiver = Utc.with_ymd_and_hms(2026, 1, 15, 0, 0, 0).unwrap();
        assert_eq!(paris_hhmm(&ev, ete), "11:30");
        assert_eq!(paris_hhmm(&ev, hiver), "11:30");
    }

    /// La réouverture hebdo est un événement du DIMANCHE uniquement (17h NY
    /// = 23h Paris).
    #[test]
    fn reouverture_hebdo_le_dimanche() {
        let ev = crene(New_York, 17, 0, &[7]);
        let vendredi = Utc.with_ymd_and_hms(2026, 9, 25, 0, 0, 0).unwrap();
        let p = prochaine_occurrence(&ev, vendredi)
            .expect("occurrence")
            .with_timezone(&chrono_tz::Europe::Paris);
        assert_eq!(p.weekday().number_from_monday(), 7);
        assert_eq!(format!("{:02}:{:02}", p.hour(), p.minute()), "23:00");
    }

    /// La réouverture or (owner 15/10) : lundi 00h00 Paris, en été comme en
    /// hiver (00h00 local est 00h00 local quelle que soit la bascule).
    #[test]
    fn reouverture_or_lundi_minuit_paris() {
        let ev = crene(chrono_tz::Europe::Paris, 0, 0, &[1]);
        // Un vendredi soir : la prochaine occurrence doit être lundi minuit.
        let vendredi = Utc.with_ymd_and_hms(2026, 10, 16, 20, 0, 0).unwrap();
        let p = prochaine_occurrence(&ev, vendredi)
            .expect("occurrence")
            .with_timezone(&chrono_tz::Europe::Paris);
        assert_eq!(p.weekday().number_from_monday(), 1);
        assert_eq!(format!("{:02}:{:02}", p.hour(), p.minute()), "00:00");
    }

    /// Les événements de semaine ne matchent jamais un samedi new-yorkais :
    /// la fenêtre du samedi 18:00 ET (marché fermé) doit être vide.
    #[test]
    fn fenetres_ignorent_le_samedi() {
        let ev = crene(New_York, 18, 0, &[1, 2, 3, 4, 5]);
        let samedi = Utc.with_ymd_and_hms(2026, 9, 12, 22, 0, 0).unwrap();
        let t = fenetres_evenement(&ev, samedi.timestamp(), samedi.timestamp());
        assert!(t.is_empty(), "aucune fenêtre attendue le samedi");
        let lundi = Utc.with_ymd_and_hms(2026, 9, 14, 22, 0, 0).unwrap();
        let t = fenetres_evenement(&ev, lundi.timestamp() - 86_400, lundi.timestamp());
        assert_eq!(t.len(), 3);
    }

    /// Tokyo n'a pas d'heure d'été, Paris oui : l'ouverture 9h Tokyo est à
    /// 2h Paris en été et 1h en hiver — la machinerie DST suit toute seule.
    #[test]
    fn ouverture_tokyo_suit_les_bascules_paris() {
        let ev = crene(chrono_tz::Asia::Tokyo, 9, 0, &[1, 2, 3, 4, 5]);
        let ete = Utc.with_ymd_and_hms(2026, 7, 15, 0, 0, 0).unwrap();
        let hiver = Utc.with_ymd_and_hms(2026, 1, 15, 0, 0, 0).unwrap();
        assert_eq!(paris_hhmm(&ev, ete), "02:00");
        assert_eq!(paris_hhmm(&ev, hiver), "01:00");
    }

    /// Le classificateur mappe les titres ForexFactory réels vers les types.
    #[test]
    fn classifie_les_titres_reels() {
        assert_eq!(classifie_annonce("Non-Farm Employment Change"), Some("cal_nfp"));
        assert_eq!(classifie_annonce("CPI m/m"), Some("cal_cpi"));
        assert_eq!(classifie_annonce("Core CPI m/m"), Some("cal_cpi"));
        assert_eq!(classifie_annonce("FOMC Statement"), Some("cal_fomc"));
        assert_eq!(classifie_annonce("Federal Funds Rate"), Some("cal_fomc"));
        assert_eq!(classifie_annonce("Core PCE Price Index m/m"), Some("cal_pce"));
        assert_eq!(classifie_annonce("Advance GDP q/q"), Some("cal_gdp"));
        assert_eq!(classifie_annonce("Retail Sales m/m"), Some("cal_retail"));
        assert_eq!(classifie_annonce("Unemployment Claims"), Some("cal_jobless"));
        assert_eq!(classifie_annonce("Personal Spending m/m"), Some("cal_perso"));
        assert_eq!(classifie_annonce("ISM Manufacturing PMI"), None, "non classée → filet cal_autres");
        assert_eq!(
            classifie_annonce("Personal Income and Core PCE Price Index"),
            Some("cal_pce"),
            "PCE prime sur Personal (spécificité décroissante)"
        );
    }
}
