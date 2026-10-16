use actix_web::{web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};

use crate::state::AppState;
use crate::utils::{parse_asset, parse_timeframe};
use data::{providers::BinanceProvider, DataProvider};
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
// NB (owner 15/10) : PAS de W1 — le panneau Kasper s'arrête à D1, notre
// tableau est son miroir exact (7 lignes).

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

// ─── Bougie vivante (espion KAS 15/10) ────────────────────────────────────────

/// Reconstitue la bougie EN COURS du TF depuis les M1 de sa période.
/// Pourquoi : le panneau Kasper évalue ses EMA sur la bougie vivante de
/// chaque TF ; en base, seules les M1 (push EA à la seconde) sont fraîches
/// — les bougies des TF supérieurs n'existent qu'à leur clôture. La
/// période vivante démarre juste après la dernière bougie CLÔTURÉE du TF
/// (grille serveur respectée, DST comprise).
pub(crate) fn composer_vivante(
    dernieres_tf: &[common::Candle],
    m1: &[common::Candle],
    periode_sec: i64,
    maintenant: i64,
) -> Option<(common::Candle, bool)> {
    let derniere = dernieres_tf.last()?;
    if m1.is_empty() {
        return None;
    }
    // Ancrage sur la GRILLE du TF (dérivée de la dernière barre — serveur et
    // DST respectés) : la période vivante est celle qui contient « maintenant.
    // » k = nombre de périodes entières écoulées depuis la dernière barre.
    let derniere_ts = derniere.timestamp.timestamp();
    let k = ((maintenant - derniere_ts) / periode_sec).max(0);
    let debut = derniere_ts + k * periode_sec;
    // k = 0 → la dernière barre EST la vivante (poussée une fois puis figée :
    // le quotidien du broker) → on la REMPLACE ; k ≥ 1 → on APPEND la vivante.
    let remplacer = k == 0;
    let fenetre: Vec<&common::Candle> = m1
        .iter()
        .filter(|c| c.timestamp.timestamp() >= debut && c.timestamp.timestamp() < debut + periode_sec)
        .collect();
    let premiere = *fenetre.first()?;
    let derniere_m1 = *fenetre.last()?;
    let composee = common::Candle {
        timestamp: premiere.timestamp,
        open: premiere.open,
        high: fenetre.iter().map(|c| c.high).fold(f64::MIN, f64::max),
        low: fenetre.iter().map(|c| c.low).fold(f64::MAX, f64::min),
        close: derniere_m1.close,
        volume: fenetre.iter().map(|c| c.volume).sum(),
    };
    Some((composee, remplacer))
}

// ─── Tests : agrégation de la bougie vivante ─────────────────────────────────

#[cfg(test)]
mod tests_vivante {
    use super::*;
    use chrono::TimeZone;

    fn bougie(ts: i64, o: f64, h: f64, l: f64, c: f64) -> common::Candle {
        common::Candle {
            timestamp: chrono::Utc.timestamp_opt(ts, 0).unwrap(),
            open: o, high: h, low: l, close: c, volume: 1.0,
        }
    }

    /// k=1 (clôturée il y a une période) → APPEND : open de la 1ʳᵉ M1,
    /// extrêmes de la fenêtre, close de la dernière M1.
    #[test]
    fn vivante_append_apres_la_derniere_cloturee() {
        let tf_derniere = vec![bougie(0, 100.0, 110.0, 95.0, 105.0)]; // clôturée à t=900
        let m1 = vec![
            bougie(900, 105.0, 106.0, 104.5, 105.5),
            bougie(960, 105.5, 107.5, 105.0, 107.0),
            bougie(1020, 107.0, 107.0, 103.0, 104.0),
            bougie(1080, 104.0, 105.0, 103.5, 104.5),
        ];
        let (v, remplacer) = composer_vivante(&tf_derniere, &m1, 900, 1200).expect("vivante");
        assert!(!remplacer, "k=1 → append");
        assert_eq!(v.timestamp.timestamp(), 900);
        assert_eq!(v.open, 105.0);
        assert_eq!(v.high, 107.5);
        assert_eq!(v.low, 103.0);
        assert_eq!(v.close, 104.5, "close de la DERNIÈRE M1 vivante");
        assert_eq!(v.volume, 4.0);
    }

    /// k=0 (la dernière barre du TF est la vivante figée — cas D1/W1 du
    /// quotidien broker) → REMPLACER, même fenêtre, même horodatage.
    #[test]
    fn vivante_remplace_la_barre_figee() {
        // D1 : dernière barre = la journée en cours, close figée à 4154.
        let tf_derniere = vec![bougie(0, 4150.0, 4160.0, 4145.0, 4154.0)];
        let m1 = vec![
            bougie(60, 4152.0, 4153.0, 4151.0, 4152.5),
            bougie(120, 4152.5, 4159.5, 4152.0, 4159.0),
        ];
        let (v, remplacer) = composer_vivante(&tf_derniere, &m1, 86_400, 7_200).expect("vivante");
        assert!(remplacer, "k=0 → remplacer la barre figée");
        assert_eq!(v.close, 4159.0, "close actualisée au dernier prix M1");
        assert_eq!(v.high, 4159.5);
        assert_eq!(v.open, 4152.0);
    }

    /// k≥2 (clôturées manquantes au milieu — barre figée il y a longtemps) :
    /// la fenêtre s'ancre sur la période CONTENANT maintenant, pas sur la
    /// période juste après la dernière barre.
    #[test]
    fn vivante_s_ancre_sur_la_periode_courante() {
        // H1 : dernière barre 17:00, maintenant 21:02 → vivante = [21:00, 22:00).
        let tf_derniere = vec![bougie(0, 100.0, 110.0, 95.0, 105.0)];
        let m1 = vec![
            bougie(60, 200.0, 201.0, 199.0, 200.5),   // 17:01 — hors fenêtre
            bougie(7560, 300.0, 301.0, 299.0, 300.5), // 21:06 — dans [21:00
            bougie(7620, 300.5, 302.0, 300.0, 301.5), // 21:12
        ];
        let (v, remplacer) = composer_vivante(&tf_derniere, &m1, 3_600, 7_320).expect("vivante");
        assert!(!remplacer);
        assert_eq!(v.timestamp.timestamp(), 7560, "fenêtre ancrée sur 21:00");
        assert_eq!(v.open, 300.0, "la M1 de 17:01 est ignorée");
    }

    /// Fenêtre M1 vide (collecte en retard) → None, comportement inchangé.
    #[test]
    fn fenetre_m1_vide_aucune_synthese() {
        let tf_derniere = vec![bougie(0, 100.0, 110.0, 95.0, 105.0)];
        let m1 = vec![bougie(60, 105.0, 106.0, 104.0, 105.5)]; // hors fenêtre
        assert!(composer_vivante(&tf_derniere, &m1, 900, 1200).is_none());
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

        // 2. DB déjà suffisante POUR L'EMA (≥ ema_lente barres — 21 barres
        //    convergent, `limit_bougies` n'est qu'une fenêtre généreuse) → on
        //    n'interroge jamais le provider. Sinon les indices Axi (GER40,
        //    NAS100), annoncés « cotables » par l'heuristique Bybit alors que
        //    Bybit ne les cote pas, partaient en GET VIDE à chaque poll —
        //    vidant le calcul (ligne D1 nulle, owner 16/10).
        let mut bougies: Vec<common::Candle> = if bougies_db.len() >= ema_lente {
            bougies_db
        } else {
            let resultat = if asset.est_cotable_bybit() {
                BinanceProvider
                    .fetch_candles(asset.clone(), tf, limit_bougies as usize)
                    .await
            } else {
                // IG : Lightstreamer alimentera le cache
                tracing::debug!(
                    "Tendance multi-TF: cache {} {} insuffisant — LS alimentera",
                    label,
                    query.asset
                );
                Ok(bougies_db.clone())
            };
            match resultat {
                Ok(b) => {
                    if b.is_empty() {
                        // Provider OK mais vide (paire inexistante, ex.
                        // GER40USDT) : la DB garde la main plutôt que de
                        // vider le calcul de ses barres déjà collectées.
                        if bougies_db.is_empty() {
                            lignes.push(LigneTendance {
                                tf: label.to_string(),
                                tendance: None,
                                valeur_ema_rapide: None,
                                valeur_ema_lente: None,
                            });
                            continue;
                        }
                        bougies_db
                    } else {
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

        // Bougie vivante : reconstituée des M1 fraîches (Kasper évalue en
        // temps réel — sans elle, un TF supérieur resterait figé à sa
        // dernière clôture et divergerait près des croisements).
        if matches!(mode_calcul, ModeCalcul::BougieEnCours) && bougies.len() >= 2 {
            let periode_sec = tf.minutes() as i64 * 60;
            // Fenêtre M1 couvrant TOUTE la période vivante (D1 = 1 440,
            // W1 = 10 080 — une fenêtre courte faussait l'open/high/low de
            // la bougie composée, d'où la divergence D1, owner 15/10).
            let profondeur_m1 = (tf.minutes() as i64 + 120).min(10_800);
            let m1 = state
                .db
                .obtenir_bougies(&asset, &common::Timeframe::M1, profondeur_m1)
                .await
                .unwrap_or_default();
            if let Some((vivante, remplacer)) = composer_vivante(
                &bougies,
                &m1,
                periode_sec,
                chrono::Utc::now().timestamp(),
            ) {
                if remplacer {
                    // La dernière barre du TF était la vivante figée (push
                    // unique du quotidien) — l'actualiser aux M1 fraîches.
                    if let Some(derniere) = bougies.last_mut() {
                        *derniere = vivante;
                    }
                } else {
                    bougies.push(vivante);
                }
            }
        }

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
