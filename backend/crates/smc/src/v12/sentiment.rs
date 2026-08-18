//! Sentiment composite technique (100% local, gratuit).
//!
//! Calcule un score de sentiment 0-100 par classe d'actifs (crypto/forex/metaux/
//! indices) à partir d'indicateurs techniques D1 (RSI14 + MA20). Les composantes
//! externes (Fear & Greed, VIX) sont injectées par la couche API
//! (`api::sentiment_composite`) qui agrège ensuite le tout.
//!
//! Phase 1 du système de sentiment composite.

use std::collections::HashMap;

use common::Candle;
use serde::{Deserialize, Serialize};

/// Score de sentiment composite 0-100 par classe d'actifs.
///
/// Chaque classe (crypto/forex/metaux/indices) est `Option` car toutes les
/// classes ne sont pas toujours disponibles (ex: marché fermé, données
/// insuffisantes). `global` est la moyenne pondérée des classes disponibles.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SentimentScore {
    /// Score global (moyenne des classes disponibles), 0-100.
    pub global: Option<f64>,
    /// Score classe crypto, 0-100.
    pub crypto: Option<f64>,
    /// Score classe forex, 0-100.
    pub forex: Option<f64>,
    /// Score classe métaux, 0-100.
    pub metaux: Option<f64>,
    /// Score classe indices, 0-100.
    pub indices: Option<f64>,
    // ── Composantes (transparence / debug frontend) ──
    /// RSI14 D1 du BTC (score technique brut).
    pub rsi_btc: Option<f64>,
    /// RSI14 D1 de l'ETH.
    pub rsi_eth: Option<f64>,
    /// RSI14 D1 de l'or (XAUUSD).
    pub rsi_xau: Option<f64>,
    /// Breadth : % d'actifs au-dessus de leur MA20 D1.
    pub breadth_pct: Option<f64>,
    /// Fear & Greed Index (alternative.me), 0-100.
    pub fear_greed: Option<f64>,
    /// VIX normalisé inversé (VIX bas = greed → score élevé).
    pub vix_score: Option<f64>,
    /// VIX brut (volatilité implicite, ~10-50).
    pub vix_brut: Option<f64>,
    /// CNN Fear & Greed (actions US) — LA référence du marché (7 composantes
    /// officielles : put/call, VIX/50, momentum, strength, breadth, junk
    /// bonds, safe haven). Décision propriétaire 2026-08-18 : jauge globale
    /// et classe indices.
    pub cnn_fg: Option<f64>,
    /// Libellé officiel CNN ("extreme fear".."extreme greed").
    pub cnn_rating: Option<String>,
}

/// Alignement du signal par rapport au sentiment de la classe d'actif.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Alignement {
    /// Signal dans le sens du sentiment (bullish+Long / bearish+Short).
    Aligne,
    /// Signal à contre-sens du sentiment.
    Oppose,
    /// Sentiment neutre (40-60), pas d'ajustement.
    Neutre,
    /// Sentiment extrême (< 20 ou > 80) à contre-sens → signal à ignorer.
    Extreme,
}

/// Verdict du post-filtre directionnel appliqué à un signal candidat.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentimentVerdict {
    /// Ajustement en % à appliquer à la force/conviction (-100 à +15).
    pub ajustement: f64,
    /// Type d'alignement détecté.
    pub alignement: Alignement,
    /// Score de sentiment de la classe de l'asset (ou global fallback).
    pub score_classe: f64,
}

/// Classifie un asset par sa classe de marché.
///
/// Contrairement à `common::Asset::is_crypto` (qui classe par SOURCE de données,
/// ex: XAUUSD routé vers Binance), cette fonction classe par TYPE de marché.
/// XAUUSD → "metaux" (pas "crypto").
///
/// Retourne `"crypto" | "forex" | "metaux" | "indices"`.
pub fn classe_actif(asset: &str) -> &'static str {
    let a = asset.trim().to_uppercase();
    match a.as_str() {
        "BTC" | "ETH" | "SOL" | "BNB" | "XRP" | "ADA" | "DOGE" | "AVAX" | "LINK" | "DOT" => "crypto",
        "XAUUSD" | "XAGUSD" | "XPTUSD" | "XPDUSD" | "XAU" | "XAG" => "metaux",
        "DAX" | "NAS100" | "SP500" | "US30" | "FTSE100" | "CAC40" | "JP225" => "indices",
        _ => "forex", // Paires 6 lettres (EURUSD…) et défaut → forex
    }
}

/// Borne une valeur dans [0, 100].
fn borne(v: f64) -> f64 {
    v.clamp(0.0, 100.0)
}

/// Calcule le sentiment technique (0-100) pour chaque actif à partir de ses
/// bougies Daily.
///
/// Formule par actif :
///   - RSI14 D1 : `50 + (rsi - 50)` (= rsi, borné 0-100). Sert de score de base.
///   - Bonus MA20 : prix > MA20 → +5, sinon -5.
///
/// Les actifs avec < 20 bougies D1 sont ignorés (warmup RSI14 + MA20).
///
/// Retourne un map `asset → score` (uniquement les actifs calculables).
pub fn calculer_sentiment_technique(bougies_d1: &[(String, Vec<Candle>)]) -> HashMap<String, f64> {
    let mut scores = HashMap::with_capacity(bougies_d1.len());
    for (asset, candles) in bougies_d1 {
        if candles.len() < 20 {
            continue;
        }

        // RSI14 D1 — dernière valeur valide, sinon neutre (50).
        let rsi_series = indicators::calculer_rsi(candles, 14);
        let rsi = rsi_series
            .last()
            .copied()
            .filter(|v| !v.is_nan())
            .unwrap_or(50.0);
        // Normalisation : 50 + (rsi - 50) = rsi, mais on borne explicitement.
        let mut score = borne(50.0 + (rsi - 50.0));

        // Bonus MA20 : direction de la moyenne mobile simple 20 périodes.
        let sma20 = indicators::calculer_sma(candles, 20);
        if let Some(&prix) = candles.last().map(|c| &c.close) {
            if let Some(ma) = sma20.last().copied().filter(|v| !v.is_nan()) {
                score += if prix > ma { 5.0 } else { -5.0 };
            }
        }
        score = borne(score);

        scores.insert(asset.clone(), score);
    }
    scores
}

/// Agrège les scores techniques par classe d'actifs.
///
/// Pour chaque classe, moyenne des scores des actifs de cette classe. Le
/// `global` est la moyenne simple des classes disponibles (pondération égale
/// entre classes, indépendamment du nombre d'actifs par classe).
///
/// Les composantes externes (F&G, VIX) restent `None` ici — elles sont injectées
/// puis combinées par `api::sentiment_composite::calculer_composite`.
pub fn agreg_par_classe(scores: &HashMap<String, f64>) -> SentimentScore {
    // Regroupement par classe.
    let mut par_classe: HashMap<&'static str, Vec<f64>> = HashMap::new();
    let mut au_dessus_ma = 0u32;
    let mut total = 0u32;
    for (asset, &score) in scores {
        par_classe.entry(classe_actif(asset)).or_default().push(score);
        total += 1;
        // Le score technique intègre déjà le bonus MA20 (+5 si prix>MA20) :
        // un score > 50 avec contribution MA20 positive est compté comme breadth.
        // Approximation : breadth = % d'actifs avec score > 50.
        if score > 50.0 {
            au_dessus_ma += 1;
        }
    }

    let moyenne = |classe: &'static str| -> Option<f64> {
        par_classe
            .get(classe)
            .filter(|v| !v.is_empty())
            .map(|v| v.iter().sum::<f64>() / v.len() as f64)
    };

    let crypto = moyenne("crypto");
    let forex = moyenne("forex");
    let metaux = moyenne("metaux");
    let indices = moyenne("indices");

    let dispo: Vec<f64> = [crypto, forex, metaux, indices]
        .into_iter()
        .flatten()
        .collect();
    let global = if dispo.is_empty() {
        None
    } else {
        Some(dispo.iter().sum::<f64>() / dispo.len() as f64)
    };

    let breadth_pct = if total > 0 {
        Some(au_dessus_ma as f64 / total as f64 * 100.0)
    } else {
        None
    };

    SentimentScore {
        global,
        crypto,
        forex,
        metaux,
        indices,
        breadth_pct,
        // Composantes externes : remplies par la couche API.
        rsi_btc: scores.get("BTC").copied(),
        rsi_eth: scores.get("ETH").copied(),
        rsi_xau: scores.get("XAUUSD").copied(),
        fear_greed: None,
        vix_score: None,
        vix_brut: None,
        cnn_fg: None,
        cnn_rating: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn bougie(close: f64) -> Candle {
        Candle {
            timestamp: Utc::now(),
            open: close,
            high: close + 1.0,
            low: close - 1.0,
            close,
            volume: 1000.0,
        }
    }

    fn tendance_haussiere(n: usize) -> Vec<Candle> {
        (0..n)
            .map(|i| bougie(100.0 + i as f64 * 2.0))
            .collect()
    }

    fn tendance_baissiere(n: usize) -> Vec<Candle> {
        (0..n)
            .map(|i| bougie(200.0 - i as f64 * 2.0))
            .collect()
    }

    #[test]
    fn classe_actif_crypto() {
        assert_eq!(classe_actif("BTC"), "crypto");
        assert_eq!(classe_actif("ETH"), "crypto");
        assert_eq!(classe_actif("eth"), "crypto"); // insensible à la casse
    }

    #[test]
    fn classe_actif_metaux_pas_crypto() {
        // XAUUSD est routé vers Binance (is_crypto=true côté data source),
        // mais sa CLASSE de marché est "metaux".
        assert_eq!(classe_actif("XAUUSD"), "metaux");
        assert_eq!(classe_actif("XAGUSD"), "metaux");
    }

    #[test]
    fn classe_actif_forex_et_indices() {
        assert_eq!(classe_actif("EURUSD"), "forex");
        assert_eq!(classe_actif("GBPJPY"), "forex");
        assert_eq!(classe_actif("DAX"), "indices");
        assert_eq!(classe_actif("NAS100"), "indices");
        assert_eq!(classe_actif("SP500"), "indices");
    }

    #[test]
    fn score_dans_intervalle_0_100() {
        let input = vec![("BTC".to_string(), tendance_haussiere(30))];
        let scores = calculer_sentiment_technique(&input);
        let s = scores.get("BTC").unwrap();
        assert!(*s >= 0.0 && *s <= 100.0, "score BTC hors bornes: {}", s);
    }

    #[test]
    fn tendance_haussiere_donne_score_eleve() {
        let input = vec![("BTC".to_string(), tendance_haussiere(30))];
        let scores = calculer_sentiment_technique(&input);
        let s = scores["BTC"];
        // Hausse régulière → RSI14 élevé + prix > MA20 → score > 60.
        assert!(s > 60.0, "tendance haussière devrait donner score élevé, eu {}", s);
    }

    #[test]
    fn tendance_baissiere_donne_score_bas() {
        let input = vec![("EURUSD".to_string(), tendance_baissiere(30))];
        let scores = calculer_sentiment_technique(&input);
        let s = scores["EURUSD"];
        assert!(s < 40.0, "tendance baissière devrait donner score bas, eu {}", s);
    }

    #[test]
    fn actif_trop_court_ignore() {
        let input = vec![("BTC".to_string(), tendance_haussiere(10))];
        let scores = calculer_sentiment_technique(&input);
        assert!(!scores.contains_key("BTC"), "actif < 20 bougies ignoré");
    }

    #[test]
    fn agreg_par_classe_moyenne_correcte() {
        let mut scores = HashMap::new();
        scores.insert("BTC".to_string(), 70.0);
        scores.insert("ETH".to_string(), 50.0); // crypto moyenne = 60
        scores.insert("EURUSD".to_string(), 40.0); // forex = 40
        let agg = agreg_par_classe(&scores);
        assert!((agg.crypto.unwrap() - 60.0).abs() < 1e-9);
        assert!((agg.forex.unwrap() - 40.0).abs() < 1e-9);
        assert!(agg.metaux.is_none());
        assert!(agg.indices.is_none());
        // global = moyenne des 2 classes disponibles = (60+40)/2 = 50
        assert!((agg.global.unwrap() - 50.0).abs() < 1e-9);
    }

    #[test]
    fn agreg_vide_donne_none() {
        let scores = HashMap::new();
        let agg = agreg_par_classe(&scores);
        assert!(agg.global.is_none());
        assert!(agg.crypto.is_none());
    }
}
