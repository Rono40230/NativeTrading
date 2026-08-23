use common::{Result, TradingError};
use sqlx::{Row, SqlitePool};

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct AssetParams {
    pub asset: String,
    pub valeur_pips: f64,
    pub sl_pips: f64,
    pub pip_to_points: f64,
    pub lot_min: f64,
    pub lot_max: f64,
    pub taille_pip: f64,
}

pub async fn lire_tous(pool: &SqlitePool) -> Result<Vec<AssetParams>> {
    let rows = sqlx::query(
        "SELECT asset, valeur_pips, sl_pips, pip_to_points, lot_min, lot_max, taille_pip
         FROM asset_params ORDER BY asset",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;

    Ok(rows
        .iter()
        .map(|r| AssetParams {
            asset: r.get("asset"),
            valeur_pips: r.get("valeur_pips"),
            sl_pips: r.get("sl_pips"),
            pip_to_points: r.get("pip_to_points"),
            lot_min: r.get("lot_min"),
            lot_max: r.get("lot_max"),
            taille_pip: r.try_get("taille_pip").unwrap_or(0.0001),
        })
        .collect())
}

pub async fn lire_un(pool: &SqlitePool, asset: &str) -> Result<Option<AssetParams>> {
    let row = sqlx::query(
        "SELECT asset, valeur_pips, sl_pips, pip_to_points, lot_min, lot_max, taille_pip
         FROM asset_params WHERE asset = ?",
    )
    .bind(asset)
    .fetch_optional(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;

    Ok(row.map(|r| AssetParams {
        asset: r.get("asset"),
        valeur_pips: r.get("valeur_pips"),
        sl_pips: r.get("sl_pips"),
        pip_to_points: r.get("pip_to_points"),
        lot_min: r.get("lot_min"),
        lot_max: r.get("lot_max"),
        taille_pip: r.try_get("taille_pip").unwrap_or(0.0001),
    }))
}

pub async fn sauvegarder(pool: &SqlitePool, params: &AssetParams) -> Result<()> {
    sqlx::query(
        "INSERT INTO asset_params (asset, valeur_pips, sl_pips, pip_to_points, lot_min, lot_max, taille_pip, maj_le)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, datetime('now'))
         ON CONFLICT(asset) DO UPDATE SET
             valeur_pips  = excluded.valeur_pips,
             sl_pips      = excluded.sl_pips,
             pip_to_points = excluded.pip_to_points,
             lot_min      = excluded.lot_min,
             lot_max      = excluded.lot_max,
             taille_pip   = excluded.taille_pip,
             maj_le       = excluded.maj_le",
    )
    .bind(&params.asset)
    .bind(params.valeur_pips)
    .bind(params.sl_pips)
    .bind(params.pip_to_points)
        .bind(params.lot_min)
    .bind(params.lot_max)
    .bind(params.taille_pip)
    .execute(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;
    Ok(())
}

pub async fn sauvegarder_tous(pool: &SqlitePool, liste: &[AssetParams]) -> Result<()> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| TradingError::Database(e.to_string()))?;

    for p in liste {
        sqlx::query(
            "INSERT INTO asset_params (asset, valeur_pips, sl_pips, pip_to_points, lot_min, lot_max, taille_pip, maj_le)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, datetime('now'))
             ON CONFLICT(asset) DO UPDATE SET
                 valeur_pips   = excluded.valeur_pips,
                 sl_pips       = excluded.sl_pips,
                 pip_to_points = excluded.pip_to_points,
                 lot_min       = excluded.lot_min,
                 lot_max       = excluded.lot_max,
                 taille_pip    = excluded.taille_pip,
                 maj_le        = excluded.maj_le",
        )
        .bind(&p.asset)
        .bind(p.valeur_pips)
        .bind(p.sl_pips)
        .bind(p.pip_to_points)
        .bind(p.lot_min)
        .bind(p.lot_max)
        .bind(p.taille_pip)
        .execute(&mut *tx)
        .await
        .map_err(|e| TradingError::Database(e.to_string()))?;
    }

    tx.commit()
        .await
        .map_err(|e| TradingError::Database(e.to_string()))?;
    Ok(())
}
