use super::*;
use chrono::Utc;
use common::Candle;

fn bougie(c: f64) -> Candle {
    Candle {
        timestamp: Utc::now(),
        open: c,
        high: c + 1.0,
        low: c - 1.0,
        close: c,
        volume: 100.0,
    }
}

#[test]
fn pipeline_nouveau_non_pret() {
    let pipeline = PipelineML::new();
    assert!(
        !pipeline.est_pret(),
        "Un pipeline non entraîné ne doit pas être prêt"
    );
}

#[test]
fn predire_erreur_si_pas_assez_de_bougies() {
    let pipeline = PipelineML::new();
    let bougies: Vec<Candle> = (1..=30).map(|i| bougie(i as f64)).collect();
    assert!(pipeline.predire(&bougies).is_err());
}

#[test]
fn predire_erreur_si_modele_non_entraine() {
    let pipeline = PipelineML::new();
    let bougies: Vec<Candle> = (1..=70).map(|i| bougie(i as f64 * 100.0)).collect();
    assert!(pipeline.predire(&bougies).is_err());
}
