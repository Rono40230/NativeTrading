//! Étape 1 de la roadmap audit (05/10) — diagnostic KDJ : pourquoi zéro
//! signal en production alors que le scanner voit des candidats et que le
//! labo 7.G a rejoué des setups gagnants ?
//!
//! Test IGNORÉ par défaut (il lit la vraie base, chemin local) : lancer
//! explicitement via
//! `cargo test -p api --bin api -- --ignored diagnostic_kdj --nocapture`.
//!
//! Principe : mêmes bougies H1 réelles, mêmes paramètres de production —
//! le rejeur batch d'un côté, le moteur live clôture par clôture de
//! l'autre. Toute divergence de compte isole le bug ; une concordance à
//! zéro prouve que les conditions de production ne passent nulle part
//! (décision propriétaire : assouplir ou désarmer).

#[cfg(test)]
mod diagnostic_kdj {
    use common::{Asset, Timeframe};
    use engine::{ContexteCloture, Engine};

    /// Parité rejeur ↔ live sur données réelles + histogramme des
    /// conditions bloquantes si rien ne passe.
    #[tokio::test]
    #[ignore = "diagnostic local : lit la vraie base (étape 1 roadmap, chemin absolu)"]
    async fn parite_kdj_rejeur_vs_live_donnees_reelles() {
        let db = db::Database::new("/mnt/IA/native-trading-ai/data/trading.db").await.expect("DB");
        let p = db::kdj_params::lire_kdj_params(db.pool()).await;
        let base = kdj_halftrend::ParamsKdj {
            period: p.period as usize,
            signal: p.signal as usize,
            amplitude: p.amplitude as usize,
            ratio_risk: p.ratio_risk,
            adx_min: if p.adx_min < 0.0 { None } else { Some(p.adx_min) },
        };
        println!("params production : {:?}", base);

        for asset in ["BNB", "XAGUSD", "XPTUSD", "XRP", "LINK", "SOL"] {
            let a = Asset::try_from(asset).expect("asset");
            let bougies = db
                .obtenir_bougies(&a, &Timeframe::H1, 3_000)
                .await
                .unwrap_or_default();
            if bougies.len() < 300 {
                println!("{asset} : {} bougies H1 — insuffisant", bougies.len());
                continue;
            }

            // Référence : le rejeur batch sur toute la fenêtre.
            let trades = kdj_halftrend::rejouer(&bougies, &base);

            // Le moteur live, clôture par clôture (mêmes données).
            let mut moteur = kdj_halftrend::KdjEngine::nouveau(a.clone(), Timeframe::H1)
                .avec_params(base.clone());
            let mut signaux = 0usize;
            for (k, b) in bougies.iter().enumerate() {
                let ctx = ContexteCloture { asset: &a, tf: Timeframe::H1, bougie: b, index_barre: k };
                let s = moteur.on_close(&ctx);
                signaux += s.signaux.len();
            }

            println!(
                "{asset} : {} bougies H1 ({:?} → {:?}) · rejeur {} trade(s) · live {} signal(aux)",
                bougies.len(),
                bougies.first().map(|b| b.timestamp.date_naive()),
                bougies.last().map(|b| b.timestamp.date_naive()),
                trades.len(),
                signaux
            );
            for t in trades.iter().take(3) {
                println!("   rejeur : entrée {:?} dir {}", t.ts_entree, t.direction);
            }
        }
    }

    /// Validation rétro étape 1 (owner 05/10) : le moteur CHAUFFÉ tel qu'il
    /// tourne désormais en production — semé jusqu'à il y a 7 jours, nourri
    /// des clôtures H1 des 7 derniers jours — quels signaux aurait-il émis ?
    /// Lecture seule : rien n'est écrit, rien n'est annoncé.
    #[tokio::test]
    #[ignore = "diagnostic local : lit la vraie base (validation rétro KDJ)"]
    async fn kdj_signaux_7_derniers_jours() {
        let db = db::Database::new("/mnt/IA/native-trading-ai/data/trading.db").await.expect("DB");
        let p = db::kdj_params::lire_kdj_params(db.pool()).await;
        let params = kdj_halftrend::ParamsKdj {
            period: p.period as usize,
            signal: p.signal as usize,
            amplitude: p.amplitude as usize,
            ratio_risk: p.ratio_risk,
            adx_min: if p.adx_min < 0.0 { None } else { Some(p.adx_min) },
        };
        let limite = chrono::Utc::now().timestamp() - 7 * 86_400;
        let mut total = 0usize;

        for asset in ["BNB", "XAGUSD", "XPTUSD", "XRP", "LINK", "SOL"] {
            let a = Asset::try_from(asset).expect("asset");
            let bougies = db
                .obtenir_bougies(&a, &Timeframe::H1, 3_000)
                .await
                .unwrap_or_default();
            let coupe = bougies.partition_point(|b| b.timestamp.timestamp() < limite);
            if coupe < 260 {
                println!("{asset} : historique pré-fenêtre insuffisant ({})", coupe);
                continue;
            }
            // Moteur exactement comme en production : chauffe = tout ce qui
            // précède la fenêtre, puis clôtures vivantes une à une.
            let mut moteur = kdj_halftrend::KdjEngine::nouveau(a.clone(), Timeframe::H1)
                .avec_params(params.clone())
                .avec_chauffe(&bougies[..coupe]);
            let mut signaux = Vec::new();
            for (k, b) in bougies[coupe..].iter().enumerate() {
                let ctx = ContexteCloture { asset: &a, tf: Timeframe::H1, bougie: b, index_barre: coupe + k };
                signaux.extend(moteur.on_close(&ctx).signaux);
            }
            total += signaux.len();
            for s in &signaux {
                println!(
                    "   {} {:?} {} @ {:.4} le {}",
                    asset,
                    s.direction,
                    s.cle,
                    s.prix_entree,
                    chrono::DateTime::from_timestamp(s.debut_barre, 0)
                        .map(|d| d.with_timezone(&chrono_tz::Europe::Paris).format("%d/%m %Hh%M").to_string())
                        .unwrap_or_else(|| "?".into())
                );
            }
            if signaux.is_empty() {
                println!("{asset} : 0 signal sur la fenêtre (aucun flip Halftrend confirmé)");
            }
        }
        println!("── TOTAL 7 jours : {total} signal(aux) ──");
    }
}
