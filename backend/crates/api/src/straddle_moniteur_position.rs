//! Moniteur temps-réel des positions Straddle ouvertes.
//!
//! Cycle ~1 min — pour chaque jambe active :
//!   1. Fetch prix live (`fetch_prix_asset`)
//!   2. Calcule peak courant + verdict via `position_tracking::calculer_verdict`
//!   3. Met à jour la table `straddle_suivi_position`
//!   4. Log TP partiels et clôtures

use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

use crate::prix_utils::fetch_prix_asset;
use db::{straddle_suivi_position as ssp, strategies_params, Database};
use strategies::position_tracking::{calculer_verdict, PositionConfig, Verdict};

// ── Entrée publique ───────────────────────────────────────────────────────────

pub fn demarrer_moniteur_straddle(db: Arc<Database>) {
    tokio::spawn(async move {
        sleep(Duration::from_secs(90)).await; // laisse le temps aux autres workers de démarrer
        loop {
            run_cycle(&db).await;
            sleep(Duration::from_secs(60)).await;
        }
    });
    tracing::info!("⏱️  Moniteur Straddle démarré (cycle 60s)");
}

// ── Cycle principal ───────────────────────────────────────────────────────────

async fn run_cycle(db: &Arc<Database>) {
    let pool = db.pool();
    let params = strategies_params::lire_straddle_params(pool).await;

    // Auto-init : crée les lignes de suivi manquantes pour les signaux Straddle actifs
    initialiser_signaux_sans_suivi(pool, params.tp_mult_1.max(0.01)).await;

    let actifs = match ssp::lister_suivi_actifs(pool).await {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("Moniteur Straddle: lecture suivi: {}", e);
            return;
        }
    };
    if actifs.is_empty() {
        return;
    }

    let client = &*crate::http_client::HTTP_CLIENT;

    for jambe in &actifs {
        let Some(prix) = fetch_prix_asset(&client, &jambe.asset).await else {
            continue;
        };

        let risque = (jambe.prix_entree - jambe.stop_loss).abs(); // 1R

        if jambe.jambe == "LONG" {
            traiter_long(pool, jambe, prix, risque, &params).await;
        } else {
            traiter_short(pool, jambe, prix, risque, &params).await;
        }
    }
}

// ── Auto-initialisation ───────────────────────────────────────────────────────

/// Crée les lignes de suivi pour tout signal Straddle Actif sans entrée dans straddle_suivi_position.
async fn initialiser_signaux_sans_suivi(pool: &sqlx::SqlitePool, tp_mult_1: f64) {
    let rows = sqlx::query(
        "SELECT id, prix_entree, stop_loss, sl_short, take_profit
         FROM signaux
         WHERE strategie = 'Straddle' AND statut = 'Actif'
           AND id NOT IN (SELECT DISTINCT signal_id FROM straddle_suivi_position)",
    )
    .fetch_all(pool)
    .await;

    let rows = match rows {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("Moniteur Straddle auto-init: {}", e);
            return;
        }
    };

    for r in rows {
        use sqlx::Row;
        let signal_id: String = r.get("id");
        let prix_entree: f64 = r.get("prix_entree");
        let sl_long: f64 = r.get("stop_loss");
        let sl_short: f64 = r.get::<Option<f64>, _>("sl_short").unwrap_or(prix_entree);

        // Reconstituer ATR depuis tp1 : tp1 = entree + tp_mult_1 × atr
        let tp1_long = serde_json::from_str::<Vec<f64>>(
            r.get::<Option<&str>, _>("take_profit").unwrap_or("[]"),
        )
        .unwrap_or_default()
        .into_iter()
        .next()
        .unwrap_or(prix_entree);

        let atr = if tp1_long > prix_entree {
            (tp1_long - prix_entree) / tp_mult_1
        } else {
            (prix_entree - sl_long).abs() / 0.5 // fallback : SL = 0.5 × ATR
        };

        if let Err(e) = ssp::initialiser_suivi_signal(pool, &signal_id, prix_entree, atr, sl_long, sl_short).await {
            tracing::warn!("Moniteur Straddle: init suivi {}: {}", signal_id, e);
        } else {
            tracing::info!("📋 Straddle suivi initialisé pour signal {}", signal_id);
        }
    }
}

// ── LONG ──────────────────────────────────────────────────────────────────────

async fn traiter_long(
    pool: &sqlx::SqlitePool,
    j: &ssp::SuiviActif,
    prix: f64,
    risque: f64,
    params: &db::strategies_params::StraddleParams,
) {
    let peak_precedent = j.peak;
    let peak = peak_precedent.max(prix);

    let cfg = PositionConfig {
        is_long: true,
        prix_entree: j.prix_entree,
        stop_loss: j.stop_loss,
        tp1: j.tp1,
        tp2: j.tp2,
        atr: j.atr,
        trailing_coeff: j.trailing_coeff,
        vente_partielle_active: params.vente_partielle,
        pct_cloture_tp1: params.pct_cloture_tp1,
        pct_cloture_tp2: params.pct_cloture_tp2,
    };

    appliquer_verdict(pool, j, peak_precedent, peak, prix, prix, &cfg, risque, false).await;
}

// ── SHORT (miroir) ────────────────────────────────────────────────────────────

/// Pour la jambe SHORT, on travaille dans un espace "miroir" autour du prix d'entrée.
/// `flip(x) = 2 * prix_entree - x` — les prix courts croissants deviennent décroissants.
async fn traiter_short(
    pool: &sqlx::SqlitePool,
    j: &ssp::SuiviActif,
    prix: f64,
    risque: f64,
    params: &db::strategies_params::StraddleParams,
) {
    let peak_precedent = j.peak; 
    let peak = if peak_precedent == 0.0 { prix } else { peak_precedent.min(prix) }; // pour short, peak est le min (nadir)

    let cfg = PositionConfig {
        is_long: false,
        prix_entree: j.prix_entree,
        stop_loss: j.stop_loss, // On passe les VRAIES valeurs
        tp1: j.tp1,
        tp2: j.tp2,
        atr: j.atr,
        trailing_coeff: j.trailing_coeff,
        vente_partielle_active: params.vente_partielle,
        pct_cloture_tp1: params.pct_cloture_tp1,
        pct_cloture_tp2: params.pct_cloture_tp2,
    };

    // Plus besoin de tout flipper ! On passe les prix réels
    appliquer_verdict(pool, j, peak_precedent, peak, prix, prix, &cfg, risque, true).await;
}

// ── Application du verdict ────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
async fn appliquer_verdict(
    pool: &sqlx::SqlitePool,
    j: &ssp::SuiviActif,
    peak_precedent: f64,
    peak: f64,
    prix: f64,           
    prix_reel: f64,      
    cfg: &PositionConfig,
    risque: f64,
    is_short: bool,
) {
    let verdict = calculer_verdict(cfg, prix, peak, peak_precedent);
    
    if peak != peak_precedent {
        let sl_cour = j.sl_effectif; 
        let _ = ssp::maj_jambe_peak_sl(pool, j.suivi_id, peak, sl_cour).await;
    }

    match verdict {
        Verdict::Tp1Partiel { r_encaisse: _, pct_vendu: _ } => {
            tracing::info!(
                "📈 Straddle {} jambe {} TP1 partiel @ {:.5}",
                j.signal_id, j.jambe, prix
            );
            let _ = ssp::maj_jambe_tp1(pool, j.suivi_id, prix, j.prix_entree).await;
            
            // Clôture automatique de la jambe opposée
            let jambe_opposee = if is_short { "LONG" } else { "SHORT" };
            let pnl_r_opposee = if is_short {
                (prix_reel - j.prix_entree) / risque.max(1e-9) // long
            } else {
                (j.prix_entree - prix_reel) / risque.max(1e-9) // short
            };
            tracing::info!(
                "🔴 Straddle {} jambe {} clôturée (TP1 opposée) @ {:.5} ({:+.2}R)",
                j.signal_id, jambe_opposee, prix_reel, pnl_r_opposee
            );
            let _ = ssp::cloture_jambe_opposee(
                pool, &j.signal_id, jambe_opposee, prix_reel, pnl_r_opposee,
            ).await;
        }
        Verdict::Tp2Partiel { r_encaisse, pct_vendu: _ } => {
            tracing::info!(
                "📈 Straddle {} jambe {} TP2 partiel @ {:.5}",
                j.signal_id, j.jambe, prix
            );
            let _ = ssp::maj_jambe_tp2(pool, j.suivi_id, prix, j.tp1).await;
            let _ = ssp::cloture_jambe(pool, j.suivi_id, prix, r_encaisse).await;
        }
        Verdict::ClotureTotale { label, r_final, prix_cloture } => {
            tracing::info!(
                "🔴 Straddle {} jambe {} {} @ {:.5} ({:+.2}R)",
                j.signal_id, j.jambe, label, prix_cloture, r_final
            );
            let _ = ssp::cloture_jambe(pool, j.suivi_id, prix_cloture, r_final).await;
        }
        Verdict::Rien => {}
    }
}
