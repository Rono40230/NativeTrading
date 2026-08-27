//! Créneaux de volatilité — job de calcul des heures Paris les plus
//! actives par asset (dashboard « ⏰ Créneaux de volatilité »).
//!
//! Statistique pure sur l'historique M15 (24 mois glissants), sans LLM :
//! par (asset × heure Paris) — range moyen en % du prix (comparable entre
//! actifs) et fiabilité (part des JOURS PRÉSENTS où le créneau a livré au
//! moins une barre notable ≥ 1,5× la médiane de l'asset). Rafraîchi au
//! boot puis chaque 24h — la fenêtre glissante intègre la journée
//! écoulée. Itère la table assets ACTIVE : tout nouvel asset surveillé
//! apparaît au cycle suivant.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;

use db::Database;

/// Lance le job (boot + cycle quotidien).
pub fn demarrer(db: Arc<Database>) {
    tokio::spawn(async move {
        calculer(&db).await;
        let mut tick = tokio::time::interval(std::time::Duration::from_secs(24 * 3600));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tick.tick().await;
            calculer(&db).await;
        }
    });
}

async fn calculer(db: &Database) {
    let debut = std::time::Instant::now();
    let assets = match db.lister_assets_worker().await {
        Ok(a) => a.into_iter().filter(|a| a.actif).map(|a| a.id).collect::<Vec<_>>(),
        Err(e) => {
            tracing::warn!("Créneaux volatilité (assets): {}", e);
            return;
        }
    };
    let mut nb = 0usize;
    for asset in &assets {
        if calculer_asset(db, asset).await.is_ok() {
            nb += 1;
        }
    }
    tracing::info!("⏰ Créneaux volatilité : {} asset(s) en {:?}", nb, debut.elapsed());
}

async fn calculer_asset(db: &Database, asset: &str) -> anyhow::Result<()> {
    let bougies = db
        .obtenir_bougies(&common::Asset::from(asset), &common::Timeframe::M15, 70_000)
        .await
        .unwrap_or_default();
    if bougies.len() < 500 {
        return Ok(()); // historique insuffisant — traité aux prochains cycles
    }

    // Médiane des ranges % de l'asset → seuil « barre notable ».
    let mut ranges: Vec<f64> = bougies
        .iter()
        .filter(|b| b.close > 0.0)
        .map(|b| (b.high - b.low) / b.close)
        .collect();
    ranges.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let seuil_notable = 1.5 * ranges[ranges.len() / 2].max(1e-12);

    // Par heure Paris : (somme %, n barres, jours présents, jours notables).
    let mut par_heure: HashMap<u32, (f64, u64, HashSet<i64>, HashSet<i64>)> = HashMap::new();
    for b in &bougies {
        if b.close <= 0.0 {
            continue;
        }
        let paris = chrono::DateTime::from_timestamp(b.timestamp.timestamp(), 0)
            .unwrap_or_default()
            .with_timezone(&chrono_tz::Europe::Paris);
        use chrono::{Datelike as _, Timelike as _};
        let cle_jour = paris.year() as i64 * 1000 + paris.ordinal() as i64;
        let e = par_heure.entry(paris.hour()).or_default();
        let range_pct = (b.high - b.low) / b.close * 100.0;
        e.0 += range_pct;
        e.1 += 1;
        e.2.insert(cle_jour);
        if range_pct >= seuil_notable {
            e.3.insert(cle_jour);
        }
    }

    let maintenant = chrono::Utc::now().timestamp();
    for (heure, (somme, n, jours, jours_notables)) in par_heure {
        if n == 0 || jours.is_empty() {
            continue;
        }
        let _ = sqlx::query(
            "INSERT OR REPLACE INTO creneaux_volatilite (asset, heure, vol_pct, fiabilite, nb_jours, maj_le)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(asset)
        .bind(heure as i64)
        .bind(somme / n as f64)
        .bind(jours_notables.len() as f64 / jours.len() as f64)
        .bind(jours.len() as i64)
        .bind(maintenant)
        .execute(db.pool())
        .await?;
    }
    // Heures devenues vides (marché fermé / données retirées) : purgées.
    let _ = sqlx::query("DELETE FROM creneaux_volatilite WHERE asset = ? AND maj_le < ?")
        .bind(asset)
        .bind(maintenant)
        .execute(db.pool())
        .await;
    Ok(())
}

// ── Endpoint HTTP ─────────────────────────────────────────────────────────────

/// GET /api/creneaux-volatilite — top 3 créneaux par asset actif, triés
/// par score (vol × fiabilité).
pub async fn lister(state: actix_web::web::Data<crate::state::AppState>) -> impl actix_web::Responder {
    let rows = match sqlx::query(
        "SELECT c.asset, c.heure, c.vol_pct, c.fiabilite
         FROM creneaux_volatilite c
         JOIN assets a ON a.id = c.asset AND a.actif = 1
         ORDER BY c.asset, c.vol_pct * c.fiabilite DESC",
    )
    .fetch_all(state.db.pool())
    .await
    {
        Ok(r) => r,
        Err(e) => return actix_web::HttpResponse::InternalServerError().body(format!("{e}")),
    };
    use sqlx::Row as _;
    let mut par_asset: BTreeMap<String, Vec<serde_json::Value>> = BTreeMap::new();
    for r in rows {
        let asset: String = r.get("asset");
        par_asset.entry(asset).or_default().push(serde_json::json!({
            "heure": r.get::<i64, _>("heure"),
            "vol_pct": r.get::<f64, _>("vol_pct"),
            "fiabilite": r.get::<f64, _>("fiabilite"),
        }));
    }
    let sortie: Vec<serde_json::Value> = par_asset
        .into_iter()
        .map(|(asset, creneaux)| {
            serde_json::json!({ "asset": asset, "top": creneaux.into_iter().take(3).collect::<Vec<_>>() })
        })
        .collect();
    actix_web::HttpResponse::Ok().json(sortie)
}
