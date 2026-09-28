//! Labo whale watching (2.2) : comparatif avec/sans volume anormal.
//!
//! Pour chaque signal SMC clôturé, recalcule le z-score volume de la
//! bougie au moment de l'émission (rétrospectif). Compare les deux
//! cohortes : z > seuil (whale) vs z ≤ seuil (normal). Le calcul se fait
//! en Rust (SQLite n'a pas SQRT natif) — on fetch les volumes puis on
//! groupe.

use actix_web::{web, HttpResponse};
use sqlx::Row;

use crate::state::AppState;

#[derive(serde::Serialize, Clone)]
pub struct CohorteWhale {
    pub label: String,
    pub n: usize,
    pub wr_pct: f64,
    pub r_total: f64,
    pub r_moyen: f64,
    pub sl_pct: f64,
}

#[derive(serde::Serialize)]
pub struct LaboWhale {
    pub avec_whale: CohorteWhale,
    pub sans_whale: CohorteWhale,
    pub seuil_sigma: f64,
}

#[derive(serde::Deserialize)]
pub struct WhaleLaboQuery {
    pub seuil: Option<f64>,
}

/// GET /api/analyses/whale-labo?seuil=2.0
pub async fn get_whale_labo(
    state: web::Data<AppState>,
    query: web::Query<WhaleLaboQuery>,
) -> HttpResponse {
    let seuil = query.seuil.unwrap_or(2.0).max(0.5);

    // 1. Récupérer les signaux SMC clôturés (non expirés)
    let signaux = match sqlx::query(
        "SELECT id, asset, timeframe, verdict, r_realise, cree_le
         FROM signaux
         WHERE strategie = 'SMC' AND verdict IS NOT NULL AND verdict != 'Expire'",
    )
    .fetch_all(state.db.pool())
    .await
    {
        Ok(rows) => rows,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({"erreur": e.to_string()}));
        }
    };

    // 2. Pour chaque signal, fetch les 60 dernières bougies avant l'émission
    //    et calculer le z-score en Rust.
    let mut whale = CohorteWhale {
        label: "Avec 🐋 (z > seuil)".into(),
        n: 0, wr_pct: 0.0, r_total: 0.0, r_moyen: 0.0, sl_pct: 0.0,
    };
    let mut normal = CohorteWhale {
        label: "Sans 🐋 (volume normal)".into(),
        n: 0, wr_pct: 0.0, r_total: 0.0, r_moyen: 0.0, sl_pct: 0.0,
    };

    // Simple compteurs
    let mut w_gagnants = 0i64; let mut w_sl = 0i64; let mut w_r = 0.0;
    let mut n_gagnants = 0i64; let mut n_sl = 0i64; let mut n_r = 0.0;

    for sig in &signaux {
        let asset: String = sig.try_get("asset").unwrap_or_default();
        let tf: String = sig.try_get("timeframe").unwrap_or_default();
        let verdict: String = sig.try_get("verdict").unwrap_or_default();
        let r_realise: f64 = sig.try_get("r_realise").unwrap_or(0.0);
        let cree_le: i64 = sig.try_get("cree_le").unwrap_or(0);

        // Fetch les 61 dernières bougies (60 de référence + 1 = le signal)
        let bougies = match sqlx::query(
            "SELECT volume FROM bougies
             WHERE asset = ? AND timeframe = ? AND timestamp <= ?
             ORDER BY timestamp DESC LIMIT 61",
        )
        .bind(&asset)
        .bind(&tf)
        .bind(cree_le)
        .fetch_all(state.db.pool())
        .await
        {
            Ok(b) if b.len() >= 20 => b,
            _ => continue, // pas assez de données
        };

        let volumes: Vec<f64> = bougies
            .iter()
            .filter_map(|r| r.try_get::<f64, _>("volume").ok())
            .filter(|v| *v > 0.0)
            .collect();
        if volumes.len() < 20 {
            continue;
        }

        // La 1ère bougie (DESC) est celle du signal, les 60 suivantes = référence
        let vol_signal = volumes[0];
        let refs = &volumes[1..];
        let n = refs.len() as f64;
        let moy = refs.iter().sum::<f64>() / n;
        let var = refs.iter().map(|v| (v - moy) * (v - moy)).sum::<f64>() / n;

        if moy <= 0.0 || var <= 0.0 {
            continue;
        }

        // z-score sans SQRT : comparer les carrés
        // z > seuil ⟺ (vol - moy)² > seuil² × var (pour z > 0)
        let ecart = vol_signal - moy;
        let est_whale = ecart > 0.0 && ecart * ecart > seuil * seuil * var;

        let est_gagnant = verdict.starts_with("TP");
        let est_sl = verdict == "SL";

        if est_whale {
            whale.n += 1;
            if est_gagnant { w_gagnants += 1; }
            if est_sl { w_sl += 1; }
            w_r += r_realise;
        } else {
            normal.n += 1;
            if est_gagnant { n_gagnants += 1; }
            if est_sl { n_sl += 1; }
            n_r += r_realise;
        }
    }

    // Finaliser les pourcentages
    if whale.n > 0 {
        whale.wr_pct = w_gagnants as f64 / whale.n as f64 * 100.0;
        whale.sl_pct = w_sl as f64 / whale.n as f64 * 100.0;
        whale.r_total = w_r;
        whale.r_moyen = w_r / whale.n as f64;
    }
    if normal.n > 0 {
        normal.wr_pct = n_gagnants as f64 / normal.n as f64 * 100.0;
        normal.sl_pct = n_sl as f64 / normal.n as f64 * 100.0;
        normal.r_total = n_r;
        normal.r_moyen = n_r / normal.n as f64;
    }

    HttpResponse::Ok().json(LaboWhale {
        avec_whale: whale,
        sans_whale: normal,
        seuil_sigma: seuil,
    })
}
