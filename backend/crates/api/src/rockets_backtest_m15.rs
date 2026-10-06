//! Étape 13 (roadmap audit 05/10) — backtest comparatif D1 vs M15 pour la
//! détection de cassure rockets.
//!
//! Règle actuelle (D1) : cassure = close D1 ≥ pivot × 1,03 ET corps ≥ 80 %
//! de l'étendue (vérifiée au scan quotidien, entrée au prix du pivot).
//!
//! Règle proposée (M15) : le HIGH d'une bougie M15 atteint pivot × 1,03
//! dans la journée → détection intrajournalière, entrée au même prix du
//! pivot.
//!
//! La différence decisive : la COHORTE B — prix qui touche pivot × 1,03
//! intraday mais RETOMBE sous à la clôture (M15 entre, D1 n'entre pas).
//! La cohorte A (confirmée à la clôture) est identique pour les deux
//! règles → même R final. Si la cohorte B gagne en moyenne, M15 domine ;
//! si elle saigne (faux départs), D1 domine.
//!
//! Simplifications honnêtes (documentées pour le propriétaire) :
//! - Univers : panier fixe de 10 cryptos Binance (au lieu du top-100
//!   dynamique par volume — le top change chaque semaine et n'est pas
//!   reconstructible fidèlement).
//! - Points news IA non rejoués (le classement /10 de base suffit :
//!   sentiment, contexte, tendance, volatilité, breakout, liquidité —
//!   soit 6 points techniques + fondamentaux).
//! - Véto unlocks non rejoué (réservé aux calendriers externes).
//!
//! Lancement : cargo test -p api --bin api -- --ignored backtest_m15 --nocapture

#[cfg(test)]
mod tests {
    use rockets::classement::{classement_rocket, contexte_marche, BougieD1};
    use rockets::gestion::{pas_gestion, ActionRocket, PositionRocket};
    use rockets::types::{ParamsRockets, ProfilRisque};

    const SYMBOLES: &[&str] = &["BTCUSDT", "ETHUSDT", "SOLUSDT", "XRPUSDT", "BNBUSDT", "DOGEUSDT", "ADAUSDT", "AVAXUSDT", "LINKUSDT", "LTCUSDT"];

    /// Fenêtre de rejeu : 260 D1 (≈ 1 an de calendrier crypto 24/7).
    const FENETRE_D1: usize = 260;
    /// Jours de gestion après l'entrée (60 D1 ≈ 2 mois).
    const GESTION_JOURS: usize = 60;
    /// Seuil de classement pour un signal (sans news IA).
    const SEUIL_POINTS: u8 = 6;

    async fn klines_interval(symbole: &str, interval: &str, limite: usize, _debut_ms: i64) -> Vec<BougieD1> {
        if interval == "1d" {
            return crate::rockets_verticale::klines_d1(symbole, limite).await;
        }
        let url = format!("https://api.binance.com/api/v3/klines?symbol={}&interval={}&limit={}", symbole, interval, limite);
        let client = reqwest::Client::new();
        let texte = match client.get(&url).send().await {
            Ok(r) => match r.text().await { Ok(t) => t, Err(_) => return Vec::new() },
            Err(e) => { println!("  {} {} err: {}", symbole, interval, e); return Vec::new(); }
        };
        let brut: Vec<Vec<serde_json::Value>> = serde_json::from_str(&texte).unwrap_or_default();
        brut.iter().filter_map(|k| {
            Some(BougieD1 {
                ts: k.first()?.as_i64()? / 1000,
                open: k.get(1)?.as_f64()?,
                high: k.get(2)?.as_f64()?,
                low: k.get(3)?.as_f64()?,
                close: k.get(4)?.as_f64()?,
                volume: k.get(5)?.as_f64()?,
            })
        }).collect()
    }

    fn params_prod() -> ParamsRockets {
        ParamsRockets {
            profil: ProfilRisque::Neutre,
            trailing_pct: 5.0,
            stagnation_max_jours: 10,
            ..Default::default()
        }
    }

    /// Simule la gestion d'une position (entrée au pivot, stop, R1, trailing)
    /// sur `GESTION_JOURS` D1 après l'entrée. Retourne le R réalisé.
    fn simuler_gestion(d1_apres: &[BougieD1], pivot: f64, stop: f64, age_jours: &mut i64) -> f64 {
        let params = params_prod();
        let mut p = PositionRocket::nouvelle("TEST", pivot, stop);
        for b in d1_apres.iter().take(GESTION_JOURS) {
            match pas_gestion(&mut p, b.high, b.low, b.close, *age_jours, &params) {
                ActionRocket::Cloturer { r_realise, .. } => return r_realise,
                _ => { *age_jours += 1; }
            }
        }
        // Survivante à la fenêtre : sortie au dernier close.
        (d1_apres.last().map(|b| b.close).unwrap_or(pivot) - p.entree) / (p.entree - p.stop).max(1e-12)
    }

    /// Cohorte : cassure D1 confirmée (A) vs intraday seulement (B).
    struct Cohorte {
        label: &'static str,
        n: usize,
        somme_r: f64,
        gagnants: usize,
    }

    impl Cohorte {
        fn new(label: &'static str) -> Self { Self { label, n: 0, somme_r: 0.0, gagnants: 0 } }
        fn ajouter(&mut self, r: f64) { self.n += 1; self.somme_r += r; if r > 0.0 { self.gagnants += 1; } }
        fn ligne(&self) -> String {
            if self.n == 0 { return format!("  {} : 0 trade", self.label); }
            format!(
                "  {} : {} trades · Σ{:+.1}R · R/tr {:.2} · WR {:.0}%",
                self.label, self.n, self.somme_r, self.somme_r / self.n as f64,
                self.gagnants as f64 / self.n as f64 * 100.0
            )
        }
    }

    /// Diagnostic --ignored : le backtest complet D1 vs M15 sur le panier.
    #[actix_web::test]
    #[ignore = "diagnostic étape 13 : backtest D1 vs M15 sur vraie base Binance"]
    async fn backtest_m15_vs_d1() {
        let maintenant = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as i64;
        let mut coh_a = Cohorte::new("Cohorte A — D1 (confirmée à la clôture, les 2 règles prennent)");
        let mut coh_b = Cohorte::new("Cohorte B — M15 seul (intraday sans confirmation D1)");
        let mut nb_pivots = 0usize;
        let mut dist_points = [0usize; 9];

        for symbole in SYMBOLES {
            let debut = maintenant - (FENETRE_D1 as i64 + 30) * 86_400_000;
            let d1 = klines_interval(symbole, "1d", FENETRE_D1 + 30, debut).await;
            if d1.len() < 220 { println!("{} : {} D1 (insuffisant)", symbole, d1.len()); continue; }
            println!("{} : {} D1 chargées", symbole, d1.len());
            let ctx = contexte_marche(&d1.iter().map(|b| b.close).collect::<Vec<_>>()).unwrap_or(rockets::classement::ContexteMarche { marche_haussier: true, perf_marche_4s: 0.0 });

            // Phase 1 : jours candidats (classement ≥ seuil, pas de cassure
            // le jour même — le pivot est posé, il attend d'être cassé).
            // Phase 2 : pour chaque candidat, scan AVANT pour la première
            // journée où high ≥ pivot × 1,03 → c'est le jour de cassure.
            for t in 220..d1.len().saturating_sub(2) {
                let base = &d1[..=t];
                let r = classement_rocket(symbole, base, &ctx);
                dist_points[r.points as usize] += 1;
                if r.points < SEUIL_POINTS { continue; }
                let Some(pivot) = r.pivot else { continue; };
                let Some(stop) = r.stop else { continue; };
                // le jour candidat lui-même ne casse pas encore
                if base.last().map(|b| b.close).unwrap_or(0.0) >= pivot * 1.03 { continue; }
                nb_pivots += 1;

                // Scan avant : la première journée où le HIGH touche pivot×1,03
                for u in t + 1..d1.len().saturating_sub(1) {
                    let jour = &d1[u];
                    if jour.high < pivot * 1.03 { continue; }

                    // Cassure D1 confirmée (close ≥ pivot×1,03) ?
                    let cassure_d1 = jour.close >= pivot * 1.03;

                    // M15 de cette journée
                    let m15 = klines_interval(symbole, "15m", 96, jour.ts * 1000).await;
                    let m15_touche = m15.iter().any(|b| b.high >= pivot * 1.03);

                    if !m15_touche && !cassure_d1 { continue; }

                    let suite = &d1[u + 1..];
                    let mut age = 0i64;
                    if cassure_d1 {
                        coh_a.ajouter(simuler_gestion(suite, pivot, stop, &mut age));
                    } else {
                        coh_b.ajouter(simuler_gestion(suite, pivot, stop, &mut age));
                    }
                    break; // une seule cassure par candidat
                }
            }
        } // for symbole

        println!("═══ BACKTEST ROCKETS : D1 vs M15 ({} symboles, {} j de fenêtre) ═══", SYMBOLES.len(), FENETRE_D1);
        println!("Distribution points [0-8] : {:?}", dist_points);
        println!("Candidats (classement ≥ {}) : {}", SEUIL_POINTS, nb_pivots);
        println!("{}", coh_a.ligne());
        println!("{}", coh_b.ligne());
        println!();
        let total_d1 = coh_a.somme_r;
        let total_m15 = coh_a.somme_r + coh_b.somme_r;
        println!("R TOTAL règle D1  (A seulement)        : {:+.1}R sur {} trades", total_d1, coh_a.n);
        println!("R TOTAL règle M15 (A + B)             : {:+.1}R sur {} trades", total_m15, coh_a.n + coh_b.n);
        println!();
        if coh_b.n > 0 {
            let r_b = coh_b.somme_r / coh_b.n as f64;
            if r_b > 0.0 {
                println!("→ VERDICT : la cohorte B gagne ({:+.2}R/tr) — M15 DOMINE (plus de trades ET du R)", r_b);
            } else {
                println!("→ VERDICT : la cohorte B saigne ({:+.2}R/tr) — D1 DOMINE (le filtre clôture protège)", r_b);
            }
        } else {
            println!("→ Aucune cohorte B détectée sur la fenêtre — les deux règles sont équivalentes");
        }
    }
}
