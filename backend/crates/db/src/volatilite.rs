use crate::Database;
use common::{Asset, Result, Timeframe, TradingError};
use serde::Serialize;
use smartcore::cluster::kmeans::{KMeans, KMeansParameters};
use smartcore::linalg::basic::matrix::DenseMatrix;
use sqlx::Row;

#[derive(Debug, Clone, Serialize)]
pub struct PatternHoraire {
    pub heure: u8,
    pub jour_semaine: u8, // 0=dim, 1=lun, ..., 6=sam (SQLite strftime %w)
    pub atr_moyen: f64,
    pub nb_points: i64,
    pub cluster: u8, // 0=calme, 1=modéré, 2=élevé, 3=extrême
}

#[derive(Debug, Clone, Serialize)]
pub struct ReponsePatternsVolatilite {
    pub patterns: Vec<PatternHoraire>,
    pub seuil_straddle_calibre: f64,
    pub nb_points_total: i64,
    pub asset: String,
    pub timeframe: String,
}

/// Classification par k-means (k=4) : 0=calme, 1=modéré, 2=élevé, 3=extrême.
/// Fallback automatique vers quartiles si k-means échoue (trop peu de données, etc.).
fn assigner_clusters(mut patterns: Vec<PatternHoraire>) -> Vec<PatternHoraire> {
    let n = patterns.len();
    let k = 4usize.min(n);
    if k == 0 {
        return patterns;
    }

    let data: Vec<Vec<f64>> = patterns.iter().map(|p| vec![p.atr_moyen]).collect();
    let x = DenseMatrix::from_2d_vec(&data);

    let labels: Vec<usize> =
        match KMeans::fit(&x, KMeansParameters::default().with_k(k)) {
            Ok(km) => match km.predict(&x) {
                Ok(l) => l,
                Err(_) => return assigner_clusters_quartiles(patterns),
            },
            Err(_) => return assigner_clusters_quartiles(patterns),
        };

    // Calcul du centre (ATR moyen) de chaque cluster pour trier 0=calme → 3=extrême
    let mut centres: Vec<(usize, f64)> = (0..k)
        .map(|c| {
            let vals: Vec<f64> = labels
                .iter()
                .zip(patterns.iter())
                .filter(|(l, _)| **l == c)
                .map(|(_, p)| p.atr_moyen)
                .collect();
            let moy = if vals.is_empty() {
                0.0
            } else {
                vals.iter().sum::<f64>() / vals.len() as f64
            };
            (c, moy)
        })
        .collect();
    centres.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    // Table de remapping : label kmeans → rang trié (0=calme, 3=extrême)
    let mut rang = vec![0u8; k];
    for (rang_trie, (label_orig, _)) in centres.iter().enumerate() {
        rang[*label_orig] = rang_trie as u8;
    }

    for (p, l) in patterns.iter_mut().zip(labels.iter()) {
        p.cluster = rang[*l];
    }
    patterns
}

/// Fallback : classification par quartiles (comportement original)
fn assigner_clusters_quartiles(mut patterns: Vec<PatternHoraire>) -> Vec<PatternHoraire> {
    let mut valeurs: Vec<f64> = patterns.iter().map(|p| p.atr_moyen).collect();
    valeurs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = valeurs.len();
    if n == 0 {
        return patterns;
    }
    let q25 = valeurs[n * 25 / 100];
    let q50 = valeurs[n * 50 / 100];
    let q75 = valeurs[n * 75 / 100];
    for p in &mut patterns {
        p.cluster = if p.atr_moyen < q25 {
            0
        } else if p.atr_moyen < q50 {
            1
        } else if p.atr_moyen < q75 {
            2
        } else {
            3
        };
    }
    patterns
}

/// Seuil Straddle calibré = percentile 85 des ATR moyens horaires
fn calculer_seuil_straddle(patterns: &[PatternHoraire]) -> f64 {
    if patterns.is_empty() {
        return 0.0;
    }
    let mut valeurs: Vec<f64> = patterns.iter().map(|p| p.atr_moyen).collect();
    valeurs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let idx = (valeurs.len() * 85 / 100).min(valeurs.len() - 1);
    valeurs[idx]
}

impl Database {
    /// Agrège les bouges par (heure_utc, jour_semaine) et calcule l'ATR moyen (high-low proxy).
    /// Retourne les patterns classifiés en 4 clusters + le seuil Straddle calibré (P85).
    /// `mois` : fenêtre glissante (ex: 12 = 12 derniers mois).
    pub async fn obtenir_patterns_horaires(
        &self,
        asset: &Asset,
        timeframe: &Timeframe,
        mois: i64,
    ) -> Result<ReponsePatternsVolatilite> {
        let seuil_ts: i64 = mois * 30 * 86_400;
        let rows = sqlx::query(
            r#"
            SELECT
                CAST(strftime('%H', datetime(timestamp, 'unixepoch')) AS INTEGER) as heure,
                CAST(strftime('%w', datetime(timestamp, 'unixepoch')) AS INTEGER) as jour_semaine,
                AVG(high - low) as atr_moyen,
                COUNT(*) as nb_points
            FROM bougies
            WHERE asset = ? AND timeframe = ?
              AND timestamp > CAST(strftime('%s', 'now') AS INTEGER) - ?
            GROUP BY heure, jour_semaine
            ORDER BY heure, jour_semaine
            "#,
        )
        .bind(asset.as_str())
        .bind(timeframe.as_str())
        .bind(seuil_ts)
        .fetch_all(self.pool())
        .await
        .map_err(|e| TradingError::Database(e.to_string()))?;

        let patterns_bruts: Vec<PatternHoraire> = rows
            .iter()
            .map(|row| PatternHoraire {
                heure: row.get::<i64, _>("heure") as u8,
                jour_semaine: row.get::<i64, _>("jour_semaine") as u8,
                atr_moyen: row.get("atr_moyen"),
                nb_points: row.get("nb_points"),
                cluster: 0,
            })
            .collect();

        let nb_points_total: i64 = patterns_bruts.iter().map(|p| p.nb_points).sum();
        let seuil_straddle_calibre = calculer_seuil_straddle(&patterns_bruts);
        let patterns = assigner_clusters(patterns_bruts);

        Ok(ReponsePatternsVolatilite {
            patterns,
            seuil_straddle_calibre,
            nb_points_total,
            asset: asset.as_str().to_string(),
            timeframe: timeframe.as_str().to_string(),
        })
    }
}
