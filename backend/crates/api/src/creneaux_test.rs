//! §16-b (07/09) — boucle de validation des créneaux IA armés.
//!
//! Chaque créneau armé tire chaque semaine (annonce synthétique → passes
//! straddle normales). Ici, on regarde ce qu'il a produit : les passes
//! closes sont rapprochées du créneau via le ts embarqué dans la clé
//! moteur (`straddle-{asset}-{ts}` = l'heure pile du tirage, jour+heure
//! Paris), puis la boucle quotidienne statue au bout de N tirages
//! (kv `creneaux_test_min`, défaut 4) :
//!
//! - ΣR > 0 → **valide** : pilier, reste armé ;
//! - ΣR ≤ plancher (kv `creneaux_test_plancher_r`, défaut −1,5) ou
//!   0 gagnant → **refute** : désarmé automatiquement, slot libéré —
//!   règle moteur aux seuils du propriétaire (l'armement, lui, reste à
//!   l'humain, constitution §3) ;
//! - entre les deux → **incertain** : prolongé de 2 tirages, puis tranché
//!   (sans preuve à l'échéance = réfuté).

use chrono::{Datelike, Timelike};
use sqlx::Row;

use crate::creneaux_ia::JOURS;
use db::Database;

/// Réglages propriétaires de la boucle (kv, défauts sains) :
/// N tirages minimum avant verdict, plancher ΣR de réfutation.
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

/// Agrège les passes closes de chaque créneau armé (depuis son armement)
/// et statue au bout de N tirages. Appelée au boot puis quotidiennement
/// (4h Paris) par `creneaux_ia::boucle`.
pub async fn statuer(db: &Database) {
    let (seuil_min, plancher_r) = lire_seuils(db).await;
    let armes = sqlx::query(
        "SELECT asset, jour, heure, arme_le, verdict_test FROM creneaux_ia WHERE arme = 1",
    )
    .fetch_all(db.pool())
    .await
    .unwrap_or_default();

    for r in &armes {
        let asset: String = r.get("asset");
        let jour = r.get::<i64, _>("jour").clamp(1, 7);
        let heure = r.get::<i64, _>("heure").clamp(0, 23);
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

        // Rapprochement passe↔créneau : le ts de la clé moteur est l'heure
        // pile du tirage — jour + heure Paris doivent matcher le créneau.
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
                let paris = chrono::DateTime::from_timestamp(ts, 0)?
                    .with_timezone(&chrono_tz::Europe::Paris);
                (paris.weekday().number_from_monday() as i64 == jour
                    && paris.hour() as i64 == heure)
                    .then_some(r_val)
            })
            .collect();
        let occurrences = rs.len() as i64;
        let somme_r: f64 = rs.iter().sum();
        let gagnants = rs.iter().filter(|x| **x > 0.0).count() as i64;

        let _ = sqlx::query(
            "UPDATE creneaux_ia SET occurrences = ?, somme_r = ? WHERE asset = ? AND jour = ? AND heure = ?",
        )
        .bind(occurrences)
        .bind(somme_r)
        .bind(&asset)
        .bind(jour)
        .bind(heure)
        .execute(db.pool())
        .await;

        // Conclus et armé (pilier valide) : les stats continuent d'être
        // rafraîchies ci-dessus, plus de verdict à rendre.
        if matches!(en_test.as_deref(), Some("valide") | Some("refute")) {
            continue;
        }
        // Incertain = prolongation de 2 tirages au-delà du seuil de base.
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
            ("refute", true) // prolongation échue sans preuve
        } else {
            ("incertain", false)
        };
        let _ = sqlx::query(
            "UPDATE creneaux_ia SET verdict_test = ?, conclut_le = strftime('%s','now'),
                    arme = CASE WHEN ? THEN 0 ELSE arme END
             WHERE asset = ? AND jour = ? AND heure = ?",
        )
        .bind(verdict)
        .bind(desarmer)
        .bind(&asset)
        .bind(jour)
        .bind(heure)
        .execute(db.pool())
        .await;
        let libelle = format!(
            "{} {} {}h-{}h",
            asset,
            JOURS[(jour - 1) as usize],
            heure,
            heure + 1
        );
        if verdict == "valide" {
            tracing::info!(
                "🤖 Créneaux IA : {libelle} VALIDÉ après {occurrences} tirages (Σ{somme_r:+.2}R) — pilier, reste armé"
            );
        } else if verdict == "refute" {
            tracing::info!(
                "🤖 Créneaux IA : {libelle} RÉFUTÉ après {occurrences} tirages (Σ{somme_r:+.2}R, {gagnants} gagnant(s)) — désarmé, slot libéré"
            );
        } else {
            tracing::info!(
                "🤖 Créneaux IA : {libelle} incertain (Σ{somme_r:+.2}R sur {occurrences}) — prolongé de 2 tirages"
            );
        }
    }
}
