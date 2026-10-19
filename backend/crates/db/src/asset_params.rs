use common::{Result, TradingError};
use sqlx::{Row, SqlitePool};

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct AssetParams {
    pub asset: String,
    /// Type officiel de la table assets (crypto/metal/forex/indice) —
    /// servi pour la catégorisation UI (18/09, remplace la map front obsolète).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_asset: Option<String>,
    pub valeur_pips: f64,
    pub sl_pips: f64,
    pub pip_to_points: f64,
    pub lot_min: f64,
    pub lot_max: f64,
    pub taille_pip: f64,
}

pub async fn lire_tous(pool: &SqlitePool) -> Result<Vec<AssetParams>> {
    let rows = sqlx::query(
        "SELECT p.asset, a.type AS type_asset,
                p.valeur_pips, p.sl_pips, p.pip_to_points,
                p.lot_min, p.lot_max, p.taille_pip
         FROM asset_params p
         LEFT JOIN assets a ON a.id = p.asset
         ORDER BY p.asset",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;

    Ok(rows
        .iter()
        .map(|r| AssetParams {
            asset: r.get("asset"),
            type_asset: r.try_get("type_asset").ok().flatten(),
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
        type_asset: None,
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
         VALUES (?, ?, ?, ?, ?, ?, ?, datetime('now'))
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
             VALUES (?, ?, ?, ?, ?, ?, ?, datetime('now'))
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

/// Distance de SL fixe d'un asset en unités PRIX (owner 19/10 : doctrine
/// SL fixe — sl_pips × taille_pip, ex. XAU 50 × 0,1 = 5 $). Source unique
/// partagée Gestion du risque / modale SMC / moteur.
pub async fn lire_sl_fixe_prix(pool: &SqlitePool, asset: &str) -> Result<Option<f64>> {
    let row = sqlx::query(
        "SELECT sl_pips, taille_pip FROM asset_params WHERE asset = ?",
    )
    .bind(asset)
    .fetch_optional(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;
    Ok(row.map(|r| r.get::<f64, _>("sl_pips") * r.get::<f64, _>("taille_pip")))
}

/// Map asset → SL fixe en prix (toutes lignes) — un seul SELECT par tick.
pub async fn lire_sl_fixe_tous(pool: &SqlitePool) -> Result<std::collections::HashMap<String, f64>> {
    let rows = sqlx::query("SELECT asset, sl_pips * taille_pip AS sl FROM asset_params")
        .fetch_all(pool)
        .await
        .map_err(|e| TradingError::Database(e.to_string()))?;
    Ok(rows
        .iter()
        .map(|r| (r.get::<String, _>("asset"), r.get::<f64, _>("sl")))
        .collect())
}

/// Ligne (sl_pips, taille_pip) d'un asset — pour la conversion de la modale.
pub async fn lire_pips(pool: &SqlitePool, asset: &str) -> Result<Option<(f64, f64)>> {
    let row = sqlx::query("SELECT sl_pips, taille_pip FROM asset_params WHERE asset = ?")
        .bind(asset)
        .fetch_optional(pool)
        .await
        .map_err(|e| TradingError::Database(e.to_string()))?;
    Ok(row.map(|r| (r.get("sl_pips"), r.get("taille_pip"))))
}

/// Mise à jour du SL fixe (pips) d'un asset — borne ]0 ; 100 000].
pub async fn maj_sl_pips(pool: &SqlitePool, asset: &str, sl_pips: f64) -> Result<()> {
    if !sl_pips.is_finite() || sl_pips <= 0.0 || sl_pips > 100_000.0 {
        return Err(TradingError::Database("sl_pips hors bornes".into()));
    }
    sqlx::query("UPDATE asset_params SET sl_pips = ?, maj_le = datetime('now') WHERE asset = ?")
        .bind(sl_pips)
        .bind(asset)
        .execute(pool)
        .await
        .map_err(|e| TradingError::Database(e.to_string()))?;
    Ok(())
}
