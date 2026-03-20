use actix_web::{web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};

use crate::state::AppState;
use crate::utils::{parse_asset, parse_timeframe};
use data::{
    providers::{BinanceProvider, IbGatewayProvider},
    DataProvider,
};
use indicators::calculer_ema;

// ─── Query params ─────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct TendanceQuery {
    pub asset: String,
    /// Période EMA rapide (défaut : 9)
    pub ema_rapide: Option<usize>,
    /// Période EMA lente (défaut : 21)
    pub ema_lente: Option<usize>,
    /// Mode de calcul: "bougie_cloturee" (défaut) ou "bougie_en_cours"
    pub mode_calcul: Option<String>,
}

#[derive(Serialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum ModeCalcul {
    BougieCloturee,
    BougieEnCours,
}

impl ModeCalcul {
    fn depuis_query(valeur: Option<&str>) -> Self {
        match valeur {
            Some("bougie_en_cours") => Self::BougieEnCours,
            _ => Self::BougieCloturee,
        }
    }

    fn index_cible(self, longueur: usize) -> Option<usize> {
        if longueur == 0 {
            return None;
        }
        match self {
            Self::BougieEnCours => Some(longueur - 1),
            Self::BougieCloturee => {
                if longueur >= 2 {
                    Some(longueur - 2)
                } else {
                    None
                }
            }
        }
    }
}

// ─── Réponse ──────────────────────────────────────────────────────────────────

#[derive(Serialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Haussier,
    Baissier,
}

#[derive(Serialize)]
pub struct LigneTendance {
    pub tf: String,
    pub tendance: Option<Direction>,
    /// Valeur de l'EMA rapide (ex. EMA9)
    pub valeur_ema_rapide: Option<f64>,
    /// Valeur de l'EMA lente (ex. EMA21)
    pub valeur_ema_lente: Option<f64>,
}

#[derive(Serialize)]
pub struct ReponseTendanceMultiTf {
    pub asset: String,
    pub ema_rapide: usize,
    pub ema_lente: usize,
    pub mode_calcul: ModeCalcul,
    pub lignes: Vec<LigneTendance>,
}

// ─── Timeframes analysés ──────────────────────────────────────────────────────

const TIMEFRAMES_ANALYSE: &[(&str, u32)] = &[
    ("1m", 1),
    ("5m", 10),
    ("15m", 21),
    ("30m", 35),
    ("1H", 42),
    ("4H", 60),
    ("1D", 100),
];

/// Mappe label affiché → code Timeframe DB
fn label_vers_tf(label: &str) -> &'static str {
    match label {
        "1m" => "M1",
        "5m" => "M5",
        "15m" => "M15",
        "30m" => "M30",
        "1H" => "H1",
        "4H" => "H4",
        "1D" => "D1",
        _ => "M15",
    }
}

// ─── Handler ──────────────────────────────────────────────────────────────────

/// GET /api/tendance/multi-tf?asset=BTC&ema_rapide=9&ema_lente=21
///
/// Retourne la direction EMA crossover (Haussier / Baissier) pour chaque
/// timeframe. Logique : EMA rapide > EMA lente → Haussier, sinon Baissier.
pub async fn tendance_multi_tf(
    state: web::Data<AppState>,
    query: web::Query<TendanceQuery>,
) -> impl Responder {
    let asset = match parse_asset(&query.asset) {
        Some(a) => a,
        None => {
            return HttpResponse::BadRequest()
                .json(serde_json::json!({ "erreur": "Asset non supporté" }));
        }
    };

    let ema_rapide = query.ema_rapide.unwrap_or(9).max(1);
    let ema_lente = query.ema_lente.unwrap_or(21).max(2);
    let mode_calcul = ModeCalcul::depuis_query(query.mode_calcul.as_deref());

    // L'EMA a besoin d'au moins ema_lente bougies pour converger.
    // On prend un historique généreux pour que la direction soit stable.
    let limit_bougies = (ema_lente as i64 * 30).max(500);

    let mut lignes: Vec<LigneTendance> = Vec::with_capacity(TIMEFRAMES_ANALYSE.len());

    for &(label, _) in TIMEFRAMES_ANALYSE {
        let tf_code = label_vers_tf(label);
        let tf = parse_timeframe(tf_code);

        // 1. Essayer la DB
        let bougies_db = state
            .db
            .obtenir_bougies(&asset, &tf, limit_bougies)
            .await
            .unwrap_or_default();

        // 2. Si DB insuffisante → fetch Binance (ou IB pour les non-crypto)
        let bougies: Vec<common::Candle> = if bougies_db.len() >= limit_bougies as usize {
            bougies_db
        } else {
            let resultat = match &asset {
                common::Asset::BTC | common::Asset::ETH => {
                    BinanceProvider
                        .fetch_candles(asset.clone(), tf, limit_bougies as usize)
                        .await
                }
                _ => {
                    IbGatewayProvider::new(state.ib_port, state.ib_client_id)
                        .fetch_candles(asset.clone(), tf, limit_bougies as usize)
                        .await
                }
            };
            match resultat {
                Ok(b) => {
                    if let Err(e) = state.db.inserer_bougies(&asset, &tf, &b).await {
                        tracing::warn!(
                            "Tendance multi-TF — cache {} {}: {}",
                            label,
                            query.asset,
                            e
                        );
                    }
                    b
                }
                Err(e) => {
                    tracing::warn!("Tendance multi-TF — fetch {} {}: {}", label, query.asset, e);
                    if !bougies_db.is_empty() {
                        bougies_db
                    } else {
                        lignes.push(LigneTendance {
                            tf: label.to_string(),
                            tendance: None,
                            valeur_ema_rapide: None,
                            valeur_ema_lente: None,
                        });
                        continue;
                    }
                }
            }
        };

        if bougies.len() < ema_lente {
            lignes.push(LigneTendance {
                tf: label.to_string(),
                tendance: None,
                valeur_ema_rapide: None,
                valeur_ema_lente: None,
            });
            continue;
        }

        let ema9 = calculer_ema(&bougies, ema_rapide);
        let ema21 = calculer_ema(&bougies, ema_lente);

        let index_cible = match mode_calcul.index_cible(bougies.len()) {
            Some(i) => i,
            None => {
                lignes.push(LigneTendance {
                    tf: label.to_string(),
                    tendance: None,
                    valeur_ema_rapide: None,
                    valeur_ema_lente: None,
                });
                continue;
            }
        };

        let dernier_ema_rapide = ema9.get(index_cible).copied().filter(|v| v.is_finite());
        let dernier_ema_lente = ema21.get(index_cible).copied().filter(|v| v.is_finite());

        let direction = match (dernier_ema_rapide, dernier_ema_lente) {
            (Some(rapide), Some(lente)) if rapide > lente => Some(Direction::Haussier),
            (Some(rapide), Some(lente)) if rapide < lente => Some(Direction::Baissier),
            _ => None,
        };

        lignes.push(LigneTendance {
            tf: label.to_string(),
            tendance: direction,
            valeur_ema_rapide: dernier_ema_rapide,
            valeur_ema_lente: dernier_ema_lente,
        });
    }

    HttpResponse::Ok().json(ReponseTendanceMultiTf {
        asset: query.asset.to_uppercase(),
        ema_rapide,
        ema_lente,
        mode_calcul,
        lignes,
    })
}
