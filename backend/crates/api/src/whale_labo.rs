//! Labo whale watching (2.2) : comparatif avec/sans volume anormal.
//!
//! Pour chaque signal SMC clôturé, recalcule le z-score volume de la
//! bougie au moment de l'émission (rétrospectif — pas besoin d'un champ
//! en base). Compare les deux cohortes : z > 2σ (whale) vs z ≤ 2σ (normal).
//! Sert à décider si le bonus volume améliore réellement la sélection.

use actix_web::{web, HttpResponse};
use sqlx::Row;

use crate::state::AppState;

#[derive(serde::Serialize)]
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

/// GET /api/analyses/whale-labo?seuil=2.0
/// Compare les signaux SMC clôturés avec/sans volume anormal (>seuil σ).
pub async fn get_whale_labo(
    state: web::Data<AppState>,
    query: web::Query<WhaleLaboQuery>,
) -> HttpResponse {
    let seuil = query.seuil.unwrap_or(2.0);

    // Récupérer tous les signaux SMC clôturés avec leur z-score volume
    // calculé rétrospectivement.
    let rows = match sqlx::query(
        r#"
        WITH signaux_fermes AS (
            SELECT id, asset, timeframe, verdict, r_realise, cree_le
            FROM signaux
            WHERE strategie = 'SMC' AND verdict IS NOT NULL
              AND verdict != 'Expire'
        ),
        avec_zscore AS (
            SELECT
                sf.id, sf.asset, sf.timeframe, sf.verdict, sf.r_realise,
                sf.verdict LIKE 'TP%' as est_gagnant,
                sf.verdict = 'SL' as est_sl,
                (
                    SELECT b.volume
                    FROM bougies b
                    WHERE b.asset = sf.asset AND b.timeframe = sf.timeframe
                      AND b.timestamp <= sf.cree_le
                    ORDER BY b.timestamp DESC LIMIT 1
                ) as vol_signal,
                (
                    SELECT AVG(b2.volume)
                    FROM (
                        SELECT b3.volume FROM bougies b3
                        WHERE b3.asset = sf.asset AND b3.timeframe = sf.timeframe
                          AND b3.timestamp <= sf.cree_le
                        ORDER BY b3.timestamp DESC LIMIT 60 OFFSET 1
                    ) b2
                ) as vol_moyen,
                (
                    SELECT SQRT(MAX(AVG(b2.volume * b2.volume) - AVG(b2.volume) * AVG(b2.volume), 0))
                    FROM (
                        SELECT b3.volume FROM bougies b3
                        WHERE b3.asset = sf.asset AND b3.timeframe = sf.timeframe
                          AND b3.timestamp <= sf.cree_le
                        ORDER BY b3.timestamp DESC LIMIT 60 OFFSET 1
                    ) b2
                ) as vol_sigma
            FROM signaux_fermes sf
        )
        SELECT
            CASE
                WHEN vol_sigma > 0 AND vol_signal IS NOT NULL
                     AND (vol_signal - vol_moyen) / vol_sigma > $1
                THEN 'whale'
                ELSE 'normal'
            END as groupe,
            COUNT(*) as n,
            AVG(CASE WHEN est_gagnant THEN 1.0 ELSE 0.0 END) * 100 as wr_pct,
            COALESCE(SUM(r_realise), 0) as r_total,
            COALESCE(AVG(r_realise), 0) as r_moyen,
            AVG(CASE WHEN est_sl THEN 1.0 ELSE 0.0 END) * 100 as sl_pct
        FROM avec_zscore
        WHERE vol_signal IS NOT NULL AND vol_moyen IS NOT NULL
        GROUP BY 1
        "#,
    )
    .bind(seuil)
    .fetch_all(state.db.pool())
    .await
    {
        Ok(r) => r,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({"erreur": e.to_string()}));
        }
    };

    let mut whale = CohorteWhale {
        label: "Avec 🐋 (z > seuil)".into(),
        n: 0, wr_pct: 0.0, r_total: 0.0, r_moyen: 0.0, sl_pct: 0.0,
    };
    let mut normal = CohorteWhale {
        label: "Sans 🐋 (volume normal)".into(),
        n: 0, wr_pct: 0.0, r_total: 0.0, r_moyen: 0.0, sl_pct: 0.0,
    };

    for row in rows {
        let groupe: String = row.try_get("groupe").unwrap_or("normal".into());
        let target = if groupe == "whale" { &mut whale } else { &mut normal };
        target.n = row.try_get::<i64, _>("n").unwrap_or(0) as usize;
        target.wr_pct = row.try_get("wr_pct").unwrap_or(0.0);
        target.r_total = row.try_get("r_total").unwrap_or(0.0);
        target.r_moyen = row.try_get("r_moyen").unwrap_or(0.0);
        target.sl_pct = row.try_get("sl_pct").unwrap_or(0.0);
    }

    HttpResponse::Ok().json(LaboWhale {
        avec_whale: whale,
        sans_whale: normal,
        seuil_sigma: seuil,
    })
}

#[derive(serde::Deserialize)]
pub struct WhaleLaboQuery {
    pub seuil: Option<f64>,
}
