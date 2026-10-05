//! Gestion des positions rockets — extraite de rockets_verticale.rs
//! (limite 600 lignes, convention du dépôt). Un pas de gestion par bougie
//! du jour (crypto : klines Binance ; action : séance Yahoo), règle du
//! journal : invalidation → R1 → trailing → stagnation (étape 3 roadmap
//! audit 05/10 : une rocket qui ne décolle pas rend le capital).

use std::sync::Arc;

use db::Database;
use rockets::gestion::{pas_gestion, PositionRocket};
use crate::rockets_verticale::{klines_d1, lire_params};
use sqlx::Row;

// ── Gestion des positions ouvertes ─────────────────────────────────────────

/// Bougie « en cours » évaluée par la gestion : high/low/dernier, quelle que
/// soit la source (bougie D1 Binance en formation, ou séance du jour Yahoo).
struct BougieBoulee {
    high: f64,
    low: f64,
    close: f64,
}

pub(crate) async fn boucle_gestion(db: Arc<Database>) {
    // Recadrage propriétaire 06/09 : la gestion vit en CONTINU (cycle 30 s)
    // — ne pas attendre la clôture D1, sinon une redescente après R1
    // transformerait l'occasion en perte. Neutralisation dès que R1 est
    // touché, trailing déclenché alors, sorties au niveau touché.
    tracing::info!("🚀 Rockets gestion armée (30 s — live, décision 06/09)");
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(30)).await;
        gerer_positions(&db).await;
    }
}

async fn gerer_positions(db: &Arc<Database>) {
    let params = lire_params(db).await;
    let lignes = match sqlx::query(
        "SELECT cle, symbole, entree, stop, r1, neutralise, trailing, sommet, ts_entree FROM rockets_positions WHERE fermee = 0",
    )
    .fetch_all(db.pool())
    .await
    {
        Ok(l) => l,
        Err(_) => return,
    };
    // Cours live des positions ACTIONS via Yahoo (décision 06/09 — même
    // source que le Journal de Trading) : la « bougie du jour » (haut/bas/
    // dernier) joue le rôle de la bougie D1 en cours des cryptos.
    let tickers_actions: Vec<String> = lignes
        .iter()
        .filter_map(|l| {
            let s: String = l.get("symbole");
            (!s.ends_with("USDT")).then_some(s)
        })
        .collect();
    let quotes_yahoo = crate::yahoo_quotes::quotes(&tickers_actions).await;

    for l in lignes {
        let cle: String = l.get("cle");
        let symbole: String = l.get("symbole");
        let mut p = PositionRocket {
            symbole: symbole.clone(),
            entree: l.get("entree"),
            stop: l.get("stop"),
            r1: l.get("r1"),
            neutralise: l.get::<i64, _>("neutralise") != 0,
            trailing: l.try_get::<Option<f64>, _>("trailing").ok().flatten(),
        };
        // Bougie évaluée = le LIVE (décision 06/09 : « dès que R1 atteint =
        // neutralisation et TS », ne pas attendre la clôture). Crypto →
        // bougie D1 en cours Binance ; action → la séance du jour Yahoo
        // (haut/bas du jour, dernier prix). Hors session US, le « dernier »
        // est la clôture (ou le post-market) : la gestion reprend au
        // prochain cours. Précédence conservatrice inchangée (stop avant
        // R1, comme la SMC/Pine).
        let (high, low, close) = if symbole.ends_with("USDT") {
            let bougies = klines_d1(&symbole, 2).await;
            let Some(b) = bougies.last() else {
                // Plus jamais de saut silencieux (05/10 : WBTC/WIF vivaient
                // sans qu'on sache pourquoi) — la position reste, le cycle
                // retente, l'owner voit la cause.
                tracing::warn!("🚀 Rockets {} : klines Binance indisponibles — position non évaluée ce cycle", symbole);
                continue;
            };
            (b.high, b.low, b.close)
        } else {
            let Some(q) = quotes_yahoo.get(&symbole) else {
                tracing::warn!("🚀 Rockets {} : cours Yahoo indisponible — position non évaluée ce cycle", symbole);
                continue;
            };
            (q.haut_jour, q.bas_jour, q.prix)
        };
        let b = BougieBoulee { high, low, close };
        // Sommet de vie du trade (affichage de l'historique — aucune règle
        // ne le consomme) : le high de la bougie en cours, jamais vers le bas.
        let _ = sqlx::query(
            "UPDATE rockets_positions SET sommet = MAX(COALESCE(sommet, ?), ?) WHERE cle = ?",
        )
        .bind(b.high)
        .bind(b.high)
        .bind(&cle)
        .execute(db.pool())
        .await;
        // Unité de ts_entree : les émissions crypto d'avant octobre
        // écrivaient des millisecondes (WBTC/WIF) quand les actions
        // écrivent des secondes — normalisation à la lecture pour que
        // l'âge (donc la stagnation) soit juste dans tous les cas.
        let ts_entree_brut: i64 = l.get("ts_entree");
        let ts_entree = if ts_entree_brut > 10_000_000_000 { ts_entree_brut / 1_000 } else { ts_entree_brut };
        let age_jours = (chrono::Utc::now().timestamp() - ts_entree) / 86_400;
        match pas_gestion(&mut p, b.high, b.low, b.close, age_jours, &params) {
            rockets::gestion::ActionRocket::Rien => {
                let _ = sqlx::query("UPDATE rockets_positions SET neutralise = ?, trailing = ? WHERE cle = ?")
                    .bind(p.neutralise as i64)
                    .bind(p.trailing)
                    .bind(&cle)
                    .execute(db.pool())
                    .await;
            }
            rockets::gestion::ActionRocket::Neutraliser { prix, trailing } => {
                let _ = sqlx::query("UPDATE rockets_positions SET neutralise = 1, trailing = ?, prix_r1 = ? WHERE cle = ?")
                    .bind(trailing)
                    .bind(prix)
                    .bind(&cle)
                    .execute(db.pool())
                    .await;
                tracing::info!("🚀 Rockets {} : R1 atteint — 50 % vendus, trailing {:.4}", symbole, trailing);
            }
            rockets::gestion::ActionRocket::Cloturer { prix, verdict, r_realise } => {
                let verdict_str = match verdict {
                    rockets::gestion::VerdictRocket::Sl => "SL",
                    rockets::gestion::VerdictRocket::Stagnation => "STAG",
                    _ => "TS",
                };
                let _ = sqlx::query("UPDATE rockets_positions SET fermee = 1, verdict = ?, r_realise = ?, prix_sortie = ? WHERE cle = ?")
                    .bind(verdict_str)
                    .bind(r_realise)
                    .bind(prix)
                    .bind(&cle)
                    .execute(db.pool())
                    .await;
                let _ = db.fermer_signal_par_cle(&cle, &symbole, verdict_str, prix, r_realise, chrono::Utc::now().timestamp()).await;
                tracing::info!("🚀 Rockets {} : {} ({:.2} R)", symbole, verdict_str, r_realise);
            }
        }
    }
}

