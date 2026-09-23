//! Tests de cohérence du centre d'analyse (extraits d'analyses.rs).
use super::*;

    /// Base mémoire + migrations + une stratégie SMC dotée d'un capital.
    async fn base_test() -> Arc<db::Database> {
        let db = db::Database::new(":memory:").await.expect("DB mémoire");
        db.run_migrations().await.expect("migrations OK");
        sqlx::query("UPDATE strategies SET etat = 'Officielle', capital = 1000.0, risque_pct = 1.0 WHERE id = 'SMC'")
            .execute(db.pool())
            .await
            .expect("stratégie SMC");
        Arc::new(db)
    }

    /// Insère une clôture SMC remplie (entrée 2000, SL 1990 → risque 10).
    async fn cloture(db: &Arc<db::Database>, id: &str, verdict: &str, score: f64, direction: &str, r_realise: f64) {
        sqlx::query(
            "INSERT INTO signaux (id, asset, timeframe, direction, score, prix_entree, stop_loss,
                                  take_profit, strategie, statut, verdict, r_realise, cree_le,
                                  heure_entree, ferme_le)
             VALUES (?, 'BTC', 'M5', ?, ?, 2000.0, 1990.0, '[2006, 2020, 2030]',
                     'SMC', 'Fermé', ?, ?, 1700000000, 1700000600, 1700001200)",
        )
        .bind(id)
        .bind(direction)
        .bind(score)
        .bind(verdict)
        .bind(r_realise)
        .execute(db.pool())
        .await
        .expect("insertion clôture");
    }

    /// HARMONISATION (15/09 soir) : l'analyse, la simulation capital et les
    /// catégories doivent compter EXACTEMENT les mêmes clôtures avec
    /// EXACTEMENT les mêmes R — c'est le verrou qui interdit à un écran de
    /// diverger d'un autre (bug « 257 vs 348 vs camemberts »).
    #[actix_web::test]
    async fn analyser_coherent_avec_simuler_et_categories() {
        let db = base_test().await;
        cloture(&db, "t1", "TP1+BE", 7.0, "Long", 0.3).await;   // r_distance 0.6
        cloture(&db, "t2", "TP2+BE", 10.0, "Short", 1.2).await; // r_distance 2.0
        cloture(&db, "t3", "SL", 15.0, "Long", -1.0).await;     // r_distance −1.0
        cloture(&db, "t4", "Expire", 8.0, "Short", 0.0).await;  // repli r_realise 0.0

        let a = analyser(&db, "SMC").await;
        let sim = crate::capital_simule::simuler(&db, "SMC").await.expect("simulation");

        // Même effectif partout : points capital, catégories, périodes, heatmap.
        assert_eq!(a.nb_trades, 4);
        assert_eq!(sim.points.len(), 4, "points capital ≠ nb_trades");
        assert_eq!(a.tfs.iter().map(|c| c.n).sum::<usize>(), 4, "Σ tfs.n");
        assert_eq!(a.assets.iter().map(|c| c.n).sum::<usize>(), 4, "Σ assets.n");
        assert_eq!(a.verdicts.iter().map(|c| c.n).sum::<usize>(), 4, "Σ verdicts.n");
        assert_eq!(a.journalier.iter().map(|p| p.trades).sum::<usize>(), 4, "Σ journalier");
        assert_eq!(a.heatmap.iter().map(|c| c.trades).sum::<usize>(), 4, "Σ heatmap");

        // Deux voix distinctes (23/09) : r_total = Σ encaissé (compose le
        // capital, interne) ; les catégories/périodes portent la DISTANCE
        // (le R affiché). Vérifiées séparément.
        let somme_points: f64 = sim.points.iter().map(|p| p.r_pondere).sum();
        assert!((a.r_total - somme_points).abs() < 1e-9, "r_total {} ≠ Σ points {}", a.r_total, somme_points);
        // Encaissé : TP1+BE 0,3 + TP2+BE 1,02 (solde repli TP1) + SL −1 + Expire 0.
        assert!((a.r_total - 0.32).abs() < 1e-9, "ΣR encaissé attendu 0.32, obtenu {}", a.r_total);
        // Distance : 0.6 + 2.0 + (−1.0) + 0.0 = 1.6 — Σ affichée.
        assert!((a.r_distance_total - 1.6).abs() < 1e-9, "ΣR distance attendue 1.6, obtenu {}", a.r_distance_total);
        let somme_points_distance: f64 = sim.points.iter().map(|p| p.r_distance).sum();
        assert!((a.r_distance_total - somme_points_distance).abs() < 1e-9, "r_distance_total ≠ Σ points.r_distance");
        let somme_verdicts: f64 = a.verdicts.iter().map(|c| c.r).sum();
        assert!((a.r_distance_total - somme_verdicts).abs() < 1e-9, "Σ verdicts.r (distance) ≠ r_distance_total");
        assert!((a.r_moyen - 0.08).abs() < 1e-9, "r_moyen {}", a.r_moyen);
        // Clôtures exposées : même effectif, mêmes voix.
        assert_eq!(a.clotures.len(), 4, "clotures exposées");
        let somme_exposee: f64 = a.clotures.iter().map(|c| c.r_distance).sum();
        assert!((somme_exposee - a.r_distance_total).abs() < 1e-9, "Σ clotures.r_distance ≠ r_distance_total");

        // WR ($ > 0) + taux de perte ($ < 0) : complémentaires au pire des ~0 $.
        assert!(a.taux_reussite + a.taux_perte <= 1.0 + 1e-9);
        assert!(a.taux_reussite >= 0.0 && a.taux_reussite <= 1.0);

        // Tranches de score : mêmes lignes, aucun trade perdu.
        assert_eq!(a.par_score.iter().map(|t| t.n).sum::<usize>(), 4, "Σ par_score.n");
        assert_eq!(a.par_score[0].n, 2, "tranche 6–8 : scores 7 et 8");

        // Capital : départ persisté, actuel = départ + Σ profits.
        assert!((a.capital_depart - 1000.0).abs() < 1e-9);
        let somme_profits: f64 = sim.points.iter().map(|p| p.profit).sum();
        assert!((a.capital_actuel - (a.capital_depart + somme_profits)).abs() < 1e-6);
    }

    /// Le r_distance servi par /api/signaux est le même que celui des points
    /// capital (miroir signaux_lecture ↔ capital_simule).
    #[actix_web::test]
    async fn r_distance_signaux_egale_points_capital() {
        let db = base_test().await;
        cloture(&db, "u1", "TP2+BE", 10.0, "Long", 1.2).await;
        cloture(&db, "u2", "SL", 10.0, "Short", -1.0).await;

        let sim = crate::capital_simule::simuler(&db, "SMC").await.expect("simulation");
        let signaux = db.obtenir_signaux(100).await.expect("signaux");
        let par_id: std::collections::HashMap<String, f64> = signaux
            .iter()
            .filter_map(|s| {
                let r = s.get("r_distance")?.as_f64()?;
                let id = s.get("id")?.as_str()?.to_string();
                Some((id, r))
            })
            .collect();
        assert_eq!(par_id.len(), 2, "r_distance servi sur chaque clôture remplie");
        for p in &sim.points {
            let r_signal = par_id.get(&p.id).unwrap_or_else(|| panic!("signal {} absent", p.id));
            assert!((r_signal - p.r_distance).abs() < 1e-9, "signal {} : {} ≠ {}", p.id, r_signal, p.r_distance);
        }
    }
