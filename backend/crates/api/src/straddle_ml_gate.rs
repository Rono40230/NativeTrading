//! Gate ML pour la stratégie Straddle.
//! - ML confiant (>75%) dans une direction → skip (signal directionnel préférable)
//! - ML indécis (0.45–0.55) → bonus de contexte ajouté au prompt Ollama
use ml::PipelineML;
use std::sync::Arc;
use tokio::sync::Mutex;

pub enum MlContexteStraddle {
    /// ML confiant (>75%) dans une direction — ne pas émettre Straddle
    Directionnel(String),
    /// ML indécis (conf 0.45–0.55) — ajouter info dans le contexte Ollama
    Indecis(String),
    /// ML non disponible ou pas assez de bougies
    NonDisponible,
}

pub async fn evaluer_ml_straddle(
    pipeline_ml: &Arc<Mutex<PipelineML>>,
    bougies: &[common::Candle],
    asset: &str,
    tf: &str,
    seuil: f64,
) -> MlContexteStraddle {
    let ml = pipeline_ml.lock().await;
    if !ml.est_pret() {
        return MlContexteStraddle::NonDisponible;
    }
    match ml.predire(bougies) {
        Ok(pred) if pred.confiance > seuil => {
            tracing::debug!(
                "Gate ML Straddle {}/{}: ML confiant {:.0}% {:?} — skip",
                asset,
                tf,
                pred.confiance * 100.0,
                pred.direction
            );
            MlContexteStraddle::Directionnel(format!("{:?}", pred.direction))
        }
        Ok(pred) if pred.confiance >= 0.45 && pred.confiance <= 0.55 => {
            tracing::debug!(
                "Gate ML Straddle {}/{}: ML indécis {:.0}% — bonus contexte",
                asset,
                tf,
                pred.confiance * 100.0
            );
            MlContexteStraddle::Indecis(format!(
                "Signal ML : Indécision confirmée ({:.0}% de confiance) — favorable au Straddle\n",
                pred.confiance * 100.0
            ))
        }
        Ok(_) => MlContexteStraddle::NonDisponible,
        Err(_) => MlContexteStraddle::NonDisponible,
    }
}
