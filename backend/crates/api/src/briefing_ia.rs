//! §Briefing du jour (13/10, owner — inspiration KTA #11) : l'IA synthétise
//! ce que l'app SAIT déjà mais qui vit en 4 endroits — événements armés du
//! jour (matrice), annonces High 24 h, positions ouvertes, presse marquante.
//! Les TABLEAUX factuels sont servis par l'app (zéro hallucination) ; le LLM
//! ne produit que la hiérarchie (« ce qui compte aujourd'hui »).
//! Cache du jour, prompt éditable `briefing_session`.

use std::sync::{Arc, Mutex, OnceLock};

use actix_web::{web, HttpResponse};
use serde::Serialize;
use sqlx::Row;

use crate::state::AppState;

#[derive(Debug, Clone, Serialize)]
pub struct EvenementJour {
    pub heure_paris: String,
    pub asset: String,
    pub evenement: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AnnonceJour {
    pub ts: i64,
    pub heure_paris: String,
    pub devise: String,
    pub titre: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PositionJour {
    pub strategie: String,
    pub asset: String,
    pub direction: String,
    pub entree: f64,
    pub stop_loss: f64,
    /// Heure d'entrée (epoch s) — 0 si absente.
    pub heure_entree: i64,
    /// R flottant de la jambe LONG (pour Both, l'UI montre les deux).
    pub r_actuel: f64,
    /// R flottant de la jambe SHORT (uniquement Both, sinon 0).
    pub r_court: f64,
    /// P&L flottant $ (risque$ stratégie × R) — None pour Both (jambes
    /// compensées en flottant, le réel vit dans les TP acquis).
    pub dollars: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct PresseJour {
    pub titre: String,
    pub score: i64,
    pub theme: String,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct SyntheseBriefing {
    pub resume: String,
    pub ce_qui_compte: Vec<String>,
    pub confiance: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct Briefing {
    pub evenements: Vec<EvenementJour>,
    pub annonces: Vec<AnnonceJour>,
    pub positions: Vec<PositionJour>,
    pub presse: Vec<PresseJour>,
    pub synthese: Option<SyntheseBriefing>,
    pub generee_le: i64,
}

fn cache() -> &'static Mutex<Option<Briefing>> {
    static C: OnceLock<Mutex<Option<Briefing>>> = OnceLock::new();
    C.get_or_init(|| Mutex::new(None))
}

fn jour_courant(ts: i64) -> String {
    chrono::DateTime::from_timestamp(ts, 0)
        .map(|t| t.with_timezone(&chrono::Local).format("%Y-%m-%d").to_string())
        .unwrap_or_default()
}

#[derive(serde::Deserialize, Default)]
pub struct CorpsBriefing {
    /// true = ↻ Régénérer : ignore le cache du jour (owner 14/10).
    #[serde(default)]
    pub forcer: bool,
}

/// POST /api/briefing/ia — sert le cache du jour ou régénère ({forcer:true}
/// pour forcer).
pub async fn post_briefing(state: web::Data<AppState>, corps: Option<web::Json<CorpsBriefing>>) -> HttpResponse {
    let forcer = corps.map(|c| c.forcer).unwrap_or(false);
    if !forcer {
        let g = cache().lock().unwrap_or_else(|p| p.into_inner());
        if let Some(b) = g.as_ref() {
            if jour_courant(b.generee_le) == jour_courant(chrono::Utc::now().timestamp()) {
                return HttpResponse::Ok().json(serde_json::json!({ "en_cache": true, "briefing": b }));
            }
        }
    }

    let mut b = collecter(&state).await;
    let contexte = contexte(&b);
    if contexte.lines().count() >= 2 {
        let prompt = format!("{}\n\n{}", llm::prompt_effectif("briefing_session"), contexte);
        if let Ok(texte) = llm::ollama::interroger(&prompt).await {
            b.synthese = Some(parser_synthese(&texte));
        }
    } else {
        b.synthese = Some(SyntheseBriefing {
            resume: "Journée sans événement armé ni position ouverte.".into(),
            ce_qui_compte: vec!["Rien d'urgent — aucune donnée du jour.".into()],
            confiance: 100,
        });
    }
    b.generee_le = chrono::Utc::now().timestamp();
    *cache().lock().unwrap_or_else(|p| p.into_inner()) = Some(b.clone());
    HttpResponse::Ok().json(serde_json::json!({ "en_cache": false, "briefing": b }))
}

/// Les données factuelles du jour — toutes locales, toutes réelles.
async fn collecter(state: &web::Data<AppState>) -> Briefing {
    let db = &state.db;
    let now = chrono::Utc::now();
    let maintenant = now.timestamp();
    // Journée Paris COMPLÈTE (owner 14/10 : événements passés inclus) —
    // bornée à 23:59:59 du jour, JAMAIS maintenant+24h (ce débordement
    // faisait apparaître les occurrences du lendemain en doublon).
    use chrono::TimeZone as _;
    let tz_paris = chrono_tz::Europe::Paris;
    let aujourdhui = now.with_timezone(&tz_paris).date_naive();
    // and_hms_opt(0/23h) est toujours Some pour des heures valides ; le
    // repli couvre le DST théorique (heure sautée).
    let minuit = aujourdhui.and_hms_opt(0, 0, 0).or(aujourdhui.and_hms_opt(1, 0, 0));
    let dernier = aujourdhui.and_hms_opt(23, 59, 59).or(aujourdhui.and_hms_opt(22, 59, 59));
    let (debut_jour, fin_jour) = (
        minuit.and_then(|n| tz_paris.from_local_datetime(&n).single())
            .map(|d| d.timestamp()).unwrap_or(maintenant - 12 * 3600),
        dernier.and_then(|n| tz_paris.from_local_datetime(&n).single())
            .map(|d| d.timestamp()).unwrap_or(maintenant + 12 * 3600),
    );

    // ── Annonces High des prochaines 24 h (collectées en premier : elles
    // alimentent leur table ET les lignes calendrier du tableau ci-dessous) ──
    let mut annonces: Vec<AnnonceJour> = Vec::new();
    if let Ok(rows) = db.lire_calendrier_cache(6 * 3600).await {
        for r in rows {
            let impact = r.get("impact").and_then(|v| v.as_str()).unwrap_or("");
            if impact != "High" {
                continue;
            }
            let Some(Ok(dt)) = r
                .get("date_heure")
                .and_then(|v| v.as_str())
                .map(|s| chrono::DateTime::parse_from_rfc3339(s))
            else {
                continue;
            };
            let t = dt.timestamp();
            if t < maintenant - 12 * 3600 || t > maintenant + 24 * 3600 {
                continue;
            }
            let p = dt.with_timezone(&chrono_tz::Europe::Paris);
            use chrono::Timelike;
            annonces.push(AnnonceJour {
                ts: t,
                heure_paris: format!("{:02}:{:02}", p.hour(), p.minute()),
                devise: r.get("devise").and_then(|v| v.as_str()).unwrap_or("?").to_string(),
                titre: r.get("titre").and_then(|v| v.as_str()).unwrap_or("?").to_string(),
            });
        }
        annonces.sort_by_key(|a| a.ts);
    }

    // ── Événements armés du jour = programme de DÉCLENCHEMENT complet
    // (owner 14/10) : structurels armés (heure catalogue) + cases cal_*
    // armées × annonces High réelles du jour (heure du calendrier). Une
    // ligne = un déclenchement possible de passe straddle aujourd'hui. ──
    let mut evenements: Vec<EvenementJour> = Vec::new();
    if let Ok(rows) = sqlx::query(
        "SELECT asset, evenement FROM creneaux_evenements WHERE arme = 1",
    )
    .fetch_all(db.pool())
    .await
    {
        use chrono::Timelike;
        let tz = chrono_tz::Europe::Paris;
        for r in &rows {
            let asset: String = r.get("asset");
            let ident: String = r.get("evenement");
            let Some(ev) = crate::evenements::catalogue().iter().find(|e| e.ident == ident) else {
                continue;
            };
            if ev.calendrier {
                // Famille CALENDRIER armée : une ligne par annonce High
                // réelle du jour classée dans CE type, à sa vraie heure.
                for a in &annonces {
                    if a.devise != "USD" {
                        continue;
                    }
                    if crate::evenements::classifie_annonce(&a.titre) == Some(ev.ident.as_ref()) {
                        evenements.push(EvenementJour {
                            heure_paris: a.heure_paris.clone(),
                            asset: asset.clone(),
                            evenement: ev.nom.to_string(),
                        });
                    }
                }
                continue;
            }
            // Occurrences du jour : fenêtres entre maintenant-12h (un événement
            // déjà commencé compte) et fin de journée. fenetres_evenement
            // renvoie CHAQUE MINUTE de la fenêtre de 3 min — on ne garde que
            // la PREMIÈRE minute de chaque occurrence (1 ligne par
            // asset × événement, pas 3).
            let mut derniere: i64 = 0;
            for ts in crate::evenements::fenetres_evenement(ev, debut_jour, fin_jour) {
                if ts - derniere < 180 {
                    continue; // minute 2/3 de la même occurrence
                }
                derniere = ts;
                let dt = chrono::DateTime::from_timestamp(ts, 0).unwrap_or_default();
                if (debut_jour..=fin_jour).contains(&ts) {
                    let p = dt.with_timezone(&tz);
                    evenements.push(EvenementJour {
                        heure_paris: format!("{:02}:{:02}", p.hour(), p.minute()),
                        asset: asset.clone(),
                        evenement: ev.nom.to_string(),
                    });
                }
            }
        }
        evenements.sort_by_key(|e| e.heure_paris.clone());
    }

    // Table « Annonces High — 24 h » : la fenêtre glissante à venir (les
    // annonces du jour déjà passées restent dans le tableau ci-dessus).
    let annonces: Vec<AnnonceJour> = annonces
        .into_iter()
        .filter(|a| a.ts > maintenant && a.ts <= maintenant + 24 * 3600)
        .collect();

    // ── Positions ouvertes ──
    let mut positions: Vec<PositionJour> = Vec::new();
    if let Ok(rows) = sqlx::query(
        "SELECT strategie, asset, direction, prix_entree, COALESCE(sl_effectif, stop_loss) AS sl,
                COALESCE(heure_entree, cree_le, 0) AS he
         FROM signaux WHERE statut = 'Actif' AND prix_entree IS NOT NULL ORDER BY strategie, asset",
    )
    .fetch_all(db.pool())
    .await
    {
        // Dernier prix connu par asset + risque$ par stratégie (owner 14/10).
        let mut derniers: std::collections::HashMap<String, f64> = std::collections::HashMap::new();
        for r in &rows {
            let a: String = r.get("asset");
            if derniers.contains_key(&a) {
                continue;
            }
            if let Ok(p) = sqlx::query(
                "SELECT close FROM bougies WHERE asset = ? ORDER BY timestamp DESC LIMIT 1",
            )
            .bind(&a)
            .fetch_one(db.pool())
            .await
            {
                derniers.insert(a, p.get::<f64, _>("close"));
            }
        }
        let risque_pct: f64 = db
            .lire_config("risque_trade")
            .await
            .ok()
            .flatten()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.01);
        let mut capitaux: std::collections::HashMap<String, f64> = std::collections::HashMap::new();
        for m in crate::registre_strategies::MANIFESTES {
            let a = crate::analyses::analyser(db, m.id).await;
            capitaux.insert(m.id.to_string(), a.capital_actuel.max(0.0));
        }
        for r in &rows {
            let strat: String = r.get("strategie");
            let asset: String = r.get("asset");
            let direction: String = r.get("direction");
            let entree: f64 = r.get("prix_entree");
            let sl: f64 = r.get::<f64, _>("sl");
            let prix = *derniers.get(&asset).unwrap_or(&entree);
            let risque_unit = (entree - sl).abs().max(1e-9);
            let (r_long, r_short) = ((prix - entree) / risque_unit, (entree - prix) / risque_unit);
            let both = direction == "Both";
            let r_actuel = match direction.as_str() {
                "Short" => r_short,
                _ => r_long,
            };
            let risque_usd = capitaux.get(&strat).copied().unwrap_or(0.0) * risque_pct;
            positions.push(PositionJour {
                strategie: strat,
                asset,
                direction,
                entree,
                stop_loss: sl,
                heure_entree: r.get("he"),
                r_actuel,
                r_court: if both { r_short } else { 0.0 },
                dollars: if both { None } else { Some(r_actuel * risque_usd) },
            });
        }
    }

    // ── Presse marquante (owner 14/10 : 10 titres, FR en priorité, mix de
    // thèmes — le meilleur de chaque thème d'abord, puis remplissage au
    // score, pour ne pas afficher 10× actions US) ──
    let mut presse: Vec<PresseJour> = Vec::new();
    if let Ok(rows) = sqlx::query(
        "SELECT COALESCE(n.titre_fr, p.titre) AS titre, p.titre AS titre_vo, p.score, p.theme
         FROM presse_articles p LEFT JOIN news_traductions n ON n.hash_titre = p.hash_titre
         WHERE p.ajoute_le > ? AND p.score >= 60
         ORDER BY p.score DESC LIMIT 20",
    )
    .bind(maintenant - 24 * 3600)
    .fetch_all(db.pool())
    .await
    {
        // Titres sans traduction : traduits à la volée (DeepL sinon Ollama,
        // mis en cache dans news_traductions — la page Presse en profite).
        let mut tous: Vec<PresseJour> = Vec::with_capacity(rows.len());
        for r in &rows {
            let vo: String = r.get("titre_vo");
            let fr: String = r.get("titre");
            let titre = if fr != vo {
                fr // déjà traduit
            } else {
                news::news_traduction::traduire_avec_cache(db.pool(), &vo).await
            };
            tous.push(PresseJour { titre, score: r.get("score"), theme: r.get("theme") });
        }
        let mut vus: std::collections::HashSet<String> = std::collections::HashSet::new();
        for x in &tous {
            if vus.insert(x.theme.clone()) && presse.len() < 10 {
                presse.push(x.clone());
            }
        }
        for x in tous {
            if presse.len() >= 10 {
                break;
            }
            if !presse.iter().any(|y| y.titre == x.titre) {
                presse.push(x);
            }
        }
    }

    Briefing { evenements, annonces, positions, presse, synthese: None, generee_le: maintenant }
}

/// Le texte chiffré passé à l'analyste — un écho fidèle des données.
fn contexte(b: &Briefing) -> String {
    let mut l = Vec::new();
    if !b.evenements.is_empty() {
        l.push(format!(
            "ÉVÉNEMENTS ARMÉS DU JOUR (matrice straddle, heure de Paris) :\n{}",
            b.evenements
                .iter()
                .map(|e| format!("  {} — {} × {}", e.heure_paris, e.asset, e.evenement))
                .collect::<Vec<_>>()
                .join("\n")
        ));
    }
    if !b.annonces.is_empty() {
        l.push(format!(
            "ANNONCES HIGH DES PROCHAINES 24 H (heure de Paris) :\n{}",
            b.annonces
                .iter()
                .map(|a| format!("  {} — {} : {}", a.heure_paris, a.devise, a.titre))
                .collect::<Vec<_>>()
                .join("\n")
        ));
    }
    if !b.positions.is_empty() {
        l.push(format!(
            "POSITIONS OUVERTES :\n{}",
            b.positions
                .iter()
                .map(|p| {
                    let heure = chrono::DateTime::from_timestamp(p.heure_entree, 0)
                        .map(|t| t.with_timezone(&chrono::Local).format("%d/%m %H:%M").to_string())
                        .unwrap_or_else(|| "—".into());
                    let r = if p.direction == "Both" {
                        format!("L {:+.1} / S {:+.1}", p.r_actuel, p.r_court)
                    } else {
                        format!("{:+.1}", p.r_actuel)
                    };
                    format!("  {} {} {} — entrée {:.2} le {}, R flottant {}", p.strategie, p.asset, p.direction, p.entree, heure, r)
                })
                .collect::<Vec<_>>()
                .join("\n")
        ));
    }
    if !b.presse.is_empty() {
        l.push(format!(
            "PRESSE MARQUANTE (score /100, thème) :\n{}",
            b.presse
                .iter()
                .map(|p| format!("  [{}] ({}) {}", p.score, p.theme, p.titre))
                .collect::<Vec<_>>()
                .join("\n")
        ));
    }
    l.join("\n\n")
}

/// Extraction robuste { … } — repli : pas de synthèse plutôt qu'une synthèse
/// inventée.
fn parser_synthese(texte: &str) -> SyntheseBriefing {
    let n = texte.trim();
    let (d, f) = (n.find('{'), n.rfind('}'));
    if let (Some(d), Some(f)) = (d, f) {
        if f > d {
            #[derive(serde::Deserialize)]
            struct Rep {
                #[serde(default)] resume: String,
                #[serde(default)] ce_qui_compte: Vec<String>,
                #[serde(default)] confiance: f64,
            }
            if let Ok(v) = serde_json::from_str::<Rep>(&n[d..=f]) {
                return SyntheseBriefing {
                    resume: v.resume,
                    ce_qui_compte: v.ce_qui_compte,
                    confiance: crate::analyses_ia::confiance_normalisee(v.confiance),
                };
            }
        }
    }
    SyntheseBriefing { resume: n.chars().take(600).collect(), ce_qui_compte: vec![], confiance: 0 }
}
