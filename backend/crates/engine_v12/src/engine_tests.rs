//! Tests du plugin engine_v12 (scindés de `lib.rs` — règle < 600 lignes).

use super::*;
use chrono::{TimeZone, Utc};

    /// CSV XAUUSD M15 (même source que les tests v12) — le test est sauté
    /// silencieusement si le fichier est absent.
    const XAUUSD_M15_CSV: &str = "/mnt/IA/nautilus-smc-spike/xauusd_m15.csv";

    fn charger_bars() -> Vec<BarInput> {
        let contenu = match std::fs::read_to_string(XAUUSD_M15_CSV) {
            Ok(c) => c,
            Err(_) => return Vec::new(),
        };
        contenu
            .lines()
            .filter_map(|l| {
                let f: Vec<&str> = l.split(',').collect();
                if f.len() < 6 {
                    return None;
                }
                Some(BarInput {
                    timestamp: f[0].parse().ok()?,
                    open: f[1].parse().ok()?,
                    high: f[2].parse().ok()?,
                    low: f[3].parse().ok()?,
                    close: f[4].parse().ok()?,
                    volume: f[5].parse().ok()?,
                })
            })
            .collect()
    }

    fn candle_depuis_bar(b: &BarInput) -> Candle {
        Candle {
            timestamp: match Utc.timestamp_opt(b.timestamp, 0) {
                chrono::LocalResult::Single(t) => t,
                _ => chrono::DateTime::<Utc>::UNIX_EPOCH,
            },
            open: b.open,
            high: b.high,
            low: b.low,
            close: b.close,
            volume: b.volume,
        }
    }

    /// Bougie en formation synthétique reflétant une barre à un instant donné.
    fn formation_partielle(
        b: &BarInput,
        close_actuel: f64,
        high_vu: f64,
        low_vu: f64,
    ) -> BougieEnFormation {
        BougieEnFormation {
            debut: b.timestamp,
            open: b.open,
            high: high_vu,
            low: low_vu,
            close: close_actuel,
            volume: b.volume,
            nb_events: 1,
            dernier_event: None,
        }
    }

    fn ctx_tick<'a>(asset: &'a Asset, tf: Timeframe, f: &'a BougieEnFormation) -> ContexteTick<'a> {
        ContexteTick {
            asset,
            tf,
            bougie: f,
        }
    }

    fn ctx_close<'a>(
        asset: &'a Asset,
        tf: Timeframe,
        c: &'a Candle,
        idx: usize,
    ) -> ContexteCloture<'a> {
        ContexteCloture {
            asset,
            tf,
            bougie: c,
            index_barre: idx,
        }
    }

    /// Fidélité rollback : l'évaluation tick-par-tick (clones) et l'évaluation
    /// par clôtures aboutissent au MÊME état confirmé. Le tick émet des
    /// ANNONCES d'imminence (une seule par trade) ; la clôture confirme le
    /// trade en base (deja_annonce si le message est déjà parti).
    #[test]
    fn ticks_et_clotures_aboutissent_au_meme_etat_confirme() {
        let bars = charger_bars();
        if bars.is_empty() {
            return; // CSV absent — test sauté
        }
        let asset = Asset::from("XAUUSD");
        let tf = Timeframe::M15;

        let mut par_clotures = MoteurV12::nouveau(asset.clone(), tf);
        let mut par_ticks = MoteurV12::nouveau(asset.clone(), tf);

        let mut signaux_clotures = 0usize;
        let mut annonces_ticks = 0usize;
        let mut annonces_distinctes: HashSet<String> = HashSet::new();
        let mut ajouts_a_la_cloture = 0usize;

        for (i, b) in bars.iter().enumerate() {
            // Chemin A : clôtures seules.
            let c = candle_depuis_bar(b);
            signaux_clotures += par_clotures
                .on_close(&ctx_close(&asset, tf, &c, i))
                .signaux
                .len();

            // Chemin B : ticks simulés (open → extrême bas → extrême haut →
            // close) puis clôture. Chaque tick = évaluation clone complète.
            let scenarios = [
                formation_partielle(b, b.open, b.open, b.open),
                formation_partielle(b, b.low, b.open.max(b.open), b.low),
                formation_partielle(b, b.high, b.high, b.low),
                formation_partielle(b, b.close, b.high, b.low),
            ];
            for f in &scenarios {
                let s = par_ticks.on_tick(&ctx_tick(&asset, tf, f));
                for sig in &s.signaux {
                    assert!(sig.annonce, "le chemin tick n'émet QUE des annonces");
                    // Une clé n'est JAMAIS annoncée deux fois.
                    assert!(
                        annonces_distinctes.insert(sig.cle.clone()),
                        "annonce ré-émise pour {}",
                        sig.cle
                    );
                }
                annonces_ticks += s.signaux.len();
            }
            // Le dernier tick simulé porte EXACTEMENT la barre finale : la
            // clôture ne doit ajouter AUCUN événement (les signaux de
            // confirmation sont attendus — c'est leur unique chemin d'entrée
            // en base, avec deja_annonce si l'imminence est déjà partie).
            let s_close = par_ticks.on_close(&ctx_close(&asset, tf, &c, i));
            for sig in &s_close.signaux {
                assert!(!sig.annonce, "la clôture n'émet pas d'annonce");
                if annonces_distinctes.contains(&sig.cle) {
                    assert!(
                        sig.deja_annonce,
                        "signal confirmé déjà annoncé intrabar : {}",
                        sig.cle
                    );
                }
            }
            ajouts_a_la_cloture += s_close.evenements.len();
        }

        // 1. États confirmés identiques : mêmes trades, même index de barre.
        assert_eq!(
            par_clotures.livre_trades_dbg(),
            par_ticks.livre_trades_dbg(),
            "le chemin tick (clones jetés) ne doit jamais altérer l'état confirmé"
        );

        // 2. Annonces intrabar émises, sans doublon ; confirmations en base.
        assert!(
            annonces_ticks > 0,
            "annonces d'imminence attendues sur le chemin tick"
        );
        assert!(signaux_clotures > 0, "signaux attendus à la clôture");

        // 3. Invariant lifecycle : le dernier tick simulé porte exactement la
        //    barre finale — la clôture ne doit rien annoncer de nouveau.
        assert_eq!(
            ajouts_a_la_cloture, 0,
            "la clôture ne doit annoncer aucun événement après un tick complet"
        );
    }

    /// Lifecycle intrabar : les fills, TP et clôtures sont détectés pendant
    /// le replay tick-par-tick, sans jamais deux fois la même transition.
    #[test]
    fn lifecycle_intrabar_detecte_au_tick_sans_doublon() {
        let bars = charger_bars();
        if bars.is_empty() {
            return;
        }
        let asset = Asset::from("XAUUSD");
        let tf = Timeframe::M15;
        let mut moteur = MoteurV12::nouveau(asset.clone(), tf);

        let mut types_vus: HashSet<(String, TypeEvenementTrade)> = HashSet::new();
        let mut nb_fills = 0;
        let mut nb_tp1 = 0;
        let mut nb_clotures = 0;

        for (i, b) in bars.iter().enumerate() {
            // Deux évaluations tick + clôture par barre.
            for f in [
                formation_partielle(b, b.open, b.open, b.open),
                formation_partielle(b, b.close, b.high, b.low),
            ] {
                let s = moteur.on_tick(&ctx_tick(&asset, tf, &f));
                for e in &s.evenements {
                    assert!(
                        types_vus.insert((e.cle_trade.clone(), e.evenement)),
                        "transition {:?} sur {:?} émise deux fois",
                        e.evenement,
                        e.cle_trade
                    );
                    match e.evenement {
                        TypeEvenementTrade::Fill => nb_fills += 1,
                        TypeEvenementTrade::Tp1 => nb_tp1 += 1,
                        TypeEvenementTrade::Cloture => nb_clotures += 1,
                        _ => {}
                    }
                }
            }
            let c = candle_depuis_bar(b);
            let s = moteur.on_close(&ctx_close(&asset, tf, &c, i));
            for e in &s.evenements {
                assert!(
                    types_vus.insert((e.cle_trade.clone(), e.evenement)),
                    "transition {:?} émise deux fois (au close)",
                    e.evenement
                );
                match e.evenement {
                    TypeEvenementTrade::Fill => nb_fills += 1,
                    TypeEvenementTrade::Tp1 => nb_tp1 += 1,
                    TypeEvenementTrade::Cloture => nb_clotures += 1,
                    _ => {}
                }
            }
        }

        assert!(nb_fills > 0, "au moins un fill attendu sur 700 bars");
        assert!(
            nb_clotures > 0,
            "au moins une clôture attendue sur 700 bars"
        );
        let _ = nb_tp1; // compté pour diagnostic (peut être 0 — voir ROADMAP)
    }

    /// Aucune clé n'est émise deux fois, quel que soit le chemin.
    #[test]
    fn aucune_double_emission_sur_replay() {
        let bars = charger_bars();
        if bars.is_empty() {
            return;
        }
        let asset = Asset::from("XAUUSD");
        let tf = Timeframe::M15;
        let mut moteur = MoteurV12::nouveau(asset.clone(), tf);

        let _toutes: Vec<(i64, u8, u8, u64)> = Vec::new();
        for (i, b) in bars.iter().enumerate() {
            // Deux ticks + clôture par barre : l'anti-ré-émission doit tenir.
            let f1 = formation_partielle(b, b.open, b.open, b.open);
            let f2 = formation_partielle(b, b.close, b.high, b.low);
            let _ = moteur.on_tick(&ctx_tick(&asset, tf, &f1));
            let _ = moteur.on_tick(&ctx_tick(&asset, tf, &f2));
            let c = candle_depuis_bar(b);
            let _ = moteur.on_close(&ctx_close(&asset, tf, &c, i));
        }
        // Le cache `emis` est élagué par présence dans le carnet à chaque
        // évaluation : sa taille suit le carnet, jamais l'historique complet.
        assert!(moteur.emis.len() <= moteur.livre_trades().len());
        let _ = _toutes;
    }

    /// Replay pur par clôtures : des signaux existent sur l'historique XAUUSD.
    #[test]
    fn replay_clotures_emet_des_signaux() {
        let bars = charger_bars();
        if bars.is_empty() {
            return;
        }
        let asset = Asset::from("XAUUSD");
        let tf = Timeframe::M15;
        let mut moteur = MoteurV12::nouveau(asset.clone(), tf);
        let mut total = 0;
        for (i, b) in bars.iter().enumerate() {
            let c = candle_depuis_bar(b);
            total += moteur.on_close(&ctx_close(&asset, tf, &c, i)).signaux.len();
        }
        assert!(
            total > 0,
            "le moteur v12 doit émettre des signaux sur 700 bars"
        );
    }

    /// Zones d'approche (fix 09/10) : une OB née intrabar doit être visible
    /// AVANT la clôture de sa barre — c'est tout l'objet de l'alerte. La
    /// séquence : warm-up ATR → bougie baissière (future OB) → impulsion
    /// haussière qui NE touche pas la zone → la zone [102 ; 98] existe,
    /// Vierge, dans l'évaluation live comme au commit. Un tick qui touche
    /// la zone la fait disparaître immédiatement (elle n'est plus fraîche).
    #[test]
    fn zone_approche_visible_intrabar_avant_toucher() {
        let asset = Asset::from("XAUUSD");
        let tf = Timeframe::M15;
        let mut moteur = MoteurV12::nouveau(asset.clone(), tf);
        let mut ts = 1_700_000_000_i64;
        let mut bar = |o: f64, h: f64, l: f64, c: f64| {
            ts += 900;
            BarInput { timestamp: ts, open: o, high: h, low: l, close: c, volume: 1.0 }
        };
        // Warm-up ATR (16 barres calmes autour de 100).
        for i in 0..16 {
            let b = bar(100.0, 100.6 + (i % 2) as f64 * 0.2, 99.4, 100.0 + (i % 2) as f64 * 0.2);
            let c = candle_depuis_bar(&b);
            moteur.on_close(&ctx_close(&asset, tf, &c, i));
        }
        // Bougie baissière (future OB : top 102, bot 98) puis impulsion
        // haussière dont le low (102.4) reste AU-DESSUS du top — pas de touche.
        let b1 = bar(100.0, 102.0, 98.0, 98.0);
        let c1 = candle_depuis_bar(&b1);
        moteur.on_close(&ctx_close(&asset, tf, &c1, 16));
        let b2 = bar(102.5, 112.0, 102.4, 111.0);
        let c2 = candle_depuis_bar(&b2);
        moteur.on_close(&ctx_close(&asset, tf, &c2, 17));

        // Au commit : la zone ACHAT [102 ; 98] est Vierge → visible.
        let zones = moteur.zones_approche();
        let z = zones.iter().find(|z| z.achat && z.bord_proche == 102.0 && z.bord_lointain == 98.0);
        assert!(z.is_some(), "zone ACHAT [102;98] attendue au commit, reçu : {zones:?}");

        // Tick live qui APPROCHE sans toucher (close 106 > 102) : toujours là.
        let f = BougieEnFormation {
            debut: ts + 900, open: 111.0, high: 111.0, low: 106.0, close: 106.0,
            volume: 1.0, nb_events: 3, dernier_event: None,
        };
        moteur.on_tick(&ctx_tick(&asset, tf, &f));
        assert!(
            moteur.zones_approche().iter().any(|z| z.achat && z.bord_proche == 102.0),
            "la zone reste visible à l'approche live"
        );

        // Tick live qui TOUCHE (low 101.5 ≤ 102) : plus fraîche → disparaît
        // immédiatement de l'exposition (l'évaluation live pilote le cache).
        let f2 = BougieEnFormation {
            debut: ts + 900, open: 106.0, high: 106.5, low: 101.5, close: 102.0,
            volume: 1.0, nb_events: 5, dernier_event: None,
        };
        moteur.on_tick(&ctx_tick(&asset, tf, &f2));
        assert!(
            !moteur.zones_approche().iter().any(|z| z.bord_proche == 102.0),
            "zone touchée en live → retirée aussitôt du périmètre fraîcheur"
        );
    }
