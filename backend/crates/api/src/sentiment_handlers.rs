//! Référence veille du bloc Sentiment — clôtures figées, zéro flux tendu.
//!
//! Décision propriétaire (2026-08-18) : le bloc Sentiment de Marche affiche
//! exclusivement des clôtures de la veille, comme les jauges composites.
//! Un worker (cycle 30 min, partagé avec le composite) fige les clôtures :
//!   - Yahoo Finance `interval=1d` : indices US/EU, matières, VIX — seules
//!     les barres CLÔTURÉES sont lues, jamais la séance en cours ;
//!   - DB locale D1 (Bybit) : Bitcoin, Ethereum — même source que l'app.
//! `GET /api/sentiment/marche` ne fait plus aucun fetch externe : il sert la
//! dernière référence figée (idempotent, `INSERT OR REPLACE`).

use actix_web::{web, HttpResponse, Responder};
use chrono::{DateTime, Utc};
use common::{Asset, Timeframe};
use futures_util::future::join_all;
use serde::Serialize;

#[path = "sentiment/mod.rs"]
mod sentiment;
use sentiment::yahoo_live;

use crate::state::AppState;
use db::Database;

#[derive(Serialize, Clone)]
pub struct EntiteSentiment {
    pub nom: String,
    pub prix: f64,
    /// Variation de la SÉANCE EN COURS (live, colonne « Jour »).
    pub variation_pct: f64,
    /// Variation de la VEILLE clôturée (figée, colonne « Veille ») —
    /// décision propriétaire 2026-08-19 : deux colonnes par entité.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variation_veille: Option<f64>,
}

#[derive(Serialize)]
pub struct SentimentMarche {
    pub date: String,
    /// Date de la référence figée (colonne « Veille »).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_veille: Option<String>,
    pub usa: Vec<EntiteSentiment>,
    pub europe: Vec<EntiteSentiment>,
    pub matieres_premieres: Vec<EntiteSentiment>,
    pub cryptos: Vec<EntiteSentiment>,
    pub vix: Option<f64>,
}

/// Clôture figée : une entité, un jour de clôture (le vendredi pour les
/// indices le lundi matin, la veille pour les cryptos 24/7).
#[derive(Clone)]
struct ClotureVeille {
    date: String,
    entite: &'static str,
    groupe: &'static str,
    prix: f64,
    variation_pct: f64,
}

/// Ligne lue en DB (mêmes champs, `String` désérialisée).
#[derive(Clone)]
struct LigneVeille {
    date: String,
    entite: String,
    prix: f64,
    variation_pct: f64,
}

/// Sources Yahoo : (symbole URL-encodé, nom affiché, groupe d'affichage).
const SOURCES_YAHOO: &[(&str, &str, &str)] = &[
    ("%5EGSPC", "S&P500", "usa"),
    ("%5EIXIC", "Nasdaq", "usa"),
    ("%5EDJI", "Dow Jones", "usa"),
    ("%5EN100", "Euronext 100", "europe"),
    ("%5EGDAXI", "Dax", "europe"),
    ("%5EFCHI", "Cac 40", "europe"),
    ("GC%3DF", "Or", "matieres_premieres"),
    ("SI%3DF", "Argent", "matieres_premieres"),
    ("CL%3DF", "Pétrole", "matieres_premieres"),
    ("ZC%3DF", "Agriculture", "matieres_premieres"),
    ("%5EVIX", "VIX", "vix"),
];

/// Sources DB locale D1 (Bybit) — les cryptos, comme le reste de l'app.
/// ETH retiré du pipeline le 2026-08-18 (décision propriétaire : BTC seul).
const SOURCES_DB: &[(&str, &str)] = &[("BTC", "Bitcoin")];

/// Ordre d'affichage des groupes (celui du bloc Sentiment).
const ORDRE_AFFICHAGE: &[&[&str]] = &[
    &["S&P500", "Nasdaq", "Dow Jones"],
    &["Euronext 100", "Dax", "Cac 40"],
    &["Or", "Argent", "Pétrole", "Agriculture"],
    &["Bitcoin", "Ethereum"],
];

// ── Extraction pure (testable) ───────────────────────────────────────────────

/// Extrait les 2 dernières barres CLÔTURÉES (avant le jour courant) du JSON
/// Yahoo v8 chart. Retourne `(date_clôture, dernier_close, avant-dernier_close)`
/// — `None` si moins de 2 barres clôturées disponibles.
fn dernieres_clotures_yahoo(
    v: &serde_json::Value,
    minuit_utc_ms: i64,
) -> Option<(String, f64, f64)> {
    let result = v["chart"]["result"].as_array()?.first()?;
    let timestamps = result["timestamp"].as_array()?;
    let closes = result["indicators"]["quote"][0]["close"].as_array()?;

    let mut barres: Vec<(i64, f64)> = timestamps
        .iter()
        .zip(closes.iter())
        .filter_map(|(ts, c)| {
            let ts = ts.as_i64()? * 1000;
            let close = c.as_f64()?;
            (ts < minuit_utc_ms).then_some((ts, close))
        })
        .collect();
    if barres.len() < 2 {
        return None;
    }
    let (ts_derniere, derniere) = barres.pop()?;
    let (_, precedente) = barres.pop()?;
    let date = DateTime::from_timestamp_millis(ts_derniere)?
        .format("%Y-%m-%d")
        .to_string();
    Some((date, derniere, precedente))
}

/// Condense les lignes triées par date DESC en : (date de référence, une
/// ligne par entité). La date de référence est la plus fréquente — lundi
/// matin : vendredi (8 entités marchés traditionnels) plutôt que dimanche
/// (2 cryptos). Égalité → la plus récente.
fn condenser_lignes(lignes: &[LigneVeille]) -> (String, Vec<LigneVeille>) {
    let mut vues: Vec<&LigneVeille> = Vec::new();
    for l in lignes {
        if !vues.iter().any(|v| v.entite == l.entite) {
            vues.push(l);
        }
    }
    let mut comptages: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for l in &vues {
        *comptages.entry(l.date.as_str()).or_insert(0) += 1;
    }
    let date_ref = comptages
        .into_iter()
        .max_by_key(|(d, n)| (*n, *d))
        .map(|(d, _)| d.to_string())
        .unwrap_or_default();
    (date_ref, vues.into_iter().cloned().collect())
}

// ── Collecte ─────────────────────────────────────────────────────────────────

/// Minuit UTC du jour courant en ms — frontière « barre clôturée ».
fn minuit_utc_ms() -> i64 {
    let ts = Utc::now().timestamp();
    (ts - ts.rem_euclid(86_400)) * 1000
}

async fn cloture_yahoo(
    client: &reqwest::Client,
    symbole: &str,
    nom: &'static str,
    groupe: &'static str,
    minuit_ms: i64,
) -> Option<ClotureVeille> {
    let url = format!(
        "https://query2.finance.yahoo.com/v8/finance/chart/{}?interval=1d&range=7d",
        symbole
    );
    let raw: serde_json::Value = client
        .get(&url)
        .header("Accept", "application/json")
        .header("User-Agent", "Mozilla/5.0 (X11; Linux x86_64)")
        .send()
        .await
        .ok()?
        .json()
        .await
        .ok()?;
    let (date, prix, precedente) = dernieres_clotures_yahoo(&raw, minuit_ms)?;
    let variation = if precedente != 0.0 {
        (prix - precedente) / precedente * 100.0
    } else {
        0.0
    };
    Some(ClotureVeille {
        date,
        entite: nom,
        groupe,
        prix,
        variation_pct: (variation * 100.0).round() / 100.0,
    })
}

async fn cloture_db(db: &Database, asset_str: &str, nom: &'static str) -> Option<ClotureVeille> {
    let asset = Asset::try_from(asset_str).ok()?;
    let bougies = db.obtenir_bougies(&asset, &Timeframe::D1, 5).await.ok()?;
    let minuit = minuit_utc_ms();
    let mut barres: Vec<&common::Candle> = bougies
        .iter()
        .filter(|b| b.timestamp.timestamp_millis() < minuit)
        .collect();
    barres.sort_by_key(|b| b.timestamp);
    if barres.len() < 2 {
        return None;
    }
    let veille = barres[barres.len() - 1];
    let avant = barres[barres.len() - 2];
    // Garde d'ancienneté : une DB D1 figée (ex: ETH arrêté en mars) ne doit
    // JAMAIS être servie comme « référence veille » — dégradation silencieuse.
    if trop_ancienne(veille.timestamp, Utc::now()) {
        tracing::warn!(
            "cloture_db {nom} : bougie D1 du {} périmée — entité ignorée",
            veille.timestamp.format("%Y-%m-%d")
        );
        return None;
    }
    let variation = if avant.close != 0.0 {
        (veille.close - avant.close) / avant.close * 100.0
    } else {
        0.0
    };
    Some(ClotureVeille {
        date: veille.timestamp.format("%Y-%m-%d").to_string(),
        entite: nom,
        groupe: "cryptos",
        prix: veille.close,
        variation_pct: (variation * 100.0).round() / 100.0,
    })
}

/// Une clôture servable a moins de 4 jours (week-end + férié = 3 jours sans
/// marché ; au-delà, la source est considérée figée). Partagée avec le
/// composite (`calculer_composite` skippe les assets D1 périmés).
pub(crate) fn trop_ancienne(cloture: chrono::DateTime<Utc>, maintenant: chrono::DateTime<Utc>) -> bool {
    (maintenant - cloture).num_days() > 4
}

/// Collecte les clôtures de la veille et les fige en DB. Idempotent
/// (`INSERT OR REPLACE`) : chaque cycle réécrit les mêmes valeurs. Retourne
/// le nombre d'entités figées. Appelé par le worker 30 min et en one-shot
/// par le GET si la table est vide (premier lancement).
pub async fn figer_veille_marche(db: &Database) -> usize {
    let client = &*crate::http_client::HTTP_CLIENT;
    let minuit_ms = minuit_utc_ms();

    // 11 fetchs Yahoo en parallèle (dégradation silencieuse par source).
    let futurs: Vec<_> = SOURCES_YAHOO
        .iter()
        .map(|(symbole, nom, groupe)| {
            cloture_yahoo(client, symbole, nom, groupe, minuit_ms)
        })
        .collect();
    let mut clotures: Vec<ClotureVeille> = join_all(futurs)
        .await
        .into_iter()
        .flatten()
        .collect();

    for (asset, nom) in SOURCES_DB {
        if let Some(c) = cloture_db(db, asset, nom).await {
            clotures.push(c);
        }
    }

    let mut figees = 0;
    for c in &clotures {
        let res = sqlx::query(
            "INSERT OR REPLACE INTO sentiment_marche_veille
             (date, entite, groupe, prix, variation_pct) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&c.date)
        .bind(c.entite)
        .bind(c.groupe)
        .bind(c.prix)
        .bind(c.variation_pct)
        .execute(db.pool())
        .await;
        match res {
            Ok(_) => figees += 1,
            Err(e) => tracing::warn!("figer veille {} : {}", c.entite, e),
        }
    }
    figees
}

/// Dernière référence figée avant aujourd'hui : `(date_référence, lignes)`.
async fn lire_veille(db: &Database) -> Option<(String, Vec<LigneVeille>)> {
    let aujourdhui = Utc::now().format("%Y-%m-%d").to_string();
    let lignes = sqlx::query_as::<_, (String, String, String, f64, f64)>(
        "SELECT date, entite, groupe, prix, variation_pct
         FROM sentiment_marche_veille WHERE date < ? ORDER BY date DESC",
    )
    .bind(&aujourdhui)
    .fetch_all(db.pool())
    .await
    .ok()?;
    if lignes.is_empty() {
        return None;
    }
    let lignes: Vec<LigneVeille> = lignes
        .into_iter()
        .map(|(date, entite, _groupe, prix, variation_pct)| LigneVeille {
            date,
            entite,
            prix,
            variation_pct,
        })
        .collect();
    Some(condenser_lignes(&lignes))
}

// ── Handler ──────────────────────────────────────────────────────────────────

/// Bitcoin live depuis NOTRE série (DB Bybit spot) : dernier prix connu (M1
/// la plus fraîche) vs close D1 clôturée d'hier. Jamais une autre place —
/// une seule série de prix par asset (leçon L10).
async fn btc_live_db(db: &Database) -> Option<EntiteSentiment> {
    use common::{Asset, Timeframe};
    let asset = Asset::try_from("BTC").ok()?;
    // Référence : dernière bougie D1 CLÔTURÉE (avant minuit UTC courant).
    let d1: Vec<common::Candle> = db
        .obtenir_bougies(&asset, &Timeframe::D1, 5)
        .await
        .ok()?;
    let minuit = minuit_utc_ms() / 1000;
    let ref_close = d1
        .iter()
        .filter(|c| c.timestamp.timestamp() < minuit)
        .next_back()?
        .close;
    // Prix courant : dernier close M1 en base (~temps réel à la minute).
    let m1: Vec<common::Candle> = db
        .obtenir_bougies(&asset, &Timeframe::M1, 2)
        .await
        .ok()?;
    let px = m1.last()?.close;
    let variation = if ref_close != 0.0 {
        (px - ref_close) / ref_close * 100.0
    } else {
        0.0
    };
    Some(EntiteSentiment {
        nom: "Bitcoin".to_string(),
        prix: px,
        variation_pct: (variation * 100.0).round() / 100.0,
        variation_veille: None,
    })
}

/// GET /api/sentiment/marche — AMBIANCE DU JOUR (décision propriétaire
/// 2026-08-18, option A) : listes en variation de séance live — Yahoo pour
/// indices/matières/VIX, DB Bybit spot pour Bitcoin. Les JAUGES composites
/// restent en référence veille (endpoint `/composite`, inchangé).
/// Repli : si le live échoue en grande partie, la dernière veille figée est
/// servie (table `sentiment_marche_veille`, datée).
pub async fn get_sentiment_marche(state: web::Data<AppState>) -> impl Responder {
    let client = &*crate::http_client::HTTP_CLIENT;
    let futurs: Vec<_> = SOURCES_YAHOO
        .iter()
        .map(|(symbole, nom, _groupe)| yahoo_live(client, symbole, nom))
        .collect();
    let mut entites: Vec<EntiteSentiment> = join_all(futurs)
        .await
        .into_iter()
        .flatten()
        .collect();
    if let Some(btc) = btc_live_db(&state.db).await {
        entites.push(btc);
    }

    // Repli dégradé : moins de 5 sources live → servir la veille figée
    // (datée) plutôt qu'un bloc à moitié vide.
    if entites.len() < 5 {
        tracing::warn!(
            "sentiment_marche : live partiel ({}/11) — repli sur la veille figée",
            entites.len()
        );
        let (date_ref, lignes) = lire_veille(&state.db)
            .await
            .unwrap_or_else(|| (String::new(), Vec::new()));
        return construire_reponse(
            &entites_repli(&lignes),
            date_ref.clone(),
            &Some((date_ref, lignes)),
        );
    }

    let veille = lire_veille(&state.db).await;
    construire_reponse(&entites, Utc::now().format("%Y-%m-%d").to_string(), &veille)
}

/// Transforme les lignes figées en entités (repli).
fn entites_repli(lignes: &[LigneVeille]) -> Vec<EntiteSentiment> {
    lignes
        .iter()
        .map(|l| EntiteSentiment {
            nom: l.entite.clone(),
            prix: l.prix,
            variation_pct: l.variation_pct,
            variation_veille: Some(l.variation_pct),
        })
        .collect()
}

/// Assemble la réponse dans l'ordre d'affichage canonique.
fn construire_reponse(
    entites: &[EntiteSentiment],
    date: String,
    veille: &Option<(String, Vec<LigneVeille>)>,
) -> HttpResponse {
    // Fusion colonne « Veille » : attache à chaque entité live sa variation
    // figée (J-1 vs J-2) depuis la table de référence.
    let (date_veille, veille_map): (Option<String>, std::collections::HashMap<&str, f64>) =
        match veille {
            Some((d, lignes)) => (
                Some(d.clone()),
                lignes.iter().map(|l| (l.entite.as_str(), l.variation_pct)).collect(),
            ),
            None => (None, std::collections::HashMap::new()),
        };
    let entites: Vec<EntiteSentiment> = entites
        .iter()
        .map(|e| {
            let mut e = e.clone();
            if e.variation_veille.is_none() {
                e.variation_veille = veille_map.get(e.nom.as_str()).copied();
            }
            e
        })
        .collect();
    let chercher = |nom: &str| entites.iter().find(|e| e.nom == nom);
    let entite = |nom: &str| chercher(nom).cloned();

    let groupes = ORDRE_AFFICHAGE
        .iter()
        .map(|noms| noms.iter().filter_map(|n| entite(n)).collect::<Vec<_>>())
        .collect::<Vec<_>>();

    // VIX : UNE SEULE valeur, FIGÉE (décision propriétaire) — la clôture de
    // la veille si disponible, sinon le live en repli.
    let vix_figé: Option<f64> = veille
        .as_ref()
        .and_then(|(_, lignes)| {
            lignes
                .iter()
                .find(|l| l.entite == "VIX")
                .map(|l| (l.prix * 10.0).round() / 10.0)
        })
        .or_else(|| chercher("VIX").map(|l| (l.prix * 10.0).round() / 10.0));

    HttpResponse::Ok().json(SentimentMarche {
        date,
        date_veille,
        usa: groupes[0].clone(),
        europe: groupes[1].clone(),
        matieres_premieres: groupes[2].clone(),
        cryptos: groupes[3].clone(),
        vix: vix_figé,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const JOUR: i64 = 86_400_000;

    fn fixture_yahoo(closes: &[Option<f64>], base_ms: i64) -> serde_json::Value {
        let timestamps: Vec<i64> = (0..closes.len() as i64).map(|i| (base_ms + i * JOUR) / 1000).collect();
        json!({
            "chart": { "result": [{
                "timestamp": timestamps,
                "indicators": { "quote": [{ "close": closes }] }
            }]}
        })
    }

    #[test]
    fn dernieres_clotures_ignore_la_barre_du_jour() {
        // 3 barres : 2 clôturées (J-2, J-1) + la barre du jour courant.
        let base = 1_755_000_000_000_i64; // aligné 00:00 UTC arbitraire
        let minuit_du_jour = base + 2 * JOUR;
        let v = fixture_yahoo(&[Some(100.0), Some(102.0), Some(999.0)], base + 3_600_000);
        let (date, derniere, precedente) =
            dernieres_clotures_yahoo(&v, minuit_du_jour).unwrap();
        assert_eq!(derniere, 102.0);
        assert_eq!(precedente, 100.0);
        let attendu = DateTime::from_timestamp_millis(base + JOUR + 3_600_000)
            .unwrap()
            .format("%Y-%m-%d")
            .to_string();
        assert_eq!(date, attendu);
    }

    #[test]
    fn dernieres_clotures_une_seule_barre_none() {
        let base = 1_755_000_000_000_i64;
        // 2 barres, toutes deux avant la limite → Some.
        let v = fixture_yahoo(&[Some(100.0), Some(101.0)], base);
        assert!(dernieres_clotures_yahoo(&v, base + 3 * JOUR).is_some());
        // La barre du jour est exclue → une seule clôturée → None.
        let v2 = fixture_yahoo(&[Some(100.0), Some(999.0)], base);
        assert!(dernieres_clotures_yahoo(&v2, base + JOUR).is_none());
    }

    #[test]
    fn dernieres_clotures_skip_null() {
        // close null (jour férié) ignorée : les 2 clôturées restantes servent.
        let base = 1_755_000_000_000_i64;
        let v = fixture_yahoo(&[Some(100.0), None, Some(104.0), Some(999.0)], base);
        let (_, derniere, precedente) =
            dernieres_clotures_yahoo(&v, base + 3 * JOUR).unwrap();
        assert_eq!(derniere, 104.0);
        assert_eq!(precedente, 100.0);
    }

    #[test]
    fn condenser_privilegie_la_date_majoritaire() {
        // Lundi matin : 2 entités à dimanche, 3 à vendredi → référence vendredi.
        let lignes = vec![
            ligne("Bitcoin", "2026-08-16"),
            ligne("Ethereum", "2026-08-16"),
            ligne("S&P500", "2026-08-14"),
            ligne("Dax", "2026-08-14"),
            ligne("VIX", "2026-08-14"),
        ];
        let (date_ref, vues) = condenser_lignes(&lignes);
        assert_eq!(date_ref, "2026-08-14");
        assert_eq!(vues.len(), 5);
    }

    #[test]
    fn trop_ancienne_limite_4_jours() {
        let now = DateTime::parse_from_rfc3339("2026-08-18T14:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        // Hier et week-end + férié (4 jours) → servable.
        assert!(!trop_ancienne(now - chrono::Duration::days(1), now));
        assert!(!trop_ancienne(now - chrono::Duration::days(4), now));
        // 5 jours (ex: DB figée depuis mars) → rejetée.
        assert!(trop_ancienne(now - chrono::Duration::days(5), now));
        assert!(trop_ancienne(now - chrono::Duration::days(145), now));
    }

    fn ligne(entite: &str, date: &str) -> LigneVeille {
        LigneVeille {
            entite: entite.to_string(),
            date: date.to_string(),
            prix: 100.0,
            variation_pct: 1.0,
        }
    }
}
