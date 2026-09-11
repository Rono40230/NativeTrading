//! Journal du cycle de vie des setups (scanner SMC — 11/09). Le vivier
//! `setups-formation` est une liste en mémoire purgée à 2 h : ici, chaque
//! setup annoncé laisse sa trace et son issue (`signal` | `dissipe`).
//! Matière première de l'étude §3.2 « voile des setups ».

use sqlx::{Row, SqlitePool};

#[derive(Debug, Clone, serde::Serialize)]
pub struct SetupJournal {
    pub cle: String,
    pub strategie: String,
    pub asset: String,
    pub tf: String,
    pub direction: String,
    pub force_max: i64,
    pub entree: f64,
    pub sl: f64,
    pub tps: String,
    pub debut: i64,
    pub annonce_le: i64,
    pub fin: Option<i64>,
    pub issue: Option<String>,
    pub signal_id: Option<String>,
}

/// Enregistre/met à jour une annonce (la force et les niveaux peuvent
/// s'affiner en cours de barre — on garde le maximum atteint).
#[allow(clippy::too_many_arguments)]
pub async fn upsert_annonce(
    pool: &SqlitePool,
    j: &SetupJournal,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO smc_setups_journal
             (cle, strategie, asset, tf, direction, force_max, entree, sl, tps,
              debut, annonce_le, fin, issue, signal_id)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, NULL, NULL, NULL)
         ON CONFLICT(cle) DO UPDATE SET
             force_max  = MAX(force_max, excluded.force_max),
             entree     = excluded.entree,
             sl         = excluded.sl,
             tps        = excluded.tps,
             annonce_le = excluded.annonce_le",
    )
    .bind(&j.cle)
    .bind(&j.strategie)
    .bind(&j.asset)
    .bind(&j.tf)
    .bind(&j.direction)
    .bind(j.force_max)
    .bind(j.entree)
    .bind(j.sl)
    .bind(&j.tps)
    .bind(j.debut)
    .bind(j.annonce_le)
    .execute(pool)
    .await?;
    Ok(())
}

/// Clôt une entrée (issue + fin) — idempotent : ne touche que les lignes
/// encore ouvertes (`issue IS NULL`).
pub async fn clôturer(
    pool: &SqlitePool,
    cle: &str,
    issue: &str,
    fin: i64,
    signal_id: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE smc_setups_journal
         SET issue = ?, fin = ?, signal_id = ?
         WHERE cle = ? AND issue IS NULL",
    )
    .bind(issue)
    .bind(fin)
    .bind(signal_id)
    .bind(cle)
    .execute(pool)
    .await?;
    Ok(())
}

/// Les plus récentes d'abord (le scanner affiche les dernières heures).
pub async fn lister(pool: &SqlitePool, limite: i64) -> Result<Vec<SetupJournal>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT cle, strategie, asset, tf, direction, force_max, entree, sl, tps,
                debut, annonce_le, fin, issue, signal_id
         FROM smc_setups_journal
         ORDER BY annonce_le DESC
         LIMIT ?",
    )
    .bind(limite)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .iter()
        .map(|r| SetupJournal {
            cle: r.get("cle"),
            strategie: r.get("strategie"),
            asset: r.get("asset"),
            tf: r.get("tf"),
            direction: r.get("direction"),
            force_max: r.get("force_max"),
            entree: r.get("entree"),
            sl: r.get("sl"),
            tps: r.get("tps"),
            debut: r.get("debut"),
            annonce_le: r.get("annonce_le"),
            fin: r.get("fin"),
            issue: r.get("issue"),
            signal_id: r.get("signal_id"),
        })
        .collect())
}
