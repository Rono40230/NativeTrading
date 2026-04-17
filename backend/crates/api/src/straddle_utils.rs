use serde::{Deserialize, Serialize};

// ── Requêtes / réponses ──────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct RequeteAnalyse {
    pub asset: String,
    pub periode: Option<String>, // "3m" | "6m" | "1a" | "2a"
}

#[derive(Deserialize)]
pub struct MaJCreneau {
    pub statut: Option<String>,
    pub backtest_winrate: Option<f64>,
    pub backtest_profit_factor: Option<f64>,
}

#[derive(Serialize)]
pub(crate) struct ReponseAnalyse {
    pub creneaux: Vec<db::straddle::StraddleCreneau>,
    pub nb_analyses: usize,
    pub nb_retenus: usize,
}

// ── Helpers ───────────────────────────────────────────────────────────────────

pub fn periode_en_mois(p: Option<&str>) -> u32 {
    match p {
        Some("3m") => 3,
        Some("1a") => 12,
        Some("2a") => 24,
        _ => 6,
    }
}

/// Nombre de bougies H1 demandées en DB (sans plafond — le cache peut être grand).
pub fn limite_bougies(mois: u32) -> usize {
    mois as usize * 30 * 24
}

/// Plafond pour les providers réseau (Binance : max 1000 bougies par appel).
pub const MAX_BOUGIES_RESEAU: usize = 1000;

/// Retourne le groupe de corrélation de l'asset, ou `None` si isolé.
/// Règle P5 : si un signal Straddle est actif pour un autre asset du même groupe → skip.
pub fn groupe_correlation(asset: &str) -> Option<&'static [&'static str]> {
    const CRYPTO: &[&str] = &["BTC", "ETH"];
    const METAUX: &[&str] = &["XAUUSD", "XAGUSD"];
    if CRYPTO.contains(&asset) { return Some(CRYPTO); }
    if METAUX.contains(&asset) { return Some(METAUX); }
    None
}

/// Retourne le délai whipsaw (minutes) du créneau actif pour `hm` (heure*60+minute).
/// Retourne `None` si aucun créneau ne correspond ou si `whipsaw_minutes` = 0.
pub fn whipsaw_pour_heure(creneaux: &[db::straddle::StraddleCreneau], hm: u32) -> Option<i64> {
    fn parse_hm(s: &str) -> u32 {
        let mut it = s.splitn(2, ':');
        let h: u32 = it.next().and_then(|v| v.parse().ok()).unwrap_or(0);
        let m: u32 = it.next().and_then(|v| v.parse().ok()).unwrap_or(0);
        h * 60 + m
    }
    creneaux
        .iter()
        .filter(|c| c.statut == "valide" || c.statut == "a_tester")
        .filter(|c| {
            let d = parse_hm(&c.heure_debut);
            let f = parse_hm(&c.heure_fin);
            if d <= f { hm >= d && hm <= f } else { hm >= d || hm <= f }
        })
        .filter_map(|c| c.whipsaw_minutes)
        .find(|&w| w > 0)
}

/// Formate le bloc annonces HIGH impact pour le contexte LLM Straddle.
pub fn formater_annonces_contexte(annonces: &[serde_json::Value], maintenant: i64) -> String {
    if annonces.is_empty() {
        return "Annonces HIGH impact < 90min: aucune\n".to_string();
    }
    let mut s = "Annonces HIGH impact < 90min:\n".to_string();
    for a in annonces {
        let dans = a["date_heure"]
            .as_str()
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| (dt.timestamp() - maintenant) / 60)
            .unwrap_or(0);
        s.push_str(&format!(
            "  - {} | {} | dans {}min\n",
            a["titre"].as_str().unwrap_or("?"),
            a["devise"].as_str().unwrap_or("?"),
            dans
        ));
    }
    s
}

/// Sauvegarde le snapshot de features ML pour un signal Straddle.
/// Silencieux en cas d'échec (non-bloquant).
#[allow(clippy::too_many_arguments)]
pub async fn sauvegarder_snapshot_ml(
    pool: &sqlx::SqlitePool,
    db: &std::sync::Arc<db::Database>,
    signal_id: &str,
    asset_str: &str,
    tf: &common::Timeframe,
    ratio_atr: f64,
    categorie: &str,
    score_confiance: f64,
    now: chrono::DateTime<chrono::Utc>,
) {
    let Ok(bougies) = db.obtenir_bougies(
        &common::Asset::try_from(asset_str).unwrap_or(common::Asset::BTC),
        tf,
        100,
    ).await else { return };
    let Some(features_ohlcv) = ml::extraire_features(&bougies) else { return };
    let session = db::session_sortie_courante(now.timestamp());
    let features_56 = db::straddle_features::construire_features_56(
        &features_ohlcv,
        ratio_atr,
        categorie,
        &session,
        score_confiance,
    );
    let _ = db::straddle_features::inserer_snapshot(pool, signal_id, asset_str, &features_56).await;
}

/// Construit la réponse JSON d'un signal Straddle validé.
#[allow(clippy::too_many_arguments)]
pub fn reponse_signal_straddle(
    brut: &crate::straddle_types::ReponseLlm,
    signal_id: &str,
    heure_entree: Option<i64>,
    prix: f64,
    sl_long: f64, sl_short: f64,
    tp1_long: f64, tp1_short: f64,
    tp2_long: f64, tp2_short: f64,
    tp3_long: f64, tp3_short: f64,
    modele: &str,
) -> actix_web::HttpResponse {
    actix_web::HttpResponse::Ok().json(serde_json::json!({
        "signal": "STRADDLE",
        "declencheur": brut.declencheur,
        "raison": brut.raison,
        "score_confiance": brut.score_confiance,
        "amplitude_attendue_pct": brut.amplitude_attendue_pct,
        "duree_exposition_estimee_min": brut.duree_exposition_estimee_min,
        "signal_id": signal_id,
        "heure_entree": heure_entree,
        "prix_entree": prix,
        "sl_long": sl_long, "sl_short": sl_short,
        "tp1_long": tp1_long, "tp1_short": tp1_short,
        "tp2_long": tp2_long, "tp2_short": tp2_short,
        "tp3_long": tp3_long, "tp3_short": tp3_short,
        "modele": modele
    }))
}
