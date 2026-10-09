//! Réglages de stratégie PAR ASSET (spec docs/spec_reglages_par_asset.md, 0121).
//!
//! Surcharge OPTIONNELLE : tout champ `None` = repli sur le défaut global
//! (comportement d'aujourd'hui tant qu'aucune ligne n'existe). CRUD pur — la
//! FUSION surcharge ⊕ défaut vit côté API (une seule source de calcul).

use common::{Result, TradingError};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

/// Surcharge SMC par asset (tous champs optionnels).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SmcSurcharge {
    pub sl_max: Option<f64>,
    pub trailing_r: Option<f64>,
    pub tp1: Option<f64>,
    pub tp2: Option<f64>,
    pub tp3_mode: Option<String>,
    pub tp3_rfixe: Option<f64>,
    pub frac_tp1: Option<f64>,
    pub frac_tp2: Option<f64>,
    pub frac_tp3: Option<f64>,
}

/// Surcharge straddle par asset.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StraddleSurcharge {
    pub sl_mult: Option<f64>,
    pub trailing_r: Option<f64>,
    pub placement_sec: Option<i64>,
}

/// Surcharge KDJ par asset.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct KdjSurcharge {
    pub period: Option<i64>,
    pub signal: Option<i64>,
    pub amplitude: Option<i64>,
    pub ratio_risk: Option<f64>,
    pub adx_min: Option<f64>,
}

/// Une surcharge SMC est-elle TOUTE vide ?
pub fn smc_vide(s: &SmcSurcharge) -> bool {
    s.sl_max.is_none()
        && s.trailing_r.is_none()
        && s.tp1.is_none()
        && s.tp2.is_none()
        && s.tp3_mode.is_none()
        && s.tp3_rfixe.is_none()
        && s.frac_tp1.is_none()
        && s.frac_tp2.is_none()
        && s.frac_tp3.is_none()
}

/// Fractions surchargées ⇒ les TROIS fournies et somme = 1 (±0,001).
/// (décision propriétaire ③ : fractions explicites, jamais de surprise.)
pub fn fractions_smc_valides(s: &SmcSurcharge) -> Result<()> {
    let n = [s.frac_tp1, s.frac_tp2, s.frac_tp3].iter().filter(|x| x.is_some()).count();
    if n == 0 {
        return Ok(());
    }
    if n != 3 {
        return Err(TradingError::Database(
            "Fractions partielles interdites : fournir frac_tp1, frac_tp2 ET frac_tp3 (ou aucune)".into(),
        ));
    }
    let somme = s.frac_tp1.unwrap_or(0.0) + s.frac_tp2.unwrap_or(0.0) + s.frac_tp3.unwrap_or(0.0);
    if (somme - 1.0).abs() > 0.001 {
        return Err(TradingError::Database(format!(
            "Les fractions doivent sommer à 1,00 (somme reçue : {somme:.3})"
        )));
    }
    Ok(())
}

pub async fn lire_smc_surcharge(pool: &SqlitePool, asset: &str) -> Result<SmcSurcharge> {
    let ligne = sqlx::query_as::<
        _,
        (
            Option<f64>, Option<f64>, Option<f64>, Option<f64>,
            Option<String>, Option<f64>, Option<f64>, Option<f64>, Option<f64>,
        ),
    >(
        "SELECT sl_max, trailing_r, tp1, tp2, tp3_mode, tp3_rfixe, frac_tp1, frac_tp2, frac_tp3
         FROM smc_reglages_asset WHERE asset = ?",
    )
    .bind(asset)
    .fetch_optional(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;
    Ok(match ligne {
        None => SmcSurcharge::default(),
        Some((sl_max, trailing_r, tp1, tp2, tp3_mode, tp3_rfixe, f1, f2, f3)) => SmcSurcharge {
            sl_max, trailing_r, tp1, tp2, tp3_mode, tp3_rfixe,
            frac_tp1: f1, frac_tp2: f2, frac_tp3: f3,
        },
    })
}

pub async fn ecrire_smc_surcharge(pool: &SqlitePool, asset: &str, s: &SmcSurcharge) -> Result<()> {
    fractions_smc_valides(s)?;
    sqlx::query(
        "INSERT INTO smc_reglages_asset
           (asset, sl_max, trailing_r, tp1, tp2, tp3_mode, tp3_rfixe, frac_tp1, frac_tp2, frac_tp3, maj_le)
         VALUES (?,?,?,?,?,?,?,?,?,?,unixepoch())
         ON CONFLICT(asset) DO UPDATE SET
           sl_max=excluded.sl_max, trailing_r=excluded.trailing_r, tp1=excluded.tp1,
           tp2=excluded.tp2, tp3_mode=excluded.tp3_mode, tp3_rfixe=excluded.tp3_rfixe,
           frac_tp1=excluded.frac_tp1, frac_tp2=excluded.frac_tp2, frac_tp3=excluded.frac_tp3,
           maj_le=unixepoch()",
    )
    .bind(asset)
    .bind(s.sl_max)
    .bind(s.trailing_r)
    .bind(s.tp1)
    .bind(s.tp2)
    .bind(s.tp3_mode.as_deref())
    .bind(s.tp3_rfixe)
    .bind(s.frac_tp1)
    .bind(s.frac_tp2)
    .bind(s.frac_tp3)
    .execute(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;
    Ok(())
}

pub async fn supprimer_smc_surcharge(pool: &SqlitePool, asset: &str) -> Result<()> {
    sqlx::query("DELETE FROM smc_reglages_asset WHERE asset = ?")
        .bind(asset)
        .execute(pool)
        .await
        .map_err(|e| TradingError::Database(e.to_string()))?;
    Ok(())
}

pub async fn lire_straddle_surcharge(pool: &SqlitePool, asset: &str) -> Result<StraddleSurcharge> {
    let ligne = sqlx::query_as::<_, (Option<f64>, Option<f64>, Option<i64>)>(
        "SELECT sl_mult, trailing_r, placement_sec FROM straddle_reglages_asset WHERE asset = ?",
    )
    .bind(asset)
    .fetch_optional(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;
    Ok(match ligne {
        None => StraddleSurcharge::default(),
        Some((sl_mult, trailing_r, placement_sec)) => {
            StraddleSurcharge { sl_mult, trailing_r, placement_sec }
        }
    })
}

pub async fn ecrire_straddle_surcharge(
    pool: &SqlitePool,
    asset: &str,
    s: &StraddleSurcharge,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO straddle_reglages_asset (asset, sl_mult, trailing_r, placement_sec, maj_le)
         VALUES (?,?,?,?,unixepoch())
         ON CONFLICT(asset) DO UPDATE SET
           sl_mult=excluded.sl_mult, trailing_r=excluded.trailing_r,
           placement_sec=excluded.placement_sec, maj_le=unixepoch()",
    )
    .bind(asset)
    .bind(s.sl_mult)
    .bind(s.trailing_r)
    .bind(s.placement_sec)
    .execute(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;
    Ok(())
}

pub async fn supprimer_straddle_surcharge(pool: &SqlitePool, asset: &str) -> Result<()> {
    sqlx::query("DELETE FROM straddle_reglages_asset WHERE asset = ?")
        .bind(asset)
        .execute(pool)
        .await
        .map_err(|e| TradingError::Database(e.to_string()))?;
    Ok(())
}

pub async fn lire_kdj_surcharge(pool: &SqlitePool, asset: &str) -> Result<KdjSurcharge> {
    let ligne = sqlx::query_as::<
        _,
        (Option<i64>, Option<i64>, Option<i64>, Option<f64>, Option<f64>),
    >(
        "SELECT period, signal, amplitude, ratio_risk, adx_min FROM kdj_reglages_asset WHERE asset = ?",
    )
    .bind(asset)
    .fetch_optional(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;
    Ok(match ligne {
        None => KdjSurcharge::default(),
        Some((period, signal, amplitude, ratio_risk, adx_min)) => {
            KdjSurcharge { period, signal, amplitude, ratio_risk, adx_min }
        }
    })
}

pub async fn ecrire_kdj_surcharge(pool: &SqlitePool, asset: &str, s: &KdjSurcharge) -> Result<()> {
    sqlx::query(
        "INSERT INTO kdj_reglages_asset (asset, period, signal, amplitude, ratio_risk, adx_min, maj_le)
         VALUES (?,?,?,?,?,?,unixepoch())
         ON CONFLICT(asset) DO UPDATE SET
           period=excluded.period, signal=excluded.signal, amplitude=excluded.amplitude,
           ratio_risk=excluded.ratio_risk, adx_min=excluded.adx_min, maj_le=unixepoch()",
    )
    .bind(asset)
    .bind(s.period)
    .bind(s.signal)
    .bind(s.amplitude)
    .bind(s.ratio_risk)
    .bind(s.adx_min)
    .execute(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;
    Ok(())
}

pub async fn supprimer_kdj_surcharge(pool: &SqlitePool, asset: &str) -> Result<()> {
    sqlx::query("DELETE FROM kdj_reglages_asset WHERE asset = ?")
        .bind(asset)
        .execute(pool)
        .await
        .map_err(|e| TradingError::Database(e.to_string()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn base_test() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        pool
    }

    #[tokio::test]
    async fn surcharge_smc_aller_retour_et_suppression() {
        let pool = base_test().await;
        let vide = lire_smc_surcharge(&pool, "XAUUSD").await.unwrap();
        assert!(smc_vide(&vide));

        let s = SmcSurcharge {
            trailing_r: Some(0.2),
            frac_tp1: Some(1.0),
            frac_tp2: Some(0.0),
            frac_tp3: Some(0.0),
            ..Default::default()
        };
        ecrire_smc_surcharge(&pool, "XAUUSD", &s).await.unwrap();
        let lu = lire_smc_surcharge(&pool, "XAUUSD").await.unwrap();
        assert_eq!(lu.trailing_r, Some(0.2));
        assert_eq!(lu.tp1, None, "champ non surchargé = None (repli défaut)");
        assert_eq!(lu.frac_tp1, Some(1.0));

        supprimer_smc_surcharge(&pool, "XAUUSD").await.unwrap();
        assert!(smc_vide(&lire_smc_surcharge(&pool, "XAUUSD").await.unwrap()));
    }

    #[tokio::test]
    async fn fractions_partielles_ou_somme_fausse_refusees() {
        let s = SmcSurcharge { frac_tp1: Some(0.5), ..Default::default() };
        assert!(fractions_smc_valides(&s).is_err(), "fractions partielles interdites");

        let s = SmcSurcharge {
            frac_tp1: Some(0.5),
            frac_tp2: Some(0.4),
            frac_tp3: Some(0.2),
            ..Default::default()
        };
        assert!(fractions_smc_valides(&s).is_err(), "somme 1,1 refusée");

        let s = SmcSurcharge {
            frac_tp1: Some(0.5),
            frac_tp2: Some(0.3),
            frac_tp3: Some(0.2),
            ..Default::default()
        };
        assert!(fractions_smc_valides(&s).is_ok(), "somme 1,0 acceptée");

        let pool = base_test().await;
        let mauvaise = SmcSurcharge { frac_tp1: Some(1.0), ..Default::default() };
        assert!(ecrire_smc_surcharge(&pool, "BTC", &mauvaise).await.is_err());
    }

    #[tokio::test]
    async fn surcharges_straddle_et_kdj_aller_retour() {
        let pool = base_test().await;
        assert_eq!(lire_straddle_surcharge(&pool, "DAX").await.unwrap().sl_mult, None);
        ecrire_straddle_surcharge(
            &pool,
            "DAX",
            &StraddleSurcharge { sl_mult: Some(0.6), trailing_r: None, placement_sec: Some(45) },
        )
        .await
        .unwrap();
        let lu = lire_straddle_surcharge(&pool, "DAX").await.unwrap();
        assert_eq!(lu.sl_mult, Some(0.6));
        assert_eq!(lu.placement_sec, Some(45));
        assert_eq!(lu.trailing_r, None, "non surchargé = repli");
        supprimer_straddle_surcharge(&pool, "DAX").await.unwrap();
        assert_eq!(lire_straddle_surcharge(&pool, "DAX").await.unwrap().sl_mult, None);

        assert_eq!(lire_kdj_surcharge(&pool, "XRP").await.unwrap().period, None);
        ecrire_kdj_surcharge(
            &pool,
            "XRP",
            &KdjSurcharge {
                period: Some(14),
                signal: None,
                amplitude: Some(3),
                ratio_risk: Some(2.5),
                adx_min: Some(-1.0),
            },
        )
        .await
        .unwrap();
        let lu = lire_kdj_surcharge(&pool, "XRP").await.unwrap();
        assert_eq!(lu.period, Some(14));
        assert_eq!(lu.signal, None);
        assert_eq!(lu.ratio_risk, Some(2.5));
    }
}

/// Toutes les surcharges SMC par asset (empreinte hot-reload du tick).
pub async fn toutes_smc_surcharges(pool: &SqlitePool) -> Result<std::collections::HashMap<String, SmcSurcharge>> {
    let lignes = sqlx::query_as::<
        _,
        (String, Option<f64>, Option<f64>, Option<f64>, Option<f64>, Option<String>, Option<f64>, Option<f64>, Option<f64>, Option<f64>),
    >(
        "SELECT asset, sl_max, trailing_r, tp1, tp2, tp3_mode, tp3_rfixe, frac_tp1, frac_tp2, frac_tp3
         FROM smc_reglages_asset",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;
    Ok(lignes
        .into_iter()
        .map(|(asset, sl_max, trailing_r, tp1, tp2, tp3_mode, tp3_rfixe, f1, f2, f3)| {
            (
                asset,
                SmcSurcharge { sl_max, trailing_r, tp1, tp2, tp3_mode, tp3_rfixe, frac_tp1: f1, frac_tp2: f2, frac_tp3: f3 },
            )
        })
        .collect())
}

/// Toutes les surcharges straddle par asset.
pub async fn toutes_straddle_surcharges(pool: &SqlitePool) -> Result<std::collections::HashMap<String, StraddleSurcharge>> {
    let lignes = sqlx::query_as::<_, (String, Option<f64>, Option<f64>, Option<i64>)>(
        "SELECT asset, sl_mult, trailing_r, placement_sec FROM straddle_reglages_asset",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;
    Ok(lignes
        .into_iter()
        .map(|(asset, sl_mult, trailing_r, placement_sec)| (asset, StraddleSurcharge { sl_mult, trailing_r, placement_sec }))
        .collect())
}

/// Toutes les surcharges KDJ par asset.
pub async fn toutes_kdj_surcharges(pool: &SqlitePool) -> Result<std::collections::HashMap<String, KdjSurcharge>> {
    let lignes = sqlx::query_as::<_, (String, Option<i64>, Option<i64>, Option<i64>, Option<f64>, Option<f64>)>(
        "SELECT asset, period, signal, amplitude, ratio_risk, adx_min FROM kdj_reglages_asset",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;
    Ok(lignes
        .into_iter()
        .map(|(asset, period, signal, amplitude, ratio_risk, adx_min)| {
            (asset, KdjSurcharge { period, signal, amplitude, ratio_risk, adx_min })
        })
        .collect())
}
