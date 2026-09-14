//! Paramètres moteur KDJ/Halftrend (table `kdj_params`, ligne unique id=1).

use common::{Result, TradingError};
use sqlx::SqlitePool;

#[derive(Debug, Clone, serde::Serialize)]
pub struct KdjParams {
    pub period: i64,
    pub signal: i64,
    pub amplitude: i64,
    pub ratio_risk: f64,
    /// < 0 = filtre tendance désactivé (fidélité étalon).
    pub adx_min: f64,
}

impl Default for KdjParams {
    fn default() -> Self {
        Self { period: 20, signal: 7, amplitude: 2, ratio_risk: 2.0, adx_min: -1.0 }
    }
}

pub async fn lire_kdj_params(pool: &SqlitePool) -> KdjParams {
    let Ok(ligne) = sqlx::query_as::<_, (i64, i64, i64, f64, f64)>(
        "SELECT period, signal, amplitude, ratio_risk, adx_min FROM kdj_params WHERE id = 1",
    )
    .fetch_one(pool)
    .await
    else {
        return KdjParams::default();
    };
    KdjParams {
        period: ligne.0,
        signal: ligne.1,
        amplitude: ligne.2,
        ratio_risk: ligne.3,
        adx_min: ligne.4,
    }
}

pub async fn sauvegarder_kdj_params(pool: &SqlitePool, p: &KdjParams) -> Result<()> {
    sqlx::query(
        "UPDATE kdj_params SET period = ?, signal = ?, amplitude = ?, ratio_risk = ?, adx_min = ? WHERE id = 1",
    )
    .bind(p.period.max(2))
    .bind(p.signal.max(2))
    .bind(p.amplitude.max(1))
    .bind(p.ratio_risk.clamp(0.1, 10.0))
    .bind(p.adx_min.clamp(-1.0, 60.0))
    .execute(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_fideles_au_screening() {
        let p = KdjParams::default();
        assert_eq!((p.period, p.signal, p.amplitude), (20, 7, 2));
        assert!((p.ratio_risk - 2.0).abs() < 1e-9);
        assert!(p.adx_min < 0.0, "filtre ADX désactivé par défaut");
    }
}
