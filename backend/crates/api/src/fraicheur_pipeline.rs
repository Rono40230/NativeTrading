//! Surveillance de fraîcheur du pipeline (owner 16/10) — AVERTIR, jamais
//! réparer (doctrine PIL).
//!
//! Pourquoi ce garde : la « Couverture par asset × timeframe » (page 📦)
//! mesure la PROFONDEUR d'historique contre un calendrier heuristique de
//! 24 mois — elle ne dit rien de la vitalité à l'instant. L'incident du
//! 07/10 (15 assets morts en silence) a été guéri par soft-delete + garde
//! « armé ⇒ alimenté », mais celle-ci ne protège que les assets ARMÉS :
//! un asset du pipeline non armé (XPTUSD…) pouvait cesser d'être nourri
//! sans que rien ne crie.
//!
//! Contrat vérifié ici, pour chaque asset ACTIF du pipeline et par TF :
//! - crypto (source binance) : 24/7 — M1 ≤ 10 min ;
//! - mt5 : semaine de cotation dimanche 23h → vendredi 23h Paris ; en
//!   semaine M1 ≤ 90 min (la pause quotidienne XAU 22h58→00h00 dure 1 h,
//!   les autres pauses quotidiennes sont plus courtes) ;
//! - TF supérieurs : seuil 2 × période + marge — les barres Binance M5→D1
//!   n'existent qu'à la CLÔTURE (une H4 saine a ouvert il y a 1 à 2
//!   périodes) et la D1 mt5 est FIGÉE après le push quotidien (le week-end
//!   la vieillit de 2 jours) : c'est normal, le compositeur tendance la
//!   remplace aux M1 fraîches ;
//! - hors semaine de cotation : aucune alerte mt5 — un marché fermé est
//!   sain d'être immobile.
//! Limite assumée : un jour férié en semaine déclenchera une fausse
//! alerte (warn uniquement — aucune action automatique).

use std::collections::HashMap;
use std::sync::{Arc, OnceLock, RwLock};

use actix_web::HttpResponse;
use chrono::{Datelike, TimeZone, Timelike, Utc};
use chrono_tz::Europe::Paris;
use sqlx::Row;

/// Les 7 TF canoniques (pas de W1 : aucune consommation temps réel depuis
/// le retrait de la ligne KAS).
pub const TFS: &[&str] = &["M1", "M5", "M15", "M30", "H1", "H4", "D1"];

/// Seuil d'alerte en minutes par TF. Sémantique réelle mesurée 16/10 : les
/// cryptos Binance stockent M5→D1 À LA CLÔTURE (la dernière fermée a ouvert
/// il y a 1 à 2 périodes : une H4 saine « vieillit » jusqu'à 8 h, une D1
/// jusqu'à 48 h) ; les MT5 poussent leurs barres formantes M1→H4 (retard
/// ∈ [0, période]) et la D1 est figée après le push quotidien. Seuil unique
/// couvrant les deux : 2 × période + marge — une alimentation morte est
/// signalée en ≤ 2 périodes + marge, une saine n'alerte jamais. Seule
/// exception M1 : crypto 24/7 = 10 min ; mt5 = 90 min (la pause quotidienne
/// XAU 22h58→00h00 dure 62 min sans qu'aucun M1 ne soit dû).
fn seuil_tf(tf: &str, crypto: bool) -> Option<i64> {
    Some(match tf {
        "M1" if crypto => 10,
        "M1" => 90,
        "M5" => 20,
        "M15" => 45,
        "M30" => 75,
        "H1" => 150,
        "H4" => 540,
        "D1" => 50 * 60,
        _ => return None,
    })
}

#[derive(serde::Serialize, Clone, PartialEq, Debug)]
pub struct Anomalie {
    pub tf: String,
    /// Retard en minutes ; −1 = aucune barre du tout en base.
    pub retard_min: i64,
    pub seuil_min: i64,
}

/// Minutes → durée lisible (owner 17/10 : jamais « 540 min ») : < 60 min
/// reste en minutes, au-delà en heures (« 9h », « 10h20 »).
pub fn duree_lisible(min: i64) -> String {
    if min < 60 {
        return format!("{min} min");
    }
    let (h, m) = (min / 60, min % 60);
    if m == 0 {
        format!("{h}h")
    } else {
        format!("{h}h{m:02}")
    }
}

#[derive(serde::Serialize, Clone, PartialEq, Debug)]
pub struct EtatAsset {
    pub asset: String,
    pub source: String,
    /// Semaine de cotation (toujours vrai pour les crypto).
    pub marche_ouvert: bool,
    pub anomalies: Vec<Anomalie>,
}

#[derive(serde::Serialize, Clone, Default)]
pub struct Audit {
    pub ts: i64,
    pub etats: Vec<EtatAsset>,
}

/// Dernier audit publié — lu par GET /api/assets/fraicheur et la santé.
static AUDIT: OnceLock<RwLock<Audit>> = OnceLock::new();
/// Anomalies du cycle précédent, pour ne journaliser que les transitions.
static PRECEDENTES: OnceLock<RwLock<HashMap<String, Vec<Anomalie>>>> = OnceLock::new();

/// Lit l'audit courant (sans DB) — endpoint et santé.
pub fn audit_courant() -> Audit {
    AUDIT
        .get_or_init(|| RwLock::new(Audit::default()))
        .read()
        .map(|a| a.clone())
        .unwrap_or_default()
}

/// Semaine de cotation mt5 : dimanche 23:00 → vendredi 23:00 Paris.
/// Le forex rouvre dimanche ~23h ; XAU lundi 00h (Réouverture or) — l'heure
/// d'avance du forex est absorbée par le seuil M1 de 90 min.
fn marche_ouvert_mt5(instant_paris: chrono::DateTime<chrono_tz::Tz>) -> bool {
    let jour = instant_paris.weekday().num_days_from_sunday(); // 0=dimanche
    let heure = instant_paris.hour();
    match jour {
        0 => heure >= 23,       // dimanche soir
        1..=4 => true,          // lundi → jeudi
        5 => heure < 23,        // vendredi avant la clôture
        _ => false,             // samedi
    }
}

/// Décision pure : un état par asset, anomalies = barres hors contrat.
pub fn auditer(
    assets: &[(String, String)],
    dernieres: &HashMap<(String, String), i64>,
    maintenant: i64,
    ouvert_mt5: bool,
) -> Vec<EtatAsset> {
    assets
        .iter()
        .map(|(id, source)| {
            let crypto = source == "binance";
            let ouvert = crypto || ouvert_mt5;
            let mut anomalies = Vec::new();
            if ouvert {
                for tf in TFS {
                    let Some(seuil) = seuil_tf(tf, crypto) else { continue };
                    match dernieres.get(&(id.clone(), tf.to_string())) {
                        Some(ts) => {
                            let retard = (maintenant - ts) / 60;
                            if retard > seuil {
                                anomalies.push(Anomalie {
                                    tf: tf.to_string(),
                                    retard_min: retard,
                                    seuil_min: seuil,
                                });
                            }
                        }
                        None => anomalies.push(Anomalie {
                            tf: tf.to_string(),
                            retard_min: -1,
                            seuil_min: seuil,
                        }),
                    }
                }
            }
            EtatAsset {
                asset: id.clone(),
                source: source.clone(),
                marche_ouvert: ouvert,
                anomalies,
            }
        })
        .collect()
}

/// Un cycle complet : lit les maxima (seek d'index par asset × TF — jamais
/// de scan global de la table), audite, journalise les TRANSITIONS
/// seulement, publie l'état.
pub async fn auditer_db(db: &Arc<db::Database>) {
    let Ok(rows) = sqlx::query("SELECT id, source FROM assets WHERE actif = 1 ORDER BY id")
        .fetch_all(db.pool())
        .await
    else {
        return;
    };
    let assets: Vec<(String, String)> = rows
        .iter()
        .filter_map(|r| {
            let id: String = r.try_get("id").unwrap_or_default();
            let source: String = r.try_get("source").unwrap_or_default();
            if id.is_empty() { None } else { Some((id, source)) }
        })
        .collect();
    if assets.is_empty() {
        return;
    }

    let mut dernieres = HashMap::new();
    for (id, _) in &assets {
        for tf in TFS {
            let Ok(r) = sqlx::query(
                "SELECT MAX(timestamp) AS m FROM bougies WHERE asset = ? AND timeframe = ?",
            )
            .bind(id)
            .bind(tf)
            .fetch_one(db.pool())
            .await
            else {
                continue;
            };
            if let Some(m) = r.try_get::<Option<i64>, _>("m").ok().flatten() {
                dernieres.insert((id.clone(), tf.to_string()), m);
            }
        }
    }

    let maintenant = Utc::now().timestamp();
    let instant_paris = Paris.from_utc_datetime(&Utc::now().naive_utc());
    let etats = auditer(&assets, &dernieres, maintenant, marche_ouvert_mt5(instant_paris));

    // Journal des transitions uniquement (pas de spam cyclique). Les
    // émissions mobile partent APRÈS le verrou (un guard n'est pas Send).
    let mut nouvelles: Vec<(String, String, i64, i64, bool)> = Vec::new();
    let precedentes = PRECEDENTES.get_or_init(|| RwLock::new(HashMap::new()));
    if let Ok(mut prec) = precedentes.write() {
        for etat in &etats {
            let avant = prec.get(&etat.asset).cloned().unwrap_or_default();
            if etat.anomalies != avant {
                for a in &etat.anomalies {
                    if !avant.contains(a) {
                        if a.retard_min < 0 {
                            tracing::warn!(
                                "🩸 Fraîcheur : {} — {} sans AUCUNE barre en base (pipeline actif)",
                                etat.asset,
                                a.tf
                            );
                        } else {
                            tracing::warn!(
                                "🩸 Fraîcheur : {} {} en retard de {} min (seuil {} min) — marché {}",
                                etat.asset,
                                a.tf,
                                a.retard_min,
                                a.seuil_min,
                                if etat.marche_ouvert { "ouvert" } else { "fermé" }
                            );
                        }
                        nouvelles.push((
                            etat.asset.clone(),
                            a.tf.clone(),
                            a.retard_min,
                            a.seuil_min,
                            etat.marche_ouvert,
                        ));
                    }
                }
                for a in &avant {
                    if !etat.anomalies.contains(a) {
                        tracing::info!("💚 Fraîcheur : {} {} redevenu frais", etat.asset, a.tf);
                    }
                }
            }
            prec.insert(etat.asset.clone(), etat.anomalies.clone());
        }
    }
    // Relais mobile (MOB ⑦) — problème app ↔ fournisseur.
    for (asset, tf, retard, seuil, marche) in nouvelles {
        crate::mobile_relay::emettre(
            db,
            "sante",
            &format!("{asset} {tf} — retard {}", duree_lisible(retard)),
            Some(&asset),
            Some(&tf),
            serde_json::json!({
                "asset": asset, "tf": tf, "retard_min": retard,
                "seuil_min": seuil, "marche_ouvert": marche,
            }),
            false,
        )
        .await;
    }

    let audit = Audit { ts: maintenant, etats };
    if let Ok(mut a) = AUDIT
        .get_or_init(|| RwLock::new(Audit::default()))
        .write()
    {
        *a = audit;
    }
}

/// GET /api/assets/fraicheur — l'état publié du dernier cycle (≤ 60 s).
pub async fn get_fraicheur() -> impl actix_web::Responder {
    HttpResponse::Ok().json(audit_courant())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn carte(entries: &[(&str, &str, i64)]) -> HashMap<(String, String), i64> {
        entries
            .iter()
            .map(|(a, t, ts)| ((a.to_string(), t.to_string()), *ts))
            .collect()
    }

    fn paris(jour: chrono::Weekday, h: u32) -> chrono::DateTime<chrono_tz::Tz> {
        use chrono::TimeZone;
        // 16/10/2026 = vendredi — base commune des tests d'horaire.
        let vendredi = Paris.with_ymd_and_hms(2026, 10, 16, 0, 0, 0).unwrap();
        let ecart = (jour.num_days_from_monday() as i64
            - chrono::Weekday::Fri.num_days_from_monday() as i64)
            .rem_euclid(7);
        vendredi + chrono::Duration::days(ecart) + chrono::Duration::hours(h as i64)
    }

    #[test]
    fn crypto_m1_frais_aucune_anomalie() {
        let maintenant = 1_000_000i64;
        // Les 7 TF présents et dans leur contrat (M1 5 min, D1 24 h…).
        let carte_complete = carte(&[
            ("BTC", "M1", maintenant - 5 * 60),
            ("BTC", "M5", maintenant - 10 * 60),
            ("BTC", "M15", maintenant - 20 * 60),
            ("BTC", "M30", maintenant - 40 * 60),
            ("BTC", "H1", maintenant - 70 * 60),
            ("BTC", "H4", maintenant - 200 * 60),
            ("BTC", "D1", maintenant - 24 * 3600),
        ]);
        let etats = auditer(
            &[("BTC".into(), "binance".into())],
            &carte_complete,
            maintenant,
            false, // week-end mt5 — la crypto trade quand même
        );
        assert_eq!(etats[0].anomalies, vec![]);
        assert!(etats[0].marche_ouvert);
    }

    #[test]
    fn crypto_m1_mort_alerte() {
        let maintenant = 1_000_000i64;
        let etats = auditer(
            &[("BTC".into(), "binance".into())],
            &carte(&[("BTC", "M1", maintenant - 1_200)]), // 20 min
            maintenant,
            true,
        );
        assert_eq!(etats[0].anomalies.len(), 7); // M1 en retard + les 6 TF absents
        assert_eq!(etats[0].anomalies[0].tf, "M1");
        assert_eq!(etats[0].anomalies[0].retard_min, 20);
    }

    #[test]
    fn mt5_pause_quotidienne_absorbee_puis_alerte() {
        let maintenant = 1_000_000i64;
        let base = [("XAUUSD", "M1", maintenant - 3_600)]; // 60 min : pause OK
        let etats = auditer(&[("XAUUSD".into(), "mt5".into())], &carte(&base), maintenant, true);
        assert!(etats[0].anomalies.iter().all(|a| a.tf != "M1"));

        let base = [("XAUUSD", "M1", maintenant - 7_200)]; // 120 min : mort
        let etats = auditer(&[("XAUUSD".into(), "mt5".into())], &carte(&base), maintenant, true);
        assert!(etats[0].anomalies.iter().any(|a| a.tf == "M1" && a.retard_min == 120));
    }

    #[test]
    fn mt5_weekend_aucune_alerte_meme_mort() {
        let maintenant = 1_000_000i64;
        let etats = auditer(
            &[("EURUSD".into(), "mt5".into())],
            &carte(&[("EURUSD", "M1", maintenant - 3 * 86_400)]),
            maintenant,
            false,
        );
        assert_eq!(etats[0].anomalies, vec![]);
        assert!(!etats[0].marche_ouvert);
    }

    #[test]
    fn mt5_d1_figee_le_weekend_est_saine_en_semaine() {
        let maintenant = 1_000_000i64;
        // 40 h de retard D1 en semaine : sous le seuil 50 h (figée + push).
        let etats = auditer(
            &[("XAUUSD".into(), "mt5".into())],
            &carte(&[("XAUUSD", "D1", maintenant - 40 * 3600)]),
            maintenant,
            true,
        );
        assert!(etats[0].anomalies.iter().all(|a| a.tf != "D1"));
    }

    #[test]
    fn h4_cloturee_saine_ne_alerte_pas() {
        // 16/10, cas réel : les H4 Binance n'existent qu'à la clôture — la
        // dernière fermée a ouvert il y a 6 h 06 : SAIN (seuil 9 h). Ma
        // première calibration (4 h 30) produisait 8 fausses alertes.
        let maintenant = 1_000_000i64;
        for (asset, source, ouvert) in [("BTC", "binance", true), ("XAUUSD", "mt5", true)] {
            let etats = auditer(
                &[((asset).into(), source.into())],
                &carte(&[(asset, "H4", maintenant - 366 * 60)]),
                maintenant,
                ouvert,
            );
            assert!(etats[0].anomalies.iter().all(|a| a.tf != "H4"), "{asset}");
        }
    }

    #[test]
    fn d1_cloturee_crypto_38h_saine_morte_55h_alerte() {
        let maintenant = 1_000_000i64;
        let saine = auditer(
            &[("BTC".into(), "binance".into())],
            &carte(&[("BTC", "D1", maintenant - 38 * 3600)]),
            maintenant,
            true,
        );
        assert!(saine[0].anomalies.iter().all(|a| a.tf != "D1"));
        let morte = auditer(
            &[("BTC".into(), "binance".into())],
            &carte(&[("BTC", "D1", maintenant - 55 * 3600)]),
            maintenant,
            true,
        );
        assert!(morte[0].anomalies.iter().any(|a| a.tf == "D1"));
    }

    #[test]
    fn tf_absent_anomalie_moins_un() {
        let maintenant = 1_000_000i64;
        let etats = auditer(
            &[("XPTUSD".into(), "mt5".into())],
            &carte(&[("XPTUSD", "M1", maintenant - 60)]),
            maintenant,
            true,
        );
        assert!(etats[0]
            .anomalies
            .iter()
            .all(|a| a.retard_min == -1 || a.tf == "M1"));
    }

    #[test]
    fn semaine_de_cotation_mt5() {
        assert!(marche_ouvert_mt5(paris(chrono::Weekday::Fri, 22)));
        assert!(!marche_ouvert_mt5(paris(chrono::Weekday::Fri, 23)));
        assert!(!marche_ouvert_mt5(paris(chrono::Weekday::Sat, 12)));
        assert!(!marche_ouvert_mt5(paris(chrono::Weekday::Sun, 22)));
        assert!(marche_ouvert_mt5(paris(chrono::Weekday::Sun, 23)));
        assert!(marche_ouvert_mt5(paris(chrono::Weekday::Mon, 0)));
    }

    #[test]
    fn duree_lisible_passe_en_heures() {
        assert_eq!(duree_lisible(45), "45 min");
        assert_eq!(duree_lisible(540), "9h");
        assert_eq!(duree_lisible(620), "10h20");
    }
}
