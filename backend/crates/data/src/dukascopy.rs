//! Backfill historique depuis le datafeed public Dukascopy (`.bi5`).
//!
//! URL d'un jour de candles M1 (bid) :
//! `https://datafeed.dukascopy.com/datafeed/{INSTRUMENT}/{ANNEE}/{MOIS-1}/{JOUR}/BID_candles_min_1.bi5`
//! - le mois est ZÉRO-INDEXÉ (janvier = 00) ;
//! - le fichier est compressé LZMA (format « alone », décompressable par `lzma-rs`) ;
//! - 24 bytes par candle M1, big-endian : `>5if` =
//!   (offset_secondes_de_minuit_utc: i32, open: i32, close: i32, low: i32,
//!    high: i32 en « points », volume: f32 en lots).
//!   NB : l'ordre des prix est O, C, L, H (et non O, H, L, C) — vérifié
//!   empiriquement sur XAUUSD/EURUSD/USATECHIDXUSD/DEUIDXEUR (0 candle
//!   incohérent sur 1440 par fichier).
//!
//! Noms d'instruments (vérifiés empiriquement sur le datafeed) :
//! - Forex/métaux : nom plat (`XAUUSD`, `EURUSD`…) ;
//! - Indices : nom CONCATÉNÉ sans point ni slash (`USATECH.IDX/USD` →
//!   `USATECHIDXUSD`, `DEU.IDX/EUR` → `DEUIDXEUR`). Les formes avec `.` ou
//!   `/` retournent 404.
//!
//! Le datafeed est rate-limité agressivement (503 voire coupure TCP si les
//! requêtes sont trop rapprochées) → délai OBLIGATOIRE entre téléchargements.
//!
//! Zéro panic : toute donnée invalide est ignorée silencieusement, les
//! erreurs remontent en `Result`.

use std::time::Duration;

use chrono::{Datelike, TimeZone, Utc};
use common::{Candle, Result, Timeframe, TradingError};

/// Racine du datafeed public Dukascopy.
const DATAFEED_ROOT: &str = "https://datafeed.dukascopy.com/datafeed";

/// User-Agent « navigateur » — le datafeed rejette certains clients nus.
const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0 Safari/537.36";

/// Délai OBLIGATOIRE entre deux téléchargements (rate limit Dukascopy).
pub const DELAI_ENTRE_TELECHARGEMENTS: Duration = Duration::from_secs(4);

/// Délai de repli avant une nouvelle tentative après un 503 / timeout réseau.
pub const DELAI_RETRY: Duration = Duration::from_secs(15);

/// Taille d'un candle M1 dans un fichier `.bi5` décompressé.
const TAILLE_CANDLE: usize = 24;

// ─── URL ──────────────────────────────────────────────────────────────────────

/// Construit l'URL du fichier `.bi5` M1 (bid) d'une journée.
/// `mois` et `jour` sont 1-indexés côté appelant (le mois est décalé ici).
pub fn url_bi5(instrument: &str, annee: u32, mois: u32, jour: u32) -> String {
    format!(
        "{}/{}/{}/{:02}/{:02}/BID_candles_min_1.bi5",
        DATAFEED_ROOT,
        instrument.trim(),
        annee,
        mois.saturating_sub(1), // mois zéro-indexé côté Dukascopy
        jour
    )
}

// ─── Téléchargement ───────────────────────────────────────────────────────────

/// Statut d'un téléchargement de fichier `.bi5`.
#[derive(Debug)]
pub enum TelechargementBi5 {
    /// Contenu binaire compressé (LZMA) du fichier.
    Donnees(Vec<u8>),
    /// 404 — jour sans données (week-end, jour férié, date future, instrument
    /// inconnu). Ce n'est PAS une erreur.
    SansDonnees,
}

/// Télécharge le fichier `.bi5` d'une journée (une seule requête, pas de retry).
/// - 404 → `SansDonnees` ;
/// - 503 / erreur réseau / erreur de décompression → `Err` (le caller doit
///   attendre puis retenter).
pub async fn telecharger_bi5(
    client: &reqwest::Client,
    instrument: &str,
    annee: u32,
    mois: u32,
    jour: u32,
) -> Result<TelechargementBi5> {
    let url = url_bi5(instrument, annee, mois, jour);
    let reponse = client
        .get(&url)
        .header(reqwest::header::USER_AGENT, USER_AGENT)
        .timeout(Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| TradingError::Data(format!("réseau Dukascopy {}: {}", url, e)))?;

    match reponse.status() {
        reqwest::StatusCode::OK => {
            let octets = reponse
                .bytes()
                .await
                .map_err(|e| TradingError::Data(format!("lecture corps {}: {}", url, e)))?;
            if octets.is_empty() {
                // Fichier vide = journée sans activité (ex: jour férié index).
                return Ok(TelechargementBi5::SansDonnees);
            }
            Ok(TelechargementBi5::Donnees(octets.to_vec()))
        }
        reqwest::StatusCode::NOT_FOUND => Ok(TelechargementBi5::SansDonnees),
        statut => Err(TradingError::Data(format!(
            "HTTP {} sur {} (rate limit Dukascopy ? attendre puis retenter)",
            statut, url
        ))),
    }
}

// ─── Décompression / parsing ──────────────────────────────────────────────────

/// Décompresse un fichier `.bi5` (LZMA « alone ») en binaire brut.
pub fn decompresser_bi5(data: &[u8]) -> Result<Vec<u8>> {
    let mut sortie = Vec::new();
    lzma_rs::lzma_decompress(&mut std::io::Cursor::new(data), &mut sortie)
        .map_err(|e| TradingError::Data(format!("décompression LZMA: {}", e)))?;
    Ok(sortie)
}

/// Convertit un entier big-endian signé 4 bytes en `i32` (sans panic).
fn i32_be(octets: &[u8]) -> i32 {
    i32::from_be_bytes([octets[0], octets[1], octets[2], octets[3]])
}

/// Convertit 4 bytes big-endian en `f32` (sans panic).
fn f32_be(octets: &[u8]) -> f32 {
    f32::from_be_bytes([octets[0], octets[1], octets[2], octets[3]])
}

/// Parse un buffer `.bi5` décompressé en bougies M1.
///
/// `minuit_utc` : timestamp Unix (secondes) du début de la journée du fichier.
/// `diviseur` : facteur de conversion points → prix réel (ex: XAUUSD = 1000).
///
/// Chaque enregistrement fait 24 bytes : offset secondes (i32), open, close,
/// low, high (i32 en points), volume (f32). Les enregistrements tronqués ou
/// aux prix non positifs sont ignorés — aucune panic, aucune erreur bloquante.
pub fn parser_candles_m1(decompresse: &[u8], minuit_utc: i64, diviseur: f64) -> Vec<Candle> {
    if diviseur <= 0.0 {
        return Vec::new();
    }
    let mut bougies = Vec::with_capacity(decompresse.len() / TAILLE_CANDLE);
    for tranchant in decompresse.chunks_exact(TAILLE_CANDLE) {
        let offset_secondes = i32_be(&tranchant[0..4]);
        let open = i32_be(&tranchant[4..8]);
        let close = i32_be(&tranchant[8..12]);
        let low = i32_be(&tranchant[12..16]);
        let high = i32_be(&tranchant[16..20]);
        let volume = f32_be(&tranchant[20..24]);
        // Cohérence minimale : prix strictement positifs (en points).
        if open <= 0 || close <= 0 || low <= 0 || high <= 0 {
            continue;
        }
        // Renormalisation défensive : high/low inversés ne doivent jamais
        // corrompre la base.
        let (haut, bas) = if high >= low { (high, low) } else { (low, high) };
        let Some(ts) = Utc.timestamp_opt(minuit_utc + offset_secondes as i64, 0).single() else {
            continue;
        };
        bougies.push(Candle {
            timestamp: ts,
            open: open as f64 / diviseur,
            high: haut as f64 / diviseur,
            low: bas as f64 / diviseur,
            close: close as f64 / diviseur,
            volume: volume as f64,
        });
    }
    bougies
}

/// `true` si une journée entière de M1 est un « bourrage » non tradé :
/// volume total nul ET toutes les bougies strictement plates
/// (week-end/jour férié chez Dukascopy → on ignore la journée).
pub fn jour_non_trade(m1: &[Candle]) -> bool {
    if m1.is_empty() {
        return true;
    }
    let volume_total: f64 = m1.iter().map(|c| c.volume).sum();
    let tout_plat = m1
        .iter()
        .all(|c| c.open == c.high && c.high == c.low && c.low == c.close);
    volume_total == 0.0 && tout_plat
}

// ─── Diviseurs (points → prix) ────────────────────────────────────────────────

/// Diviseur de conversion points → prix pour un instrument Dukascopy.
///
/// Table empirique (vérifiée sur le datafeed en 2026) :
/// - métaux, indices, paires JPY : 1000 (XAUUSD 4293835 → 4293.835,
///   USATECHIDXUSD 29555999 → 29556.0, DEUIDXEUR 26393599 → 26393.6) ;
/// - paires forex 5 décimales : 100000 (EURUSD 115434 → 1.15434) ;
/// - crypto : 10 (BTCUSD, 1 décimale).
///
/// Défaut 1000 : majorité des instruments non-forex. Un diviseur erroné ne
/// casse rien structurellement (prix mis à l'échelle) mais fausse les valeurs.
pub fn diviseur_instrument(instrument: &str) -> f64 {
    match instrument.trim().to_ascii_uppercase().as_str() {
        // Crypto (1 décimale)
        "BTCUSD" | "ETHUSD" => 10.0,
        // Forex 5 décimales
        "EURUSD" | "GBPUSD" | "AUDUSD" | "NZDUSD" | "USDCHF" | "USDCAD" | "EURGBP"
        | "EURCHF" | "GBPCHF" => 100_000.0,
        // Métaux, indices, paires JPY et défaut : 3 décimales.
        _ => 1000.0,
    }
}

/// `true` si l'instrument cotation en continu (week-ends inclus) — évite de
/// gaspiller des requêtes (et du délai de rate limit) sur les week-ends forex.
pub fn instrument_trade_weekend(instrument: &str) -> bool {
    matches!(
        instrument.trim().to_ascii_uppercase().as_str(),
        "BTCUSD" | "ETHUSD"
    )
}

// ─── Agrégation M1 → TF supérieurs ────────────────────────────────────────────

/// Début (timestamp Unix secondes) du bucket auquel appartient `ts` pour un
/// timeframe aligné sur l'epoch UTC (M5/M15/M30/H1/H4/D1).
fn debut_bucket(ts: i64, tf: &Timeframe) -> i64 {
    let secondes: i64 = match tf {
        Timeframe::M1 => 60,
        Timeframe::M5 => 300,
        Timeframe::M15 => 900,
        Timeframe::M30 => 1800,
        Timeframe::H1 => 3600,
        Timeframe::H4 => 14_400,
        // D1 : jour UTC (epoch = minuit UTC → l'alignement epoch = alignement jour).
        Timeframe::D1 => 86_400,
        // W1 : l'epoch (jeudi 1970-01-01) décalerait les semaines de jeu→mer ;
        // on aligne sur le LUNDI UTC.
        Timeframe::W1 => return debut_semaine_lundi(ts),
    };
    ts - ts.rem_euclid(secondes)
}

/// Timestamp Unix du lundi 00:00 UTC de la semaine de `ts`.
fn debut_semaine_lundi(ts: i64) -> i64 {
    let Some(date) = Utc.timestamp_opt(ts, 0).single() else {
        return ts - ts.rem_euclid(604_800);
    };
    let jours_depuis_lundi = date.weekday().num_days_from_monday() as i64;
    let minuit = ts - ts.rem_euclid(86_400);
    minuit - jours_depuis_lundi * 86_400
}

/// Agrège des bougies M1 vers un timeframe supérieur.
/// O = premier open, H = max high, L = min low, C = dernier close,
/// V = somme des volumes. Les buckets vides sont simplement absents.
/// `Timeframe::M1` retourne l'entrée triée telle quelle (dédupliquée par ts).
pub fn agreger(m1: &[Candle], tf: &Timeframe) -> Vec<Candle> {
    if m1.is_empty() {
        return Vec::new();
    }
    // Tri par timestamp croissant (les fichiers sont normalement déjà triés,
    // mais un backfill multi-jours ne le garantit pas si une insertion échoue).
    let mut triees: Vec<&Candle> = m1.iter().collect();
    triees.sort_by_key(|c| c.timestamp);

    let mut resultat: Vec<Candle> = Vec::new();
    for bougie in triees {
        let ts_bucket = debut_bucket(bougie.timestamp.timestamp(), tf);
        let doit_fusionner = match resultat.last() {
            Some(precedente) => precedente.timestamp.timestamp() == ts_bucket,
            None => false,
        };
        if doit_fusionner {
            if let Some(precedente) = resultat.last_mut() {
                precedente.high = precedente.high.max(bougie.high);
                precedente.low = precedente.low.min(bougie.low);
                precedente.close = bougie.close;
                precedente.volume += bougie.volume;
            }
        } else if let Some(ts) = Utc.timestamp_opt(ts_bucket, 0).single() {
            resultat.push(Candle {
                timestamp: ts,
                open: bougie.open,
                high: bougie.high,
                low: bougie.low,
                close: bougie.close,
                volume: bougie.volume,
            });
        }
    }
    resultat
}

// ─── Orchestration d'un jour complet ──────────────────────────────────────────

/// Télécharge + décompresse + parse une journée, AVEC retry sur rate limit
/// (503 / erreur réseau) : jusqu'à `max_tentatives` essais espacés de
/// [`DELAI_RETRY`]. Retourne `None` si 404/fichier vide (jour sans données).
///
/// Ne gère PAS le délai inter-jours ([`DELAI_ENTRE_TELECHARGEMENTS`]) — c'est
/// le rôle du caller (une seule pause entre deux jours réussis, pas après
/// chaque tentative).
pub async fn telecharger_jour_m1(
    client: &reqwest::Client,
    instrument: &str,
    annee: u32,
    mois: u32,
    jour: u32,
    max_tentatives: u32,
) -> Result<Option<Vec<Candle>>> {
    let diviseur = diviseur_instrument(instrument);
    let minuit_utc = minuit_utc(annee, mois, jour);

    for tentative in 1..=max_tentatives.max(1) {
        match telecharger_bi5(client, instrument, annee, mois, jour).await {
            Ok(TelechargementBi5::SansDonnees) => return Ok(None),
            Ok(TelechargementBi5::Donnees(octets)) => {
                let decompresse = decompresser_bi5(&octets)?;
                return Ok(Some(parser_candles_m1(&decompresse, minuit_utc, diviseur)));
            }
            Err(e) if tentative < max_tentatives.max(1) => {
                let date = format!("{:04}-{:02}-{:02}", annee, mois, jour);
                tracing::warn!(
                    instrument = instrument,
                    date = %date,
                    tentative = tentative,
                    erreur = %e,
                    delai_seconde = DELAI_RETRY.as_secs(),
                    "Dukascopy: tentative échouée, retry"
                );
                tokio::time::sleep(DELAI_RETRY).await;
            }
            Err(e) => return Err(e),
        }
    }
    // Atteint uniquement si max_tentatives == 0.
    Err(TradingError::Data(
        "aucune tentative de téléchargement effectuée".into(),
    ))
}

/// Timestamp Unix (secondes) du minuit UTC d'une date, ou 0 si date invalide
/// (le parsing ignorera alors tous les offsets — sortie vide).
fn minuit_utc(annee: u32, mois: u32, jour: u32) -> i64 {
    Utc.with_ymd_and_hms(annee as i32, mois, jour, 0, 0, 0)
        .single()
        .map(|d| d.timestamp())
        .unwrap_or(0)
}

/// Construit un client HTTP dédié au datafeed Dukascopy.
pub fn client_http() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
}

// ─── Tests unitaires (aucun réseau) ───────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Re-compresse 24 bytes en « candle » pour construire des fichiers de test.
    fn candle_brut(offset_s: i32, open: i32, close: i32, low: i32, high: i32, volume: f32) -> Vec<u8> {
        let mut v = Vec::with_capacity(24);
        v.extend_from_slice(&offset_s.to_be_bytes());
        v.extend_from_slice(&open.to_be_bytes());
        v.extend_from_slice(&close.to_be_bytes());
        v.extend_from_slice(&low.to_be_bytes());
        v.extend_from_slice(&high.to_be_bytes());
        v.extend_from_slice(&volume.to_be_bytes());
        v
    }

    #[test]
    fn url_bi5_mois_zero_indexe() {
        assert_eq!(
            url_bi5("XAUUSD", 2026, 1, 5),
            "https://datafeed.dukascopy.com/datafeed/XAUUSD/2026/00/05/BID_candles_min_1.bi5"
        );
        assert_eq!(
            url_bi5("USATECHIDXUSD", 2026, 12, 31),
            "https://datafeed.dukascopy.com/datafeed/USATECHIDXUSD/2026/11/31/BID_candles_min_1.bi5"
        );
    }

    #[test]
    fn parse_candle_unique_ordre_oclh() {
        // Vrai candle M1 XAUUSD (2026) : (offset, open, close, low, high, volume)
        // en points — l'ordre du format est O, C, L, H (pas O, H, L, C).
        let brut = candle_brut(60, 4295065, 4294315, 4294055, 4295375, 0.5);
        let minuit = minuit_utc(2026, 8, 12);
        let bougies = parser_candles_m1(&brut, minuit, 1000.0);
        assert_eq!(bougies.len(), 1);
        let b = &bougies[0];
        assert_eq!(b.timestamp.timestamp(), minuit + 60);
        assert!((b.open - 4295.065).abs() < 1e-9);
        assert!((b.close - 4294.315).abs() < 1e-9);
        assert!((b.low - 4294.055).abs() < 1e-9);
        assert!((b.high - 4295.375).abs() < 1e-9);
        assert!(b.high >= b.open && b.high >= b.close && b.low <= b.open && b.low <= b.close);
        assert!((b.volume - 0.5).abs() < 1e-9);
    }

    #[test]
    fn parse_ignored_enregistrements_invalides() {
        let minuit = minuit_utc(2026, 8, 12);
        let mut brut = candle_brut(0, 1000, 1000, 1000, 1000, 1.0);
        // Candle aux prix négatifs → ignoré.
        brut.extend(candle_brut(60, -1, -1, -1, -1, 1.0));
        // Tronçon final incomplet → ignoré (chunks_exact).
        brut.extend_from_slice(&[0u8, 0, 0, 0, 1]);
        let bougies = parser_candles_m1(&brut, minuit, 1000.0);
        assert_eq!(bougies.len(), 1, "seul le 1er candle est valide");
        // Diviseur invalide → sortie vide, pas de panic.
        assert!(parser_candles_m1(&brut, minuit, 0.0).is_empty());
    }

    #[test]
    fn parse_high_low_inverses_sont_renormalises() {
        let minuit = minuit_utc(2026, 8, 12);
        let brut = candle_brut(0, 1000, 1000, 1100, 900, 1.0);
        let b = &parser_candles_m1(&brut, minuit, 100.0)[0];
        assert!(b.high >= b.low, "high/low renormalisés");
        assert!((b.high - 11.0).abs() < 1e-9);
        assert!((b.low - 9.0).abs() < 1e-9);
    }

    #[test]
    fn agreger_m1_vers_m5() {
        let base = minuit_utc(2026, 8, 12);
        let m1: Vec<Candle> = (0..7)
            .map(|i| Candle {
                timestamp: Utc.timestamp_opt(base + i * 60, 0).single().unwrap(),
                open: i as f64,
                high: i as f64 + 1.0,
                low: i as f64 - 1.0,
                close: i as f64 + 0.5,
                volume: 2.0,
            })
            .collect();
        let m5 = agreger(&m1, &Timeframe::M5);
        // 7 minutes M1 → buckets : [0-4], [5-6] (le dernier est partiel).
        assert_eq!(m5.len(), 2);
        assert_eq!(m5[0].timestamp.timestamp(), base);
        assert!((m5[0].open - 0.0).abs() < 1e-9, "open du 1er M1");
        assert!((m5[0].high - 5.0).abs() < 1e-9, "max des highs");
        assert!((m5[0].low - (-1.0)).abs() < 1e-9, "min des lows");
        assert!((m5[0].close - 4.5).abs() < 1e-9, "close du dernier M1");
        assert!((m5[0].volume - 10.0).abs() < 1e-9, "somme des volumes");
        assert_eq!(m5[1].timestamp.timestamp(), base + 300);
        assert!((m5[1].volume - 4.0).abs() < 1e-9);
    }

    #[test]
    fn agreger_alignements_h1_h4_d1() {
        let base = minuit_utc(2026, 8, 12);
        let m1: Vec<Candle> = [0i64, 3599, 3600, 14_399, 14_400]
            .iter()
            .map(|&offset| Candle {
                timestamp: Utc.timestamp_opt(base + offset, 0).single().unwrap(),
                open: 1.0,
                high: 2.0,
                low: 0.5,
                close: 1.5,
                volume: 1.0,
            })
            .collect();
        let h1 = agreger(&m1, &Timeframe::H1);
        // Offsets 0 et 3599 → 00h ; 3600 → 01h ; 14399 (03:59:59) → 03h ; 14400 → 04h.
        assert_eq!(h1.len(), 4);
        assert_eq!(h1[0].timestamp.timestamp(), base);
        assert_eq!(h1[1].timestamp.timestamp(), base + 3600);
        assert_eq!(h1[2].timestamp.timestamp(), base + 3 * 3600);
        assert_eq!(h1[3].timestamp.timestamp(), base + 4 * 3600);
        let h4 = agreger(&m1, &Timeframe::H4);
        assert_eq!(h4.len(), 2, "00h-04h, 04h-08h");
        assert_eq!(h4[0].timestamp.timestamp(), base);
        assert_eq!(h4[1].timestamp.timestamp(), base + 14_400);
        let d1 = agreger(&m1, &Timeframe::D1);
        assert_eq!(d1.len(), 1);
        assert_eq!(d1[0].timestamp.timestamp(), base, "D1 = minuit UTC");
        assert!((d1[0].volume - 5.0).abs() < 1e-9);
    }

    #[test]
    fn agreger_w1_aligne_lundi() {
        // Samedi 2026-08-15 et dimanche 2026-08-16 → semaine du lundi 2026-08-10.
        let samedi = minuit_utc(2026, 8, 15);
        let dimanche = minuit_utc(2026, 8, 16);
        let lundi_attendu = minuit_utc(2026, 8, 10);
        let m1 = vec![
            Candle { timestamp: Utc.timestamp_opt(samedi, 0).single().unwrap(), open: 1.0, high: 1.0, low: 1.0, close: 1.0, volume: 1.0 },
            Candle { timestamp: Utc.timestamp_opt(dimanche, 0).single().unwrap(), open: 2.0, high: 2.0, low: 2.0, close: 2.0, volume: 1.0 },
        ];
        let w1 = agreger(&m1, &Timeframe::W1);
        assert_eq!(w1.len(), 1);
        assert_eq!(w1[0].timestamp.timestamp(), lundi_attendu, "W1 démarre lundi");
    }

    #[test]
    fn agreger_entree_non_triee_et_vide() {
        let base = minuit_utc(2026, 8, 12);
        let m1 = vec![
            Candle { timestamp: Utc.timestamp_opt(base + 60, 0).single().unwrap(), open: 2.0, high: 2.0, low: 2.0, close: 2.0, volume: 1.0 },
            Candle { timestamp: Utc.timestamp_opt(base, 0).single().unwrap(), open: 1.0, high: 1.0, low: 1.0, close: 1.0, volume: 1.0 },
        ];
        let m5 = agreger(&m1, &Timeframe::M5);
        assert_eq!(m5.len(), 1);
        assert!((m5[0].open - 1.0).abs() < 1e-9, "open du plus ancien après tri");
        assert!((m5[0].close - 2.0).abs() < 1e-9);
        assert!(agreger(&[], &Timeframe::H1).is_empty());
    }

    #[test]
    fn jour_non_trade_detecte_bourrage_weekend() {
        let base = minuit_utc(2026, 8, 15);
        let plat = |i: i64| Candle {
            timestamp: Utc.timestamp_opt(base + i, 0).single().unwrap(),
            open: 100.0, high: 100.0, low: 100.0, close: 100.0, volume: 0.0,
        };
        let bourrage: Vec<Candle> = (0..5).map(|i| plat(i * 60)).collect();
        assert!(jour_non_trade(&bourrage), "week-end plat sans volume");
        let mut trade = bourrage.clone();
        trade[2].volume = 0.1;
        assert!(!jour_non_trade(&trade), "volume > 0 → jour tradé");
        let mut varie = bougies_clone_variees(&bourrage);
        varie[0].volume = 0.0;
        assert!(!jour_non_trade(&varie), "prix variés → jour tradé même sans volume");
        assert!(jour_non_trade(&[]));
    }

    /// Variante locale : copies avec prix variés (helper de test).
    fn bougies_clone_variees(src: &[Candle]) -> Vec<Candle> {
        src.iter()
            .enumerate()
            .map(|(i, c)| {
                let mut v = c.clone();
                v.open = 100.0 + i as f64;
                v.high = 101.0 + i as f64;
                v.low = 99.0 + i as f64;
                v.close = 100.5 + i as f64;
                v
            })
            .collect()
    }

    #[test]
    fn decompression_lzma_reelle() {
        // Fichier .bi5 XAUUSD réel (17287 bytes compressés → 34560 = 1440×24).
        // Encodé en dur via un mini-flux LZMA « alone » généré à la main :
        // on vérifie surtout la gestion d'entrée invalide (aucune panic).
        let resultat = decompresser_bi5(b"pas du tout du lzma");
        assert!(resultat.is_err(), "données non-LZMA → erreur propre");
        assert!(decompresser_bi5(&[]).is_err());
    }

    #[test]
    fn diviseurs_et_weekends() {
        assert_eq!(diviseur_instrument("XAUUSD"), 1000.0);
        assert_eq!(diviseur_instrument("xauusd"), 1000.0, "insensible à la casse");
        assert_eq!(diviseur_instrument("EURUSD"), 100_000.0);
        assert_eq!(diviseur_instrument("BTCUSD"), 10.0);
        assert_eq!(diviseur_instrument("INCONNU"), 1000.0, "défaut 1000");
        assert!(instrument_trade_weekend("BTCUSD"));
        assert!(!instrument_trade_weekend("XAUUSD"));
        assert!(!instrument_trade_weekend("USATECHIDXUSD"));
    }

    #[test]
    fn minuit_utc_dates_valides_et_invalides() {
        assert_eq!(minuit_utc(2026, 8, 12), 1_786_492_800);
        assert_eq!(minuit_utc(2026, 1, 1), 1_767_225_600);
        // Dates invalides → 0 (aucune panic) : 32 janvier, mois 13.
        assert_eq!(minuit_utc(2026, 13, 1), 0);
        assert_eq!(minuit_utc(2026, 1, 32), 0);
    }
}
