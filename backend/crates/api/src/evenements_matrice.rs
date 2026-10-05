//! Matrice de réactivité événementielle — extraite d'evenements.rs
//! (limite 600 lignes, même convention que creneaux_ia → creneaux_perimetre).
//! ATR M1 des 3 premières minutes de chaque occurrence vs habitude de
//! l'asset, regroupé par heure LOCALE de l'événement (DST-correct).
//! Préchauffée en tâche de fond au boot (étape 1-bis, incident 05/10).

use std::collections::HashMap;
use std::sync::OnceLock;

use actix_web::{web, HttpResponse, Responder};
use chrono::{Timelike, Utc};
use sqlx::Row;
use tokio::sync::RwLock;

use crate::state::AppState;
use crate::evenements::{EVENEMENTS, fenetres_evenement, prochaine_occurrence, FENETRE_MINUTES, PERIODE_JOURS, MIN_MINUTES, MIN_BOUGIES};
use db::Database;

// ── Matrice de réactivité ────────────────────────────────────────────────────

/// Une ligne (événement × asset) : l'ATR des 3 premières minutes de
/// l'événement rapporté à l'habitude M1 de l'asset sur la période.
fn ligne_reactivite(asset: &str, somme: f64, minutes: i64, habitude: f64) -> serde_json::Value {
    let atr = somme / minutes as f64;
    serde_json::json!({
        "asset": asset,
        "ratio": (atr / habitude * 100.0).round() / 100.0,
        "atr": (atr * 10_000.0).round() / 10_000.0,
        "habitude": (habitude * 10_000.0).round() / 10_000.0,
        "minutes": minutes,
    })
}

/// Corps du calcul (séparé du handler pour être testable sans DB) :
/// `bougies` = (timestamp, high, low) M1 dédoublonnées d'un asset.
fn matrice_asset(
    asset: &str,
    bougies: &[(i64, f64, f64)],
    masques: &HashMap<i64, u16>,
    nb_evenements: usize,
) -> Vec<(usize, serde_json::Value)> {
    if bougies.len() < MIN_BOUGIES {
        return Vec::new();
    }
    let habitude = bougies.iter().map(|b| b.1 - b.2).sum::<f64>() / bougies.len() as f64;
    if habitude <= 0.0 {
        return Vec::new();
    }
    let mut sommes = vec![0.0f64; nb_evenements];
    let mut minutes = vec![0i64; nb_evenements];
    for &(ts, high, low) in bougies {
        if let Some(masque) = masques.get(&ts) {
            for i in 0..nb_evenements {
                if masque & (1 << i) != 0 {
                    sommes[i] += high - low;
                    minutes[i] += 1;
                }
            }
        }
    }
    (0..nb_evenements)
        .filter(|&i| minutes[i] >= MIN_MINUTES)
        .map(|i| (i, ligne_reactivite(asset, sommes[i], minutes[i], habitude)))
        .collect()
}

/// GET /api/evenements/matrice — taxonomie + réactivité par asset.
/// Le scan parcourt l'historique M1 de chaque asset actif : réponse mise en
/// cache 1 h (même politique que les patterns horaires).
/// Cache de la matrice (1 h) — au niveau module pour que le préchauffage
/// de démarrage (étape 1-bis, incident 05/10) remplisse le MÊME cache que
/// l'endpoint : à l'ouverture de la fenêtre, calcul déjà fait.
static CACHE_MATRICE: OnceLock<RwLock<Option<(std::time::Instant, serde_json::Value)>>> = OnceLock::new();

/// Préchauffe la matrice en tâche de fond au boot (étape 1-bis) : le calcul
/// à froid scannait des dizaines de millions de bougies PENDANT que le
/// dashboard chargeait — tous les fetchs patientaient derrière (incident
/// 05/10 « tout à 0 »). Retourne la valeur calculée (et remplit le cache).
pub async fn prechauffer_matrice(db: &std::sync::Arc<Database>) -> serde_json::Value {
    let cache = CACHE_MATRICE.get_or_init(|| RwLock::new(None));
    let valeur = calculer_matrice(db).await;
    *cache.write().await = Some((std::time::Instant::now(), valeur.clone()));
    valeur
}

/// GET /api/evenements/matrice — taxonomie + réactivité par asset.
/// Le scan parcourt l'historique M1 de chaque asset actif : réponse mise en
/// cache 1 h (même politique que les patterns horaires).
pub async fn get_matrice(state: web::Data<AppState>) -> impl Responder {
    let cache = CACHE_MATRICE.get_or_init(|| RwLock::new(None));

    if let Some((calcule_le, valeur)) = cache.read().await.clone() {
        if calcule_le.elapsed() < std::time::Duration::from_secs(3600) {
            return HttpResponse::Ok().json(valeur);
        }
    }

    let valeur = calculer_matrice(&state.db).await;
    *cache.write().await = Some((std::time::Instant::now(), valeur.clone()));
    HttpResponse::Ok().json(valeur)
}

async fn calculer_matrice(db: &std::sync::Arc<Database>) -> serde_json::Value {
    let maintenant = Utc::now().timestamp();
    let debut = maintenant - PERIODE_JOURS * 86_400;

    // Masque timestamp → événements (plusieurs événements peuvent coïncider,
    // ex. annonces US 10:00 et fix PM Londres en heure Paris d'été).
    let mut masques: HashMap<i64, u16> = HashMap::new();
    for (i, ev) in EVENEMENTS.iter().enumerate() {
        for ts in fenetres_evenement(ev, debut, maintenant) {
            *masques.entry(ts).or_insert(0) |= 1 << i;
        }
    }

    // Lignes de réactivité par événement, assemblées par asset.
    let mut par_evenement: Vec<Vec<serde_json::Value>> = EVENEMENTS
        .iter()
        .map(|_| Vec::new())
        .collect();
    if let Ok(workers) = db.lister_assets_worker().await {
        for w in workers.into_iter().filter(|w| w.actif) {
            // Priorité à la source vivante du pipeline ; dédoublonnage par
            // timestamp (binance et bybit se recouvrent sur le BTC).
            let rows = sqlx::query(
                "SELECT timestamp, high, low FROM bougies
                 WHERE asset = ? AND timeframe = 'M1' AND timestamp >= ?
                 ORDER BY timestamp ASC,
                     CASE source WHEN 'bybit_ws' THEN 0 WHEN 'mt5' THEN 1 ELSE 2 END",
            )
            .bind(&w.id)
            .bind(debut)
            .fetch_all(db.pool())
            .await
            .unwrap_or_default();
            let mut bougies: Vec<(i64, f64, f64)> = Vec::with_capacity(rows.len());
            let mut dernier_ts = 0i64;
            for r in &rows {
                let ts: i64 = r.try_get("timestamp").unwrap_or(0);
                let high: f64 = r.try_get("high").unwrap_or(0.0);
                let low: f64 = r.try_get("low").unwrap_or(0.0);
                if ts > dernier_ts && high >= low {
                    bougies.push((ts, high, low));
                    dernier_ts = ts;
                }
            }
            for (i, ligne) in matrice_asset(&w.id, &bougies, &masques, EVENEMENTS.len()) {
                par_evenement[i].push(ligne);
            }
        }
    }

    // Assemblage final : chaque événement avec sa prochaine occurrence,
    // convertie en heure de Paris (la bascule DST du jour s'applique).
    let maintenant_utc = Utc::now();
    let mut evenements: Vec<serde_json::Value> = EVENEMENTS
        .iter()
        .enumerate()
        .map(|(i, ev)| {
            let mut reactivite = std::mem::take(&mut par_evenement[i]);
            reactivite.sort_by(|a, b| {
                b["ratio"].as_f64().unwrap_or(0.0).partial_cmp(&a["ratio"].as_f64().unwrap_or(0.0)).unwrap_or(std::cmp::Ordering::Equal)
            });
            let max_ratio = reactivite
                .first()
                .and_then(|l| l["ratio"].as_f64())
                .unwrap_or(0.0);
            let prochaine = prochaine_occurrence(ev, maintenant_utc);
            let paris = prochaine.map(|p| p.with_timezone(&chrono_tz::Europe::Paris));
            serde_json::json!({
                "ident": ev.ident,
                "nom": ev.nom,
                "detail": ev.detail,
                "fuseau": ev.tz.name(),
                "heure_locale": format!("{:02}:{:02}", ev.heure, ev.minute),
                "jours": ev.jours,
                "prochaine_ts": prochaine.map(|p| p.timestamp()),
                "prochaine_heure_paris": paris.map(|p| format!("{:02}:{:02}", p.hour(), p.minute())),
                "max_ratio": (max_ratio * 100.0).round() / 100.0,
                "reactivite": reactivite,
            })
        })
        .collect();
    evenements.sort_by(|a, b| {
        b["max_ratio"].as_f64().unwrap_or(0.0).partial_cmp(&a["max_ratio"].as_f64().unwrap_or(0.0)).unwrap_or(std::cmp::Ordering::Equal)
    });

    serde_json::json!({
        "calcule_le": maintenant,
        "periode_jours": PERIODE_JOURS,
        "fenetre_minutes": FENETRE_MINUTES,
        "evenements": evenements,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

/// La matrice mesure bien le ratio : 30 minutes d'événement au double
    /// de l'habitude ressortent autour de ×2 ; en dessous de 20 minutes
    /// observées, la ligne n'est pas publiée (garde-fou anti-bruit).
    #[test]
    fn matrice_mesure_le_ratio_et_filtre_le_bruit() {
        let evenement_ts = chrono_tz::Europe::Paris
            .with_ymd_and_hms(2026, 9, 14, 16, 0, 0)
            .unwrap()
            .timestamp();
        let mut masques = HashMap::new();
        for k in 0..30 {
            masques.insert(evenement_ts + k * 60, 1u16);
        }
        // Habitude : 20 000 bougies de range 1.0, 30 bougies d'événement de
        // range 2.0 (contamination de l'habitude négligeable : ~×1.99).
        let mut bougies: Vec<(i64, f64, f64)> = (0..20_000)
            .map(|i| (1_700_000_000 + i * 60, 1.0, 0.0))
            .collect();
        for k in 0..30 {
            bougies.push((evenement_ts + k * 60, 2.0, 0.0));
        }
        let lignes = matrice_asset("TEST", &bougies, &masques, 1);
        let (_, premiere) = &lignes[0];
        let ratio = premiere["ratio"].as_f64().expect("ratio");
        assert!((1.9..2.05).contains(&ratio), "ratio attendu ~2, obtenu {ratio}");
        assert_eq!(premiere["minutes"].as_i64(), Some(30));

        // Même scénario avec 3 minutes d'événement seulement : sous le
        // seuil MIN_MINUTES, la ligne n'est pas publiée.
        let mut maigres: Vec<(i64, f64, f64)> = (0..20_000)
            .map(|i| (1_700_000_000 + i * 60, 1.0, 0.0))
            .collect();
        for k in 0..3 {
            maigres.push((evenement_ts + k * 60, 2.0, 0.0));
        }
        assert!(matrice_asset("TEST", &maigres, &masques, 1).is_empty());
    }
}
