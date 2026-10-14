//! §Straddle événements (28/09) — taxonomie + matrice de réactivité.
//!
//! Question propriétaire : « quels événements PRÉVISIBLES génèrent le plus de
//! liquidité, et quels assets y réagissent le mieux ? » Les créneaux
//! statistiques (`creneaux_ia`) savent QUANDIL y a de la volatilité, pas
//! POURQUOI. Ici, chaque événement à l'horloge (ouvertures, fixes LBMA,
//! réouverture CME, slots d'annonces US) est défini dans SON fuseau d'origine
//! : la conversion en heure de Paris suit automatiquement les bascules
//! d'heure d'été — y compris les 2 fenêtres de l'année où l'Europe et les
//! États-Unis ne sont plus alignés (annonces 8:30 NY = 13:30 Paris, pas
//! 14:30, pendant ~3 semaines).
//!
//! La matrice mesure, sur 120 jours de M1, l'ATR des 3 premières minutes de
//! chaque occurrence rapporté à l'habitude de l'asset (×N). Elle ne décide
//! RIEN : c'est la matière première de l'armement événement par événement
//! (phase 3, décision propriétaire).


use chrono::{DateTime, Datelike, Duration, TimeZone, Utc};
use chrono_tz::Tz;


// ── Taxonomie : événements prévisibles à l'horloge ───────────────────────────
//
// DOCTRINE (owner 14/10, actée) — deux familles, deux sources de vérité :
//   • STRUCTURELS (calendrier = false) : événements récurrents garantis par
//     le calendrier des marchés (ouvertures, fixes, clôtures, réouvertures).
//     Le catalogue fixe leur HEURE ; la matrice (`creneaux_evenements`)
//     décide des ASSETS armés.
//   • CALENDRIER (calendrier = true) : types d'annonces (NFP, CPI, FOMC…,
//     filet cal_autres). Leur existence et leur heure au quotidien relèvent
//     du CALENDRIER ÉCONOMIQUE SEUL (annonces High — décision owner 14/10 :
//     les Medium ne se tradent pas). La matrice les arme comme les autres.
//   Les anciens créneaux génériques « annonces US 8:30 / 10:00 » (les seuls à
//   tirer sur Medium) ont été RETIRÉS le 14/10 : doublon des types ci-dessus.

/// Un événement récurrent, défini dans son fuseau d'origine. Les jours sont
/// ISO lundi=1..dimanche=7 **dans le fuseau de l'événement**.
pub(crate) struct EvenementModele {
    pub(crate) ident: &'static str,
    pub(crate) nom: &'static str,
    pub(crate) detail: &'static str,
    pub(crate) tz: Tz,
    pub(crate) heure: u32,
    pub(crate) minute: u32,
    pub(crate) jours: &'static [u32],
    /// true = créneau « annonce » : ne tire QUE si une annonce réelle USD
    /// (High ou Medium) existe au calendrier à la minute du slot (owner
    /// 29/09 — « entrer uniquement sur les annonces réelles »). Les
    /// événements de marché (ouvertures, fixs, réouvertures) ont leur
    /// cause chaque jour : jamais gated.
    /// true = TYPE CALENDAIRE (owner 10/10) : pas d'heure fixe — l'heure
    /// vient de la VRAIE annonce du calendrier, classée par titre. tz/heure/
    /// jours sont ignorés pour ces lignes (placeholder New York 8:30).
    pub(crate) calendrier: bool,
}

/// Fixe le monde : Londres et Paris basculent ensemble (dernier dimanche de
/// mars/octobre), New York bascule aux siennes (2e dimanche de mars,
/// 1er de novembre) — l'écart New York↔Paris est de 6 h sauf pendant ces
/// deux fenêtres d'entre-bascules où il passe à 5 h.
pub(crate) const EVENEMENTS: &[EvenementModele] = &[
    EvenementModele {
        ident: "eu_ouverture",
        nom: "Ouverture Francfort + Londres",
        detail: "Xetra ouvre à 09:00 Paris et la session forex de Londres à 08:00 UK — la même minute. Le DAX y prend son plus gros range de la matinée européenne.",
        tz: chrono_tz::Europe::Paris,
        heure: 9,
        minute: 0,
        jours: &[1, 2, 3, 4, 5],
        calendrier: false,
    },
    EvenementModele {
        ident: "lbma_am",
        nom: "Fix LBMA matin (or)",
        detail: "Enchère de fixing de l'or de Londres (10:30 UK) : les banques concentrent leurs ordres, l'or et l'argent bougent à la minute.",
        tz: chrono_tz::Europe::London,
        heure: 10,
        minute: 30,
        jours: &[1, 2, 3, 4, 5],
        calendrier: false,
    },
    EvenementModele {
        ident: "nyse_ouverture",
        nom: "Ouverture NYSE",
        detail: "Ouverture de la séance actions américaine (9:30 New York) : les indices et tout ce qui s'y réfère cherchent leur prix d'équilibre.",
        tz: chrono_tz::America::New_York,
        heure: 9,
        minute: 30,
        jours: &[1, 2, 3, 4, 5],
        calendrier: false,
    },
    EvenementModele {
        ident: "nyse_cloture",
        nom: "Clôture NYSE",
        detail: "Sonnerie de clôture (16:00 New York) : derniers équilibres du jour sur les indices US.",
        tz: chrono_tz::America::New_York,
        heure: 16,
        minute: 0,
        jours: &[1, 2, 3, 4, 5],
        calendrier: false,
    },
    EvenementModele {
        ident: "londres_cloture",
        nom: "Clôture Londres (forex)",
        detail: "Fin de la session européenne à 17:00 UK : la liquidité forex se raréfie d'un coup avant le relais américain de l'après-midi.",
        tz: chrono_tz::Europe::London,
        heure: 17,
        minute: 0,
        jours: &[1, 2, 3, 4, 5],
        calendrier: false,
    },
    EvenementModele {
        ident: "cme_reouverture",
        nom: "Réouverture CME (or, indices)",
        detail: "Fin de la pause quotidienne 17:00-18:00 New York : les futures métaux et indices reprennent — première minute la plus nerveuse de la journée pour l'or et l'argent.",
        tz: chrono_tz::America::New_York,
        heure: 18,
        minute: 0,
        jours: &[1, 2, 3, 4, 5],
        calendrier: false,
    },
    EvenementModele {
        ident: "marche_reouverture_hebdo",
        nom: "Réouverture week-end (forex, CFD)",
        detail: "Dimanche 17:00 New York : après ~47 h de fermeture, les cotations MT5 reprennent — le gap de week-end se prix immédiatement.",
        tz: chrono_tz::America::New_York,
        heure: 17,
        minute: 0,
        jours: &[7],
        calendrier: false,
    },
    EvenementModele {
        ident: "asie_ouverture",
        nom: "Ouverture Tokyo (session asiatique)",
        detail: "9h00 Tokyo : le relais asiatique prend la liquidité — 2h00 Paris en été, 1h00 en hiver (Tokyo ne bascule jamais d'heure, Paris oui).",
        tz: chrono_tz::Asia::Tokyo,
        heure: 9,
        minute: 0,
        jours: &[1, 2, 3, 4, 5],
        calendrier: false,
    },
    // ── Types calendrier (owner 10/10) : l'heure vient de la vraie annonce ──
    EvenementModele {
        ident: "cal_nfp",
        nom: "NFP — emploi US (1er vendredi)",
        detail: "Non-Farm Payrolls et taux de chômage : la plus grosse volatilité mensuelle du calendrier.",
        tz: chrono_tz::America::New_York,
        heure: 8,
        minute: 30,
        jours: &[1, 2, 3, 4, 5],
        calendrier: true,
    },
    EvenementModele {
        ident: "cal_cpi",
        nom: "CPI US — inflation",
        detail: "Indice des prix à la consommation (CPI/cœur) : le chiffre qui décide des attentes de taux.",
        tz: chrono_tz::America::New_York,
        heure: 8,
        minute: 30,
        jours: &[1, 2, 3, 4, 5],
        calendrier: true,
    },
    EvenementModele {
        ident: "cal_fomc",
        nom: "FOMC — taux + conférence",
        detail: "Décision de taux et conférence de la Fed : 14h00 puis 14h30 New York les jours de réunion.",
        tz: chrono_tz::America::New_York,
        heure: 14,
        minute: 0,
        jours: &[1, 2, 3, 4, 5],
        calendrier: true,
    },
    EvenementModele {
        ident: "cal_pce",
        nom: "PCE US — inflation Fed",
        detail: "Déflateur PCE (mesure d'inflation préférée de la Fed), 8:30 New York.",
        tz: chrono_tz::America::New_York,
        heure: 8,
        minute: 30,
        jours: &[1, 2, 3, 4, 5],
        calendrier: true,
    },
    EvenementModele {
        ident: "cal_gdp",
        nom: "PIB US (GDP)",
        detail: "Croissance trimestrielle (advance/second/third estimate), 8:30 New York.",
        tz: chrono_tz::America::New_York,
        heure: 8,
        minute: 30,
        jours: &[1, 2, 3, 4, 5],
        calendrier: true,
    },
    EvenementModele {
        ident: "cal_retail",
        nom: "Retail Sales US",
        detail: "Ventes au détail (et core retail), 8:30 New York — pouls du consommateur.",
        tz: chrono_tz::America::New_York,
        heure: 8,
        minute: 30,
        jours: &[1, 2, 3, 4, 5],
        calendrier: true,
    },
    EvenementModele {
        ident: "cal_jobless",
        nom: "Jobless Claims US (jeudi)",
        detail: "Inscriptions hebdo au chômage, 8:30 New York chaque jeudi.",
        tz: chrono_tz::America::New_York,
        heure: 8,
        minute: 30,
        jours: &[4],
        calendrier: true,
    },
    EvenementModele {
        ident: "cal_perso",
        nom: "Dépenses personnelles US",
        detail: "Personal Spending / Personal Income, 8:30 New York.",
        tz: chrono_tz::America::New_York,
        heure: 8,
        minute: 30,
        jours: &[1, 2, 3, 4, 5],
        calendrier: true,
    },
    EvenementModele {
        ident: "cal_autres",
        nom: "Autres annonces US High",
        detail: "Filet : toute annonce High non classée dans un type ci-dessus (ISM, confiance, ventes de logements…) — armée par défaut pour ne perdre aucune passe.",
        tz: chrono_tz::America::New_York,
        heure: 8,
        minute: 30,
        jours: &[1, 2, 3, 4, 5],
        calendrier: true,
    },
];

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
pub(crate) fn fenetres_evenement(ev: &EvenementModele, debut: i64, fin: i64) -> Vec<i64> {
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

/// La taxonomie complète (agenda, armement).
pub(crate) fn catalogue() -> &'static [EvenementModele] {
    EVENEMENTS
}

/// Prochaine occurrence strictement future, en absolu UTC.
pub(crate) fn prochaine_occurrence(ev: &EvenementModele, maintenant: DateTime<Utc>) -> Option<DateTime<Utc>> {
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

// ── Tests : conversion DST et fenêtres ───────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Timelike;
    use std::collections::HashMap;
    use chrono_tz::America::New_York;

    fn ev(ident: &str) -> &'static EvenementModele {
        EVENEMENTS.iter().find(|e| e.ident == ident).expect("événement inconnu")
    }

    fn paris_hhmm(ident: &str, utc: DateTime<Utc>) -> String {
        let p = prochaine_occurrence(ev(ident), utc - Duration::hours(12))
            .expect("occurrence")
            .with_timezone(&chrono_tz::Europe::Paris);
        format!("{:02}:{:02}", p.hour(), p.minute())
    }

    /// 9:30 New York = 15:30 Paris toute l'année hors entre-bascules :
    /// l'écart New York↔Paris est structurellement de 6 h.
    #[test]
    fn nyse_ouverture_stable_entre_ete_et_hiver() {
        // Plein été (EDT / CEST) et plein hiver (EST / CET).
        let ete = Utc.with_ymd_and_hms(2026, 7, 15, 0, 0, 0).unwrap();
        let hiver = Utc.with_ymd_and_hms(2026, 1, 15, 0, 0, 0).unwrap();
        assert_eq!(paris_hhmm("nyse_ouverture", ete), "15:30");
        assert_eq!(paris_hhmm("nyse_ouverture", hiver), "15:30");
    }

    /// LA fenêtre piège : l'Europe bascule le dernier dimanche d'octobre,
    /// les États-Unis le 1er novembre — entre les deux, l'écart passe à 5 h
    /// et l'ouverture NYSE tombe à 14:30 Paris.
    #[test]
    fn entre_bascules_octobre_nyse_a_1430_paris() {
        let lundi_oct = Utc.with_ymd_and_hms(2026, 10, 26, 0, 0, 0).unwrap();
        assert_eq!(paris_hhmm("nyse_ouverture", lundi_oct), "14:30");
        // Idem au printemps : les US avancent le 2e dimanche de mars, l'Europe
        // le dernier — trois semaines de décalage.
        let mi_mars = Utc.with_ymd_and_hms(2026, 3, 15, 0, 0, 0).unwrap();
        assert_eq!(paris_hhmm("nyse_ouverture", mi_mars), "14:30");
    }

    /// La réouverture CME 18:00 New York tombe toujours à 00:00 Paris —
    /// c'est la réconciliation or mesurée le 28/09.
    #[test]
    fn reouverture_cme_toujours_minuit_paris() {
        let ete = Utc.with_ymd_and_hms(2026, 7, 15, 0, 0, 0).unwrap();
        let hiver = Utc.with_ymd_and_hms(2026, 1, 15, 0, 0, 0).unwrap();
        assert_eq!(paris_hhmm("cme_reouverture", ete), "00:00");
        assert_eq!(paris_hhmm("cme_reouverture", hiver), "00:00");
    }

    /// Londres et Paris basculent ensemble : le fix du matin reste à 11:30.
    #[test]
    fn fix_lbma_am_stable() {
        let ete = Utc.with_ymd_and_hms(2026, 7, 15, 0, 0, 0).unwrap();
        let hiver = Utc.with_ymd_and_hms(2026, 1, 15, 0, 0, 0).unwrap();
        assert_eq!(paris_hhmm("lbma_am", ete), "11:30");
        assert_eq!(paris_hhmm("lbma_am", hiver), "11:30");
    }

    /// La réouverture hebdo est un événement du DIMANCHE uniquement.
    #[test]
    fn reouverture_hebdo_le_dimanche() {
        // Un vendredi : la prochaine occurrence doit retomber dimanche.
        let vendredi = Utc.with_ymd_and_hms(2026, 9, 25, 0, 0, 0).unwrap();
        let p = prochaine_occurrence(ev("marche_reouverture_hebdo"), vendredi)
            .expect("occurrence")
            .with_timezone(&chrono_tz::Europe::Paris);
        assert_eq!(p.weekday().number_from_monday(), 7);
        assert_eq!(format!("{:02}:{:02}", p.hour(), p.minute()), "23:00");
    }

    /// Les événements de semaine ne matchent jamais un samedi new-yorkais :
    /// la fenêtre du samedi 18:00 ET (marché fermé) doit être vide.
    #[test]
    fn fenetres_ignorent_le_samedi() {
        // Samedi 12 septembre 2026, 22:00 UTC : 18:00 à New York.
        let samedi = Utc.with_ymd_and_hms(2026, 9, 12, 22, 0, 0).unwrap();
        let t = fenetres_evenement(ev("cme_reouverture"), samedi.timestamp(), samedi.timestamp());
        assert!(t.is_empty(), "aucune fenêtre attendue le samedi");
        // Le lundi suivant en revanche : 3 minutes de fenêtre.
        let lundi = Utc.with_ymd_and_hms(2026, 9, 14, 22, 0, 0).unwrap();
        let t = fenetres_evenement(ev("cme_reouverture"), lundi.timestamp() - 86_400, lundi.timestamp());
        assert_eq!(t.len(), 3);
    }

}

#[cfg(test)]
mod tests_calendrier {
    use super::*;
    use chrono::Timelike;

    fn ev(ident: &str) -> &'static EvenementModele {
        EVENEMENTS.iter().find(|e| e.ident == ident).expect("événement inconnu")
    }

    /// Tokyo n'a pas d'heure d'été, Paris oui : l'ouverture 9h Tokyo est à
    /// 2h Paris en été (CEST=UTC+2, JST=UTC+9, écart 7h) et 1h en hiver
    /// (écart 8h) — la machinerie DST doit suivre toute seule.
    #[test]
    fn ouverture_tokyo_suit_les_bascules_paris() {
        let ete = Utc.with_ymd_and_hms(2026, 7, 15, 0, 0, 0).unwrap();
        let hiver = Utc.with_ymd_and_hms(2026, 1, 15, 0, 0, 0).unwrap();
        for (now, attendu) in [(ete, "02:00"), (hiver, "01:00")] {
            let p = prochaine_occurrence(ev("asie_ouverture"), now - Duration::hours(12))
                .expect("occurrence")
                .with_timezone(&chrono_tz::Europe::Paris);
            assert_eq!(format!("{:02}:{:02}", p.hour(), p.minute()), attendu);
        }
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
    }

    /// PCE contient « Personal » : la spécificité doit primer (PCE avant
    /// dépenses personnelles dans l'ordre de classification).
    #[test]
    fn pce_prime_sur_personal() {
        assert_eq!(classifie_annonce("Personal Income and Core PCE Price Index"), Some("cal_pce"));
    }

    /// Séparation des familles : les 8 structurels (9 fixes d'origine +
    /// Tokyo − les 2 créneaux annonces retirés le 14/10) vs les 9 types.
    #[test]
    fn familles_fixes_vs_calendrier() {
        let fixes = EVENEMENTS.iter().filter(|e| !e.calendrier).count();
        let cal = EVENEMENTS.iter().filter(|e| e.calendrier).count();
        assert_eq!(fixes, 8, "structurels (créneaux annonces retirés 14/10)");
        assert_eq!(cal, 9, "8 types + filet cal_autres");
    }
}
