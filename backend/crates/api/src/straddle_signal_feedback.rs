//! Sauvegarde du feedback Straddle et du snapshot features ML.
//! Extrait de straddle_signal_ollama pour respecter la limite 300 lignes.
use common::{Asset, Signal, Timeframe};
use db::Database;
use std::sync::Arc;

/// Lie le pic ATR au signal, insère le feedback et le snapshot features ML.
pub async fn sauvegarder_feedback_et_features(
    db: &Arc<Database>,
    signal: &Signal,
    asset: &Asset,
    tf: &Timeframe,
    score_confiance: f64,
    ratio_atr: f64,
    bougies: &[common::Candle],
) {
    let signal_id_str = signal.id.to_string();

    // Lier le pic ATR détecté à ce signal
    let pic_id = if let Ok(Some(pid)) =
        db::straddle_pics::dernier_pic_asset(db.pool(), asset.as_str(), tf.as_str(), 60).await
    {
        let _ = db::straddle_pics::lier_signal(db.pool(), pid, &signal_id_str).await;
        Some(pid)
    } else {
        None
    };

    // Charger le pic pour récupérer catégorie, session, ratio réels
    let pic = if let Some(pid) = pic_id {
        db::straddle_pics::charger_par_id(db.pool(), pid)
            .await
            .unwrap_or(None)
    } else {
        None
    };

    let fb = db::straddle_feedback::NouveauFeedback {
        signal_id: &signal_id_str,
        pic_id,
        asset: asset.as_str(),
        timeframe: tf.as_str(),
        timestamp_signal: signal.cree_le.timestamp(),
        categorie: pic
            .as_ref()
            .map(|p| p.categorie.as_str())
            .unwrap_or("choc_isole"),
        evenement_nom: pic.as_ref().and_then(|p| p.evenement_nom.as_deref()),
        session_active: pic.as_ref().map(|p| p.session_active.as_str()),
        ratio_atr: pic.as_ref().map(|p| p.ratio_atr).unwrap_or(0.0),
        score_llm: score_confiance,
    };
    let _ = db::straddle_feedback::inserer_feedback(db.pool(), &fb).await;

    // Snapshot features ML (56 = 52 OHLCV + 4 contextuelles Straddle)
    if let Some(features_ohlcv) = ml::extraire_features(bougies) {
        let session = pic
            .as_ref()
            .map(|p| p.session_active.as_str())
            .unwrap_or("Asia/Off");
        let features_56 = db::straddle_features::construire_features_56(
            &features_ohlcv,
            ratio_atr,
            fb.categorie,
            session,
            score_confiance,
        );
        let _ = db::straddle_features::inserer_snapshot(
            db.pool(),
            &signal_id_str,
            asset.as_str(),
            &features_56,
        )
        .await;
    }
}
