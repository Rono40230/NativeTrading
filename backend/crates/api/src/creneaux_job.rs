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

/// Fusionne les heures consécutives en plages horaires continues.
/// ÉTAPE 1 : ne garder que les heures SIGNIFICATIVES (vol ≥ 1,2× la médiane
/// de l'asset — sinon les 24h se fusionnent en une plage 00h→24h).
/// ÉTAPE 2 : fusionner les consécutives.
/// Retourne les 3 meilleures plages par score (vol × fiabilité × durée).
fn fusionner_plages(creneaux: &[serde_json::Value]) -> Vec<serde_json::Value> {
    // Extraire et trier par vol descendante.
    let mut toutes: Vec<(i64, f64, f64)> = creneaux
        .iter()
        .filter_map(|c| {
            let h = c.get("heure")?.as_i64()?;
            let v = c.get("vol_pct")?.as_f64()?;
            let f = c.get("fiabilite")?.as_f64()?;
            Some((h, v, f))
        })
        .collect();
    toutes.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    // Médiane des vols → seuil « significatif » = 1,2× médiane.
    let mut vols: Vec<f64> = toutes.iter().map(|&(_, v, _)| v).collect();
    vols.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let seuil = vols.get(vols.len() / 2).copied().unwrap_or(0.0) * 1.2;

    // Garder les heures au-dessus du seuil (max 8 pour former des clusters).
    let mut heures: Vec<(i64, f64, f64)> = toutes
        .into_iter()
        .filter(|&(_, v, _)| v >= seuil)
        .take(8)
        .collect();
    heures.sort_by_key(|&(h, _, _)| h);

    // Grouper les consécutives.
    let mut plages: Vec<(i64, i64, f64, f64, usize)> = Vec::new(); // (debut, fin, vol, fiab, nb)
    for &(h, v, f) in &heures {
        match plages.last_mut() {
            Some(p) if p.1 + 1 == h => {
                p.1 = h;
                p.2 += v;
                p.3 += f;
                p.4 += 1;
            }
            _ => plages.push((h, h, v, f, 1)),
        }
    }

    // Score = vol moyenne × fiabilité moyenne × heures couvertes.
    let mut triees: Vec<_> = plages
        .into_iter()
        .map(|(debut, fin, sv, sf, n)| {
            let vol = sv / n as f64;
            let fiab = sf / n as f64;
            let score = vol * fiab * n as f64;
            (debut, fin, vol, fiab, n, score)
        })
        .collect();
    triees.sort_by(|a, b| b.5.partial_cmp(&a.5).unwrap_or(std::cmp::Ordering::Equal));

    triees
        .into_iter()
        .take(3)
        .map(|(debut, fin, vol, fiab, n, _)| {
            serde_json::json!({
                "debut": debut,
                "fin": fin + 1, // exclusive : 15h→18h = heures 15,16,17
                "vol_pct": vol,
                "fiabilite": fiab,
                "nb_heures": n,
            })
        })
        .collect()
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
    // Fusion des heures consécutives en PLAGES (15h+16h+17h → 15h→18h)
    // puis top 3 plages par score combiné.
    let sortie: Vec<serde_json::Value> = par_asset
        .into_iter()
        .map(|(asset, creneaux)| {
            let plages = fusionner_plages(&creneaux);
            serde_json::json!({ "asset": asset, "top": plages })
        })
        .collect();
    actix_web::HttpResponse::Ok().json(sortie)
}
