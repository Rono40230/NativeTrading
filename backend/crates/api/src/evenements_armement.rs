//! Armement événementiel straddle (phase 3, 28/09) — extrait d'evenements.rs
//! (limite 600 lignes). Créneaux (asset × événement) armables, annonces
//! synthétiques au rail M1, gate « annonces réelles » (owner 29/09) et
//! boucle de validation §16-b transposée.


use chrono::{DateTime, Datelike, Duration, TimeZone, Timelike, Utc};
use sqlx::Row;

use crate::evenements::{EVENEMENTS, EvenementModele};
use db::Database;

// ── Phase 3 (28/09) : armement événementiel ──────────────────────────────────
//
// Décisions propriétaire : balayage large (tous les événements × tous les
// assets du périmètre, tout armé), REMPLACEMENT des créneaux statistiques
// (désarmés en migration 0115, lignes archivées), XAGUSD au périmètre.
// Un créneau armé devient des annonces synthétiques au rail M1 (moteur
// straddle inchangé : 2 jambes à T-10 s, Observation) et la boucle de
// validation statue comme pour les créneaux IA d'avant (§16-b) : ΣR > 0
// → valide, ΣR ≤ plancher ou 0 gagnant → réfuté (désarmé), sinon incertain
// prolongé 2 tirages. L'armement reste au propriétaire seul.

/// Horizon d'annonces générées par créneau armé : les moteurs sont montés
/// à l'ajout du couple (pas à chaque tick 60 s) — 14 occurrences donnent
/// 14 jours de runway aux événements quotidiens (14 semaines aux hebdo),
/// au-delà un changement de périmètre ou un redémarrage re-synchronise.
pub(crate) const OCCURRENCES_ANNONCES: usize = 14;

/// Boot : périmètre (XAGUSD entre par défaut si jamais configuré — décision
/// owner 28/09) + semis des créneaux événements. Idempotent : les lignes
/// existantes gardent leurs stats, seules les manquantes sont créées armées.
pub async fn initialiser(db: &Database) {
    if db
        .lire_config("perimetre_straddle")
        .await
        .ok()
        .flatten()
        .is_none()
    {
        let _ = db
            .ecrire_config(
                "perimetre_straddle",
                r#"["XAUUSD","BTC","DAX","NAS100","SP500","XAGUSD"]"#,
            )
            .await;
        tracing::info!("📯 Périmètre straddle : XAGUSD ajouté (défaut enrichi, décision owner 28/09)");
    }
    semer(db).await;
}

/// Crée armés les créneaux (périmètre × taxonomie) qui n'existent pas.
pub async fn semer(db: &Database) {
    let perimetre = crate::runtime_perimetre::lire_perimetre_straddle(db).await;
    let maintenant = Utc::now().timestamp();
    let mut crees = 0u32;
    for asset in &perimetre {
        for ev in EVENEMENTS {
            let res = sqlx::query(
                "INSERT OR IGNORE INTO creneaux_evenements (asset, evenement, arme, arme_le)
                 VALUES (?, ?, 1, ?)",
            )
            .bind(asset)
            .bind(ev.ident)
            .bind(maintenant)
            .execute(db.pool())
            .await;
            if res.map(|r| r.rows_affected()).unwrap_or(0) > 0 {
                crees += 1;
            }
        }
    }
    if crees > 0 {
        tracing::info!(
            "📯 Créneaux événements : {crees} créé(s) armé(s) — {} asset(s) × {} événement(s)",
            perimetre.len(),
            EVENEMENTS.len()
        );
    }
}

/// Les N prochaines occurrences d'un événement, en absolu UTC croissant.
pub(crate) fn occurrences_suivantes(ev: &EvenementModele, maintenant: DateTime<Utc>, n: usize) -> Vec<i64> {
    let local = maintenant.with_timezone(&ev.tz);
    let mut out = Vec::new();
    for delta in 0..=(n as i64 * 7 + 7) {
        if out.len() >= n {
            break;
        }
        let jour = (local + Duration::days(delta)).date_naive();
        if !ev.jours.contains(&jour.weekday().number_from_monday()) {
            continue;
        }
        let naive = match jour.and_hms_opt(ev.heure, ev.minute, 0) {
            Some(x) => x,
            None => continue,
        };
        if let Some(absolu) = ev.tz.from_local_datetime(&naive).single() {
            if absolu > local {
                out.push(absolu.with_timezone(&Utc).timestamp());
            }
        }
    }
    out
}

/// Parse une date_heure du cache calendrier (RFC3339, parfois secondes
/// tronquées « +02:0 ») en epoch secondes.
pub(crate) fn parse_ts_calendrier(dh: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(dh)
        .or_else(|_| {
            chrono::DateTime::parse_from_rfc3339(&format!(
                "{}:{}",
                &dh[..dh.len().saturating_sub(2)],
                &dh[dh.len().saturating_sub(2)..]
            ))
        })
        .ok()
        .map(|d| d.timestamp())
}

/// Annonces USD HIGH à venir (≤ horizon) : (ts, titre) — matière première
/// des types calendrier (owner 10/10) et du garde des créneaux annonces.
pub(crate) async fn annonces_high_futures(
    db: &Database,
    horizon_sec: i64,
) -> Vec<(i64, String)> {
    let rows = sqlx::query(
        "SELECT date_heure, titre FROM calendrier_cache
         WHERE devise = 'USD' AND impact = 'High'",
    )
    .fetch_all(db.pool())
    .await
    .unwrap_or_default();
    let maintenant = Utc::now().timestamp();
    rows.iter()
        .filter_map(|r| {
            let dh: String = r.try_get("date_heure").ok()?;
            let titre: String = r.try_get("titre").ok()?;
            let ts = parse_ts_calendrier(&dh)?;
            (ts > maintenant && ts <= maintenant + horizon_sec).then_some((ts, titre))
        })
        .collect()
}

/// Minutes (epoch/60) des annonces réelles USD High/Medium — le garde du
/// propriétaire : les créneaux « annonces US » ne tirent que si une vraie
/// annonce existe à la minute du slot.
async fn minutes_annonces_reelles(db: &Database) -> std::collections::HashSet<i64> {
    let rows = sqlx::query(
        "SELECT date_heure FROM calendrier_cache
         WHERE devise = 'USD' AND impact IN ('High', 'Medium')",
    )
    .fetch_all(db.pool())
    .await
    .unwrap_or_default();
    rows.iter()
        .filter_map(|r| {
            let dh: String = r.try_get("date_heure").ok()?;
            Some(parse_ts_calendrier(&dh)?.div_euclid(60))
        })
        .collect()
}

/// Annonces synthétiques des créneaux ÉVÉNEMENT armés d'un asset — même
/// rail que les tier 1 et les anciens créneaux IA : le moteur straddle M1
/// trie par ts, pose les 2 jambes à T-10 s de la première occurrence.
/// Annonces du rail UNIFIÉ (owner 10/10) pour un asset : les créneaux à
/// heure fixe armés (annonces synthétiques, garde calendrier pour les
/// slots « annonce ») + les annonces RÉELLES High dont le type est armé
/// dans la matrice (classification par titre, filet « cal_autres »).
/// C'est la SEULE source du moteur straddle — plus de rail tier 1
/// inconditionnel : chaque passe passe par une case armée.
pub async fn annonces_evenements(db: &Database, asset: &str) -> Vec<straddle::Annonce> {
    let rows = sqlx::query(
        "SELECT evenement FROM creneaux_evenements WHERE asset = ? AND arme = 1",
    )
    .bind(asset)
    .fetch_all(db.pool())
    .await
    .unwrap_or_default();
    let maintenant = Utc::now();
    let devise = if asset == "DAX" { "EUR" } else { "USD" };
    let armes: Vec<&'static EvenementModele> = rows
        .iter()
        .filter_map(|r| {
            let ident: String = r.get("evenement");
            EVENEMENTS.iter().find(|e| e.ident == ident)
        })
        .collect();

    let mut out = Vec::new();
    // (b) D'ABORD les types CALENDAIRE armés : chaque annonce High réelle
    // classée dans un type armé devient une passe à SA vraie heure (les non
    // classées tombent dans le filet « cal_autres » s'il est armé). Leurs
    // minutes sont retenues : un slot fixe gardé à la même minute ne tire
    // PAS — sinon une NFP déclencherait DEUX passes (slot 8:30 + ligne 📅).
    let mut minutes_classees: std::collections::HashSet<i64> = std::collections::HashSet::new();
    if armes.iter().any(|e| e.calendrier) {
        for (ts, titre) in annonces_high_futures(db, 7 * 24 * 3600).await {
            let classe = crate::evenements::classifie_annonce(&titre);
            let ident = classe.unwrap_or("cal_autres");
            let Some(ev) = EVENEMENTS.iter().find(|e| e.ident == ident) else {
                continue;
            };
            if !armes.iter().any(|a| a.ident == ev.ident) {
                continue; // ce type n'est pas armé pour cet asset
            }
            minutes_classees.insert(ts.div_euclid(60));
            out.push(straddle::Annonce {
                ts,
                devise: devise.to_string(),
                titre: format!("📅 {} — {}", ev.nom, titre),
            });
        }
    }
    // (a) Ensuite les créneaux à heure fixe (toujours, ou gated par annonce
    // réelle). Déduplication owner 10/10 : un slot gardé dont la minute
    // coïncide avec une annonce déjà servie par sa ligne 📅 ne tire pas ;
    // sans annonce réelle à la minute du slot : silence (owner 29/09).
    let gates = armes.iter().any(|e| e.gate_calendrier);
    let minutes_reelles = if gates {
        minutes_annonces_reelles(db).await
    } else {
        std::collections::HashSet::new()
    };
    for ev in armes.iter().filter(|e| !e.calendrier) {
        for ts in occurrences_suivantes(ev, maintenant, OCCURRENCES_ANNONCES) {
            if ev.gate_calendrier
                && (minutes_classees.contains(&(ts / 60)) || !minutes_reelles.contains(&(ts / 60)))
            {
                continue;
            }
            out.push(straddle::Annonce {
                ts,
                devise: devise.to_string(),
                titre: format!("📯 {}", ev.nom),
            });
        }
    }
    out
}

/// Réglages propriétaires de la boucle de validation (kv, défauts sains) :
/// N tirages minimum avant verdict, plancher ΣR de réfutation. Mêmes clés
/// que l'ancienne boucle des créneaux IA — continuité des réglages.
pub async fn lire_seuils(db: &Database) -> (i64, f64) {
    let min = db
        .lire_config("creneaux_test_min")
        .await
        .ok()
        .flatten()
        .and_then(|s| s.parse::<i64>().ok())
        .filter(|n| (1..=52).contains(n))
        .unwrap_or(4);
    let plancher = db
        .lire_config("creneaux_test_plancher_r")
        .await
        .ok()
        .flatten()
        .and_then(|s| s.parse::<f64>().ok())
        .filter(|r| (-10.0..0.0).contains(r))
        .unwrap_or(-1.5);
    (min, plancher)
}

/// Une passe straddle (clé moteur `straddle-{asset}-{ts}`) appartient-elle
/// à cet événement ? Le ts de la clé EST l'instant de l'annonce : il
/// suffit de le relire dans le fuseau d'origine — jour et heure locale
/// exacts, DST comprise (une passe à 13:30 Paris pendant une entre-bascule
/// matche bien l'événement 8:30 New York).
fn passe_appartient(ts: i64, ev: &EvenementModele) -> bool {
    match DateTime::from_timestamp(ts, 0) {
        Some(utc) => {
            let locale = utc.with_timezone(&ev.tz);
            ev.jours.contains(&locale.weekday().number_from_monday())
                && locale.hour() == ev.heure
                && locale.minute() == ev.minute
        }
        None => false,
    }
}

/// Boucle de validation (§16-b transposée aux événements) : agrège les
/// passes closes de chaque créneau armé depuis son armement et statue au
/// bout de N tirages. Appelée au boot puis quotidiennement (4 h Paris).
pub async fn statuer_evenements(db: &Database) {
    let (seuil_min, plancher_r) = lire_seuils(db).await;
    let armes = sqlx::query(
        "SELECT asset, evenement, arme_le, verdict_test FROM creneaux_evenements WHERE arme = 1",
    )
    .fetch_all(db.pool())
    .await
    .unwrap_or_default();

    for r in &armes {
        let asset: String = r.get("asset");
        let ident: String = r.get("evenement");
        let Some(ev) = EVENEMENTS.iter().find(|e| e.ident == ident) else {
            continue;
        };
        let arme_le = r
            .try_get::<Option<i64>, _>("arme_le")
            .ok()
            .flatten()
            .unwrap_or(0);
        let en_test = r.try_get::<Option<String>, _>("verdict_test").ok().flatten();

        let passes = sqlx::query(
            "SELECT cle_moteur, r_realise FROM signaux
             WHERE strategie = 'straddle' AND asset = ? AND statut = 'Fermé'
               AND r_realise IS NOT NULL AND ferme_le IS NOT NULL AND ferme_le >= ?",
        )
        .bind(&asset)
        .bind(arme_le)
        .fetch_all(db.pool())
        .await
        .unwrap_or_default();

        // Rapprochement passe↔événement par l'heure LOCALE de l'événement.
        let rs: Vec<f64> = passes
            .iter()
            .filter_map(|p| {
                let cle: String = p.get("cle_moteur");
                let ts = cle
                    .strip_prefix("straddle-")?
                    .split('-')
                    .nth(1)?
                    .parse::<i64>()
                    .ok()?;
                let r_val = p.try_get::<f64, _>("r_realise").ok()?;
                passe_appartient(ts, ev).then_some(r_val)
            })
            .collect();
        let occurrences = rs.len() as i64;
        let somme_r: f64 = rs.iter().sum();
        let gagnants = rs.iter().filter(|x| **x > 0.0).count() as i64;

        let _ = sqlx::query(
            "UPDATE creneaux_evenements SET occurrences = ?, somme_r = ?
             WHERE asset = ? AND evenement = ?",
        )
        .bind(occurrences)
        .bind(somme_r)
        .bind(&asset)
        .bind(&ident)
        .execute(db.pool())
        .await;

        if matches!(en_test.as_deref(), Some("valide") | Some("refute")) {
            continue;
        }
        let seuil = if en_test.as_deref() == Some("incertain") {
            seuil_min + 2
        } else {
            seuil_min
        };
        if occurrences < seuil {
            continue;
        }
        let (verdict, desarmer) = if somme_r > 0.0 {
            ("valide", false)
        } else if somme_r <= plancher_r || gagnants == 0 {
            ("refute", true)
        } else if en_test.as_deref() == Some("incertain") {
            ("refute", true)
        } else {
            ("incertain", false)
        };
        let _ = sqlx::query(
            "UPDATE creneaux_evenements SET verdict_test = ?, conclut_le = strftime('%s','now'),
                    arme = CASE WHEN ? THEN 0 ELSE arme END
             WHERE asset = ? AND evenement = ?",
        )
        .bind(verdict)
        .bind(desarmer)
        .bind(&asset)
        .bind(&ident)
        .execute(db.pool())
        .await;
        if verdict == "valide" {
            tracing::info!(
                "📯 Événement {ident} × {asset} VALIDÉ après {occurrences} tirages (Σ{somme_r:+.2}R) — pilier, reste armé"
            );
        } else if verdict == "refute" {
            tracing::info!(
                "📯 Événement {ident} × {asset} RÉFUTÉ après {occurrences} tirages (Σ{somme_r:+.2}R, {gagnants} gagnant(s)) — désarmé"
            );
        } else {
            tracing::info!(
                "📯 Événement {ident} × {asset} incertain (Σ{somme_r:+.2}R sur {occurrences}) — prolongé de 2 tirages"
            );
        }
    }
}

/// Boucle de fond : semis idempotent + verdicts quotidiens (4 h Paris,
/// loin des sessions), au boot puis chaque nuit.
pub async fn boucle_validation(db: std::sync::Arc<Database>) {
    initialiser(&db).await;
    statuer_evenements(&db).await;
    loop {
        let maintenant = Utc::now().with_timezone(&chrono_tz::Europe::Paris);
        let prochain = (maintenant + Duration::days(1))
            .with_hour(4)
            .and_then(|d| d.with_minute(0))
            .unwrap_or(maintenant + Duration::hours(24));
        tokio::time::sleep(std::time::Duration::from_secs(
            (prochain - maintenant).num_seconds().max(60) as u64,
        ))
        .await;
        semer(&db).await;
        statuer_evenements(&db).await;
    }
}

#[cfg(test)]
mod tests_dst {
    use super::*;
    use chrono::TimeZone;

    fn ev(ident: &str) -> &'static EvenementModele {
        EVENEMENTS.iter().find(|e| e.ident == ident).expect("événement inconnu")
    }

    /// Une passe à 13:30 Paris pendant l'entre-bascule d'octobre appartient
    /// bien à l'événement 8:30 New York — le ts de la clé moteur est relu
    /// dans le fuseau d'ORIGINE, pas en heure Paris figée.
    #[test]
    fn passe_entre_bascules_appartient_aux_annonces_830() {
        let ev = ev("annonces_us_0830");
        // Lundi 26 octobre 2026, 13:30 Paris (l'Europe a reculé, les US pas).
        let ts = chrono_tz::Europe::Paris
            .with_ymd_and_hms(2026, 10, 26, 13, 30, 0)
            .unwrap()
            .timestamp();
        assert!(passe_appartient(ts, ev), "13:30 Paris ce jour-là = 8:30 NY");
        // 14:30 Paris le même jour : PAS l'heure de l'événement cette semaine.
        let ts_faux = chrono_tz::Europe::Paris
            .with_ymd_and_hms(2026, 10, 26, 14, 30, 0)
            .unwrap()
            .timestamp();
        assert!(!passe_appartient(ts_faux, ev));
    }
}

#[cfg(test)]
mod tests_occurrences {
    use super::*;
    use chrono::TimeZone;

    fn ev(ident: &str) -> &'static EvenementModele {
        EVENEMENTS.iter().find(|e| e.ident == ident).expect("événement inconnu")
    }

    /// 14 occurrences d'un événement quotidien = 14 jours ouvrés devant,
    /// toutes futures et croissantes (runway des annonces synthétiques).
    #[test]
    fn occurrences_suivantes_quotidiennes() {
        let ev = ev("cme_reouverture"); // lun-ven 18:00 New York
        let base = Utc.with_ymd_and_hms(2026, 9, 28, 12, 0, 0).unwrap(); // lundi
        let occ = occurrences_suivantes(ev, base, 14);
        assert_eq!(occ.len(), 14, "14 jours ouvrés devant");
        assert!(occ.windows(2).all(|w| w[0] < w[1]), "croissantes");
        assert!(occ.iter().all(|&t| t > base.timestamp()), "toutes futures");
    }
}
