//! Backfill profond Bybit — chantier voie dev n°2 de la ROADMAP.
//!
//! Usage :
//! ```sh
//! cargo run -p api --bin backfill_profond -- --tous [--mois 24] [--delai-ms 200]
//! cargo run -p api --bin backfill_profond -- --asset BTC --tf M15 --mois 24
//! ```
//!
//! Pour chaque couple (asset Bybit actif × TF configuré) :
//! 1. bougie la plus ancienne en DB = curseur de départ
//! 2. pagination descendante (API `end=`, 1000 bougies/page) jusqu'à la
//!    cible (now − N mois) ou jusqu'à épuisement de l'historique Bybit
//! 3. INSERT OR IGNORE (doublons impossibles — clé unique asset×tf×ts)
//!
//! Reprendre après interruption = relancer : le curseur est toujours la
//! plus ancienne bougie en DB. Système identique pour tout asset ajouté
//! (les couples sont lus depuis la DB comme le reste du pipeline).

use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use common::{Asset, Timeframe};
use db::Database;

const PAGES_MAX: usize = 3000; // garde-fou : ~3 M bougies par couple
const DELAI_MS_DEFAUT: u64 = 200; // 5 req/s — rate limit Bybit 600/s, marge ×100

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let mut tous = false;
    let mut asset_cible: Option<String> = None;
    let mut tf_cible: Option<String> = None;
    let mut mois: i64 = 24;
    let mut delai_ms = DELAI_MS_DEFAUT;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--tous" => tous = true,
            "--asset" if i + 1 < args.len() => {
                asset_cible = Some(args[i + 1].clone());
                i += 1;
            }
            "--tf" if i + 1 < args.len() => {
                tf_cible = Some(args[i + 1].clone());
                i += 1;
            }
            "--mois" if i + 1 < args.len() => {
                mois = args[i + 1].parse().unwrap_or(24).clamp(1, 60);
                i += 1;
            }
            "--delai-ms" if i + 1 < args.len() => {
                delai_ms = args[i + 1].parse().unwrap_or(DELAI_MS_DEFAUT);
                i += 1;
            }
            autre => anyhow::bail!("argument inconnu : {autre}"),
        }
        i += 1;
    }

    if !tous && asset_cible.is_none() {
        anyhow::bail!("usage : backfill_profond --tous [--mois N] | --asset X [--tf M15] [--mois N]");
    }

    let db_path =
        std::env::var("DATABASE_PATH").unwrap_or_else(|_| "data/trading.db".to_string());
    let db = Arc::new(Database::new(&db_path).await?);
    db.run_migrations().await?;

    // Couples : depuis la DB (assets Bybit actifs × TF configurés) — tout
    // asset ajouté via l'UI est couvert automatiquement.
    let assets = assets_bybit(&db).await;
    let tfs = data::worker_config::lire_timeframes(&db).await;

    let couples: Vec<(Asset, Timeframe)> = if tous {
        assets.iter().flat_map(|a| tfs.iter().map(move |tf| (a.clone(), *tf))).collect()
    } else {
        let a = Asset::try_from(asset_cible.as_deref().unwrap())
            .map_err(|e| anyhow::anyhow!("asset inconnu : {e:?}"))?;
        let liste_tfs: Vec<Timeframe> = match &tf_cible {
            Some(t) => vec![Timeframe::try_from(t.as_str())
                .map_err(|e| anyhow::anyhow!("timeframe inconnu : {e:?}"))?],
            None => tfs.clone(),
        };
        liste_tfs.into_iter().map(|tf| (a.clone(), tf)).collect()
    };

    if couples.is_empty() {
        anyhow::bail!("aucun couple à traiter (assets Bybit actifs ou timeframes configurés)");
    }

    println!("═ Backfill profond Bybit — {} couple(s), cible {} mois ═", couples.len(), mois);
    let cible_ts = Utc::now().timestamp() - mois * 30 * 86_400;
    let provider = data::providers::BinanceProvider;
    let mut total_inserees: u64 = 0;
    let debut_global = std::time::Instant::now();

    for (idx, (asset, tf)) in couples.iter().enumerate() {
        println!(
            "\n[{}/{}] {} {}",
            idx + 1,
            couples.len(),
            asset.as_str(),
            tf.as_str()
        );
        match backfill_couple(&db, &provider, asset.clone(), *tf, cible_ts, delai_ms).await {
            Ok((inserees, plus_ancienne)) => {
                total_inserees += inserees;
                match plus_ancienne {
                    Some(ts) if ts <= cible_ts => println!(
                        "  ✅ cible atteinte — {} bougies insérées, historique jusqu'à {}",
                        inserees,
                        chrono::DateTime::<chrono::Utc>::from_timestamp(ts, 0)
                            .map(|d| d.format("%Y-%m-%d").to_string())
                            .unwrap_or_default()
                    ),
                    Some(ts) => println!(
                        "  ⚠️ historique Bybit épuisé à {} — {} bougies insérées (la cible {} mois n'est pas disponible pour ce couple)",
                        chrono::DateTime::<chrono::Utc>::from_timestamp(ts, 0)
                            .map(|d| d.format("%Y-%m-%d").to_string())
                            .unwrap_or_default(),
                        inserees,
                        mois
                    ),
                    None => println!("  — déjà complet, rien à faire"),
                }
            }
            Err(e) => println!("  ❌ échec : {e} — couple suivant"),
        }
    }

    println!(
        "\n═ Terminé en {:?} — {} bougies insérées au total ═",
        debut_global.elapsed(),
        total_inserees
    );
    Ok(())
}

/// Backfill d'UN couple : pagination descendante depuis la plus ancienne
/// bougie en DB jusqu'à la cible. Retourne (insérées, plus ancienne atteinte).
async fn backfill_couple(
    db: &Arc<Database>,
    provider: &data::providers::BinanceProvider,
    asset: Asset,
    tf: Timeframe,
    cible_ts: i64,
    delai_ms: u64,
) -> anyhow::Result<(u64, Option<i64>)> {
    // Curseur : plus ancienne bougie existante (ou now si vide).
    let plus_ancienne_db = plus_ancienne_bougie(db, &asset, tf).await?;
    let mut cursor_ms = plus_ancienne_db
        .map(|ts| ts * 1000)
        .unwrap_or_else(|| Utc::now().timestamp_millis());

    let mut inserees: u64 = 0;
    let mut plus_ancienne_atteinte: Option<i64> = None;
    let mut pages_vides_consecutives: u32 = 0;
    let mut pages_sans_insertion: u32 = 0;

    for _page in 0..PAGES_MAX {
        if cursor_ms / 1000 <= cible_ts {
            break; // cible atteinte
        }
        let tf_ms = tf.minutes() as i64 * 60_000;
        let cursor_avant = cursor_ms;
        let (bougies, plus_ancienne_brute_ms) = provider
            .fetch_page_avant_brute(&asset, tf, cursor_ms)
            .await?;
        // Anti-boucle (pages 2+) : Bybit doit servir une fenêtre
        // STRICTEMENT plus ancienne (au moins une période sous le curseur
        // précédent). La page 1 déborde au-dessus du curseur DB par
        // construction — elle est exemptée. Une page ultérieure qui ne
        // descend pas = données chevauchantes déjà connues → fin.
        if _page > 0 && cursor_ms > cursor_avant - tf_ms {
            break;
        }
        // Toujours avancer le curseur sur la plus ancienne bougie BRUTE
        // (avant filtrage week-end) — sinon une zone de week-end entier
        // laisse le curseur immobile et la boucle tourne à vide.
        if let Some(brute) = plus_ancienne_brute_ms {
            if brute < cursor_ms {
                cursor_ms = brute;
                plus_ancienne_atteinte = Some(brute / 1000);
            }
        }
        if bougies.is_empty() {
            break; // historique Bybit épuisé pour ce couple
        }
        let n = db
            .inserer_bougies_avec_source(&asset, &tf, &bougies, "bybit_backfill")
            .await?;
        inserees += n;
        if _page % 10 == 0 {
            println!(
                "    page {} — {} bougies cumulées, cursor {}",
                _page + 1,
                inserees,
                chrono::DateTime::<chrono::Utc>::from_timestamp(cursor_ms / 1000, 0)
                    .map(|d| d.format("%Y-%m-%d").to_string())
                    .unwrap_or_default()
            );
        }
        tokio::time::sleep(Duration::from_millis(delai_ms)).await;
    }

    Ok((inserees, plus_ancienne_atteinte))
}

/// Plus ancienne bougie d'un couple en DB (hors source mt5, comme les charts).
async fn plus_ancienne_bougie(
    db: &Arc<Database>,
    asset: &Asset,
    tf: Timeframe,
) -> anyhow::Result<Option<i64>> {
    let ligne: Option<(i64,)> = sqlx::query_as(
        "SELECT MIN(timestamp) FROM bougies WHERE asset = ?1 AND timeframe = ?2",
    )
    .bind(asset.as_str())
    .bind(tf.as_str())
    .fetch_optional(db.pool())
    .await?;
    Ok(ligne.and_then(|(t,)| if t > 0 { Some(t) } else { None }))
}

/// Assets Bybit actifs depuis la DB (source = binance, symbol_bybit présent) —
/// identique au worker WS : tout asset ajouté/coché est couvert.
async fn assets_bybit(db: &Arc<Database>) -> Vec<Asset> {
    match db.lister_assets_worker().await {
        Ok(assets) => assets
            .into_iter()
            .filter(|a| a.actif && a.source == "binance")
            .filter_map(|a| a.symbol_bybit.map(|_| a.id))
            .filter_map(|id| Asset::try_from(id.as_str()).ok())
            .collect(),
        Err(e) => {
            eprintln!("lecture assets impossible : {e}");
            Vec::new()
        }
    }
}
