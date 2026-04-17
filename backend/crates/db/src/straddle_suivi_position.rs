use common::{Result, TradingError};
use sqlx::{Row, SqlitePool};

// ── Structures ────────────────────────────────────────────────────────────────

pub struct SuiviJambe {
    pub id: i64,
    pub signal_id: String,
    pub jambe: String,          // "LONG" | "SHORT"
    pub atr: f64,               // ATR capturé au moment du signal
    pub peak: f64,              // max favorable (min pour SHORT)
    pub sl_effectif: f64,
    pub statut_jambe: String,   // "actif" | "tp1_touche" | "tp2_touche" | "cloturee"
    pub prix_tp1: Option<f64>,
    pub prix_tp2: Option<f64>,
    pub prix_cloture: Option<f64>,
    pub pnl_r_final: Option<f64>,
}

/// Vue jointe avec les niveaux du signal pour le moniteur.
pub struct SuiviActif {
    pub suivi_id: i64,
    pub signal_id: String,
    pub asset: String,
    pub jambe: String,
    pub prix_entree: f64,
    pub stop_loss: f64,         // SL d'origine de cette jambe
    pub tp1: f64,
    pub tp2: f64,
    pub atr: f64,
    pub peak: f64,
    pub sl_effectif: f64,
    pub statut_jambe: String,
    pub trailing_coeff: f64,    // Chargé depuis straddle_calibration via feedback
}

// ── Création ──────────────────────────────────────────────────────────────────

/// Initialise les deux lignes de suivi (LONG + SHORT) pour un signal Straddle.
/// Appel idempotent grâce à INSERT OR IGNORE.
pub async fn initialiser_suivi_signal(
    pool: &SqlitePool,
    signal_id: &str,
    prix_entree: f64,
    atr: f64,
    sl_long: f64,
    sl_short: f64,
) -> Result<()> {
    sqlx::query(
        "INSERT OR IGNORE INTO straddle_suivi_position
         (signal_id, jambe, atr, peak, sl_effectif)
         VALUES (?, 'LONG', ?, ?, ?), (?, 'SHORT', ?, ?, ?)",
    )
    .bind(signal_id)
    .bind(atr)
    .bind(prix_entree)  // peak initial = prix d'entrée
    .bind(sl_long)
    .bind(signal_id)
    .bind(atr)
    .bind(prix_entree)
    .bind(sl_short)
    .execute(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;
    Ok(())
}

// ── Lecture ───────────────────────────────────────────────────────────────────

/// Charge toutes les jambes actives (non clôturées) avec les niveaux du signal.
pub async fn lister_suivi_actifs(pool: &SqlitePool) -> Result<Vec<SuiviActif>> {
    let rows = sqlx::query(
        "SELECT s.id as suivi_id, s.signal_id, sig.asset, s.jambe,
                sig.prix_entree, sig.stop_loss, sig.sl_short,
                sig.take_profit, sig.take_profit_short,
                s.atr, s.peak, s.sl_effectif, s.statut_jambe,
                COALESCE(
                    (SELECT sc.trailing_coeff
                     FROM straddle_feedback sf
                     JOIN straddle_calibration sc
                       ON sc.asset = sf.asset AND sc.categorie = sf.categorie
                     WHERE sf.signal_id = s.signal_id
                     LIMIT 1),
                    2.0
                ) AS trailing_coeff
         FROM straddle_suivi_position s
         JOIN signaux sig ON sig.id = s.signal_id
         WHERE s.statut_jambe != 'cloturee'
           AND sig.statut = 'Actif'",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;

    let mut actifs = Vec::with_capacity(rows.len());
    for r in rows {
        let jambe: String = r.get("jambe");
        let prix_entree: f64 = r.get("prix_entree");
        let atr: f64 = r.get("atr");

        // Extraire tp1 et tp2 depuis le JSON de la jambe correspondante
        let (tp1, tp2, stop_loss) = if jambe == "LONG" {
            let tps: Vec<f64> = serde_json::from_str(
                r.get::<Option<&str>, _>("take_profit").unwrap_or("[]"),
            )
            .unwrap_or_default();
            let sl: f64 = r.get("stop_loss");
            (
                tps.first().copied().unwrap_or(prix_entree + 2.0 * atr),
                tps.get(1).copied().unwrap_or(prix_entree + 3.5 * atr),
                sl,
            )
        } else {
            let tps: Vec<f64> = serde_json::from_str(
                r.get::<Option<&str>, _>("take_profit_short").unwrap_or("[]"),
            )
            .unwrap_or_default();
            let sl: f64 = r.get::<Option<f64>, _>("sl_short").unwrap_or(prix_entree + 0.5 * atr);
            (
                tps.first().copied().unwrap_or(prix_entree - 2.0 * atr),
                tps.get(1).copied().unwrap_or(prix_entree - 3.5 * atr),
                sl,
            )
        };

        actifs.push(SuiviActif {
            suivi_id: r.get("suivi_id"),
            signal_id: r.get("signal_id"),
            asset: r.get("asset"),
            jambe,
            prix_entree,
            stop_loss,
            tp1,
            tp2,
            atr,
            peak: r.get("peak"),
            sl_effectif: r.get("sl_effectif"),
            statut_jambe: r.get("statut_jambe"),
            trailing_coeff: r.get::<Option<f64>, _>("trailing_coeff").unwrap_or(2.0),
        });
    }
    Ok(actifs)
}

// ── Mise à jour état intermédiaire ────────────────────────────────────────────

pub async fn maj_jambe_peak_sl(
    pool: &SqlitePool,
    suivi_id: i64,
    peak: f64,
    sl_effectif: f64,
) -> Result<()> {
    sqlx::query(
        "UPDATE straddle_suivi_position
         SET peak = ?, sl_effectif = ?, maj_le = unixepoch()
         WHERE id = ?",
    )
    .bind(peak)
    .bind(sl_effectif)
    .bind(suivi_id)
    .execute(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;
    Ok(())
}

pub async fn maj_jambe_tp1(pool: &SqlitePool, suivi_id: i64, prix_tp1: f64, sl: f64) -> Result<()> {
    sqlx::query(
        "UPDATE straddle_suivi_position
         SET statut_jambe = 'tp1_touche', prix_tp1 = ?, sl_effectif = ?, maj_le = unixepoch()
         WHERE id = ?",
    )
    .bind(prix_tp1)
    .bind(sl)
    .bind(suivi_id)
    .execute(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;
    Ok(())
}

pub async fn maj_jambe_tp2(pool: &SqlitePool, suivi_id: i64, prix_tp2: f64, sl: f64) -> Result<()> {
    sqlx::query(
        "UPDATE straddle_suivi_position
         SET statut_jambe = 'tp2_touche', prix_tp2 = ?, sl_effectif = ?, maj_le = unixepoch()
         WHERE id = ?",
    )
    .bind(prix_tp2)
    .bind(sl)
    .bind(suivi_id)
    .execute(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;
    Ok(())
}

pub async fn cloture_jambe(
    pool: &SqlitePool,
    suivi_id: i64,
    prix_cloture: f64,
    pnl_r: f64,
) -> Result<()> {
    sqlx::query(
        "UPDATE straddle_suivi_position
         SET statut_jambe = 'cloturee', prix_cloture = ?, pnl_r_final = ?, maj_le = unixepoch()
         WHERE id = ?",
    )
    .bind(prix_cloture)
    .bind(pnl_r)
    .bind(suivi_id)
    .execute(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;
    Ok(())
}

/// Clôture la jambe opposée d'un signal Straddle (appelé quand l'autre jambe touche TP1).
/// Sans effet si la jambe est déjà clôturée.
pub async fn cloture_jambe_opposee(
    pool: &SqlitePool,
    signal_id: &str,
    jambe_opposee: &str,  // "LONG" | "SHORT"
    prix_cloture: f64,
    pnl_r: f64,
) -> Result<()> {
    sqlx::query(
        "UPDATE straddle_suivi_position
         SET statut_jambe = 'cloturee', prix_cloture = ?, pnl_r_final = ?, maj_le = unixepoch()
         WHERE signal_id = ? AND jambe = ? AND statut_jambe != 'cloturee'",
    )
    .bind(prix_cloture)
    .bind(pnl_r)
    .bind(signal_id)
    .bind(jambe_opposee)
    .execute(pool)
    .await
    .map_err(|e| TradingError::Database(e.to_string()))?;
    Ok(())
}
