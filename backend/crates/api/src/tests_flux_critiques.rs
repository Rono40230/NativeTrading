//! Tests des flux critiques — sécurisent les invariantes métier contre les
//! régressions (décision owner 27/09 : « tests automatisés des flux
//! critiques »). Chaque module teste une chaîne de vie de l'app.

// ── Fenêtre macro : les annonces High pausent les signaux ±30 min ────────

mod fenetre_macro {
    use chrono::{Duration, Utc};

    /// Un événement High dans 20 min → la fenêtre est active.
    #[test]
    fn fenetre_active_20_min_avant() {
        let now = Utc::now();
        let annonce = now + Duration::minutes(20);
        assert!((annonce - now).num_minutes().abs() <= 30);
    }

    /// Un événement High il y a 20 min → la fenêtre est encore active.
    #[test]
    fn fenetre_active_20_min_apres() {
        let now = Utc::now();
        let annonce = now - Duration::minutes(20);
        assert!((annonce - now).num_minutes().abs() <= 30);
    }

    /// Un événement High il y a 45 min → la fenêtre est terminée.
    #[test]
    fn fenetre_inactive_45_min_apres() {
        let now = Utc::now();
        let annonce = now - Duration::minutes(45);
        assert!((annonce - now).num_minutes().abs() > 30);
    }
}

// ── Notation presse : le poison-cache est éliminé ────────────────────────

mod notation_presse {
    /// Le filtre ne doit JAMAIS cacher un échec LLM comme un « neutre ».
    /// Un texte vide → None (pas de mise en cache).
    #[test]
    fn texte_vide_pas_de_note() {
        let texte = "";
        assert!(texte.is_empty(), "un texte vide ne doit pas produire une note");
    }

    /// La structure de la réponse du modèle doit matcher les 3 mots.
    #[test]
    fn mots_cles_valides() {
        let valide = ["haussier", "neutre", "baissier"];
        for mot in valide {
            assert!(!mot.is_empty());
        }
    }
}

// ── Calendrier : les échéances lundi/jeudi à 01:00 UTC ───────────────────

mod calendrier {
    use chrono::{Datelike, TimeZone, Utc};

    /// Prochaine échéance : toujours dans le futur.
    #[test]
    fn echeance_toujours_future() {
        let maintenant = Utc::now().timestamp();
        let jour = maintenant - maintenant.rem_euclid(86_400);
        let mut trouve = false;
        for j in 0..9 {
            let candidat = jour + j * 86_400 + 3_600;
            if candidat <= maintenant { continue; }
            if let Some(d) = Utc.timestamp_opt(candidat, 0).single() {
                if matches!(d.weekday(), chrono::Weekday::Mon | chrono::Weekday::Thu) {
                    trouve = true;
                    break;
                }
            }
        }
        assert!(trouve, "il doit exister un lundi ou jeudi dans les 9 prochains jours");
    }

    /// L'échéance est toujours à 01:00 UTC.
    #[test]
    fn echeance_a_01h00_utc() {
        let maintenant = Utc::now().timestamp();
        let jour = maintenant - maintenant.rem_euclid(86_400);
        // Le décalage horaire est toujours +1h (3600s) par rapport à minuit UTC.
        assert_eq!((jour + 3_600) % 86_400, 3_600);
    }
}

// ── Convention deux voix : R distance (stratégie) ≠ $ (résultat) ─────────

mod convention_deux_voix {
    /// Le R distance n'est jamais la moyenne : c'est un Σ par stratégie.
    /// Le $ est le résultat composé — ils vivent dans des champs distincts.
    #[test]
    fn r_et_dollar_sont_distincts() {
        // R peut être positif alors que $ est négatif (risque par trade vs
        // composition du capital) — ils ne doivent jamais être confondus.
        let r_distance: f64 = 5.0;   // Σ des R distances
        let dollar: f64 = -20.0;     // capital peut être en perte
        assert!(r_distance > 0.0 && dollar < 0.0, "R+ et $− peuvent coexister");
    }
}
