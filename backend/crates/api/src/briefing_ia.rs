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

use crate::briefing_synthese::{parser_synthese, SectionBriefing, SyntheseBriefing};
use crate::state::AppState;

pub use crate::briefing_synthese::SyntheseBriefing as SyntheseBriefingPub;

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
    pub url: String,
    pub score: i64,
    pub theme: String,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct NuitBriefing {
    /// Lignes « strategie asset verdict +xR » des clôtures 21h→8h.
    pub clotures: Vec<String>,
    /// ΣR des clôtures de la nuit (somme des r_realise).
    pub somme_r: f64,
    /// Lignes « HH:MM type — titre » des alertes de la nuit.
    pub alertes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Briefing {
    pub evenements: Vec<EvenementJour>,
    pub annonces: Vec<AnnonceJour>,
    pub positions: Vec<PositionJour>,
    pub presse: Vec<PresseJour>,
    pub nuit: NuitBriefing,
    /// Régime du jour, pré-formaté (« F&G 68 (appêt) · presse 48 h : … »).
    pub regime: Option<String>,
    pub synthese: Option<SyntheseBriefing>,
    pub generee_le: i64,
}

fn cache() -> &'static Mutex<Option<Briefing>> {
    static C: OnceLock<Mutex<Option<Briefing>>> = OnceLock::new();
    C.get_or_init(|| Mutex::new(None))
}

/// Vide le cache du jour (pipeline owner 15/10 : l'univers des événements
/// armés/positions change ⇒ le briefing se régénère à la demande suivante).
pub(crate) fn vider_cache() {
    *cache().lock().unwrap_or_else(|p| p.into_inner()) = None;
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
        let frais = {
            let g = cache().lock().unwrap_or_else(|p| p.into_inner());
            g.as_ref()
                .filter(|b| jour_courant(b.generee_le) == jour_courant(chrono::Utc::now().timestamp()))
                .map(|b| (b.synthese.clone(), b.generee_le))
        };
        if let Some((synthese, generee_le)) = frais {
            // Owner 23/10 : les TABLEAUX factuels sont recalculés à chaque
            // ouverture (ils ne mentent jamais) ; seule la synthèse LLM vit
            // en cache du jour.
            let mut vif = collecter(&state.db).await;
            vif.synthese = synthese;
            vif.generee_le = generee_le;
            return HttpResponse::Ok().json(serde_json::json!({ "en_cache": true, "briefing": vif }));
        }
    }

    let b = generer(&state.db).await;
    HttpResponse::Ok().json(serde_json::json!({ "en_cache": false, "briefing": b }))
}

/// Génère le briefing du jour (collecte factuelle + synthèse LLM) et pose
/// le cache. Partagé POST /api/briefing/ia et digest 8h30 du relais mobile.
pub(crate) async fn generer(db: &Arc<db::Database>) -> Briefing {
    let mut b = collecter(db).await;
    let contexte = contexte(&b, chrono::Utc::now().timestamp());
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
            sections: vec![],
        });
    }
    b.generee_le = chrono::Utc::now().timestamp();
    *cache().lock().unwrap_or_else(|p| p.into_inner()) = Some(b.clone());
    b
}

/// Briefing du jour : cache frais sinon génération — le digest 8h30 du
/// relais mobile est le déclencheur matinal (chantier MOB owner 17/10).
pub(crate) async fn briefing_du_jour(db: &Arc<db::Database>) -> Briefing {
    {
        let g = cache().lock().unwrap_or_else(|p| p.into_inner());
        if let Some(b) = g.as_ref() {
            if jour_courant(b.generee_le) == jour_courant(chrono::Utc::now().timestamp()) {
                return b.clone();
            }
        }
    }
    generer(db).await
}

/// Synthèse 🌍 du jour si le cache est frais — LECTURE seule (aucune
/// génération : l'onglet mobile « Aujourd'hui » ne doit jamais payer un LLM).
pub(crate) fn synthese_cache_du_jour() -> Option<SyntheseBriefing> {
    let g = cache().lock().unwrap_or_else(|p| p.into_inner());
    g.as_ref()
        .filter(|b| jour_courant(b.generee_le) == jour_courant(chrono::Utc::now().timestamp()))
        .and_then(|b| b.synthese.clone())
}

/// Les données factuelles du jour — toutes locales, toutes réelles.
async fn collecter(db: &Arc<db::Database>) -> Briefing {
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
        let catalogue = crate::evenements_catalogue::lire(db).await;
        for r in &rows {
            let asset: String = r.get("asset");
            let ident: String = r.get("evenement");
            let Some(ev) = catalogue.iter().find(|e| e.ident == ident && e.actif) else {
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

    // ── Positions ouvertes — RÈGLE D'ENGAGEMENT EXACTE du dashboard
    // (owner 23/10 : l'ancienne règle statut='Actif' comptait les ORDRES EN
    // ATTENTE comme positions — un setup SMC DAX jamais rempli depuis 10
    // jours apparaissait comme position ouverte). SMC/KDJ : entrée touchée
    // (heure_entree posée) ; straddle : heure E atteinte ; rockets : tout.
    let mut positions: Vec<PositionJour> = Vec::new();
    if let Ok(rows) = sqlx::query(
        "SELECT strategie, asset, direction, prix_entree, COALESCE(sl_effectif, stop_loss) AS sl,
                COALESCE(heure_entree, 0) AS he, COALESCE(score, 10) AS score
         FROM signaux
         WHERE statut != 'Fermé'
           AND id IN (SELECT id FROM signaux ORDER BY cree_le DESC LIMIT 500)
           AND ((strategie IN ('SMC','kdj_halftrend') AND heure_entree IS NOT NULL)
             OR (strategie = 'straddle' AND COALESCE(heure_entree, 0) <= strftime('%s','now'))
             OR strategie = 'rockets')
         ORDER BY strategie, asset",
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
        let palier_faible = crate::reglages_smc::lire_palier(db, "smc_sl_palier_faible", 0.7).await;
        let palier_fort = crate::reglages_smc::lire_palier(db, "smc_sl_palier_fort", 1.3).await;
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
            // Option C (owner 19/10) : le risque$ SMC est modulé par le palier
            // de score — même facteur que la distance SL (taille constante).
            let palier = if strat == "SMC" {
                crate::reglages_smc::palier_pure(
                    r.get::<f64, _>("score"),
                    palier_faible,
                    palier_fort,
                )
            } else {
                1.0
            };
            let risque_usd = capitaux.get(&strat).copied().unwrap_or(0.0) * risque_pct * palier;
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

    // ── Presse marquante (owner 23/10 : 4 sections — macro, crypto,
    // métaux, techno (actions US) — × 5 meilleurs titres, FR en priorité) ──
    let mut presse: Vec<PresseJour> = Vec::new();
    for theme in ["macro", "crypto", "metaux", "actions_us"] {
        if let Ok(rows) = sqlx::query(
            "SELECT COALESCE(NULLIF(n.titre_fr, ''), p.titre) AS titre, p.titre AS titre_vo, p.url, p.score, p.theme
             FROM presse_articles p LEFT JOIN news_traductions n ON n.hash_titre = p.hash_titre
             WHERE p.ajoute_le > ? AND p.theme = ?
             ORDER BY p.score DESC LIMIT 5",
        )
        .bind(maintenant - 24 * 3600)
        .bind(theme)
        .fetch_all(db.pool())
        .await
        {
            // Titres sans traduction : traduits à la volée (DeepL sinon
            // Ollama, mis en cache — la page Presse en profite).
            for r in &rows {
                let vo: String = r.get("titre_vo");
                let fr: String = r.get("titre");
                let trad = if fr != vo { fr } else {
                    news::news_traduction::traduire_avec_cache(db.pool(), &vo).await
                };
                // Garde-fou : traduction vide = VO affichée (owner 23/10 —
                // la colonne Techno affichait des titres absents).
                let titre = if trad.trim().is_empty() { vo.clone() } else { trad };
                presse.push(PresseJour { titre, url: r.get("url"), score: r.get("score"), theme: r.get("theme") });
            }
        }
    }

    // ── Nuit 21h→8h (owner 23/10, section ① du briefing) : clôtures et
    // alertes survenues pendant la muette ──
    let mut nuit = NuitBriefing::default();
    // Fenêtre MUETTE exacte : hier 21h00 → aujourd'hui 08h00, Paris.
    let huit_heures = aujourdhui.and_hms_opt(8, 0, 0).or(aujourdhui.and_hms_opt(9, 0, 0));
    let fin_nuit = huit_heures
        .and_then(|n| tz_paris.from_local_datetime(&n).single())
        .map(|d| d.timestamp())
        .unwrap_or(maintenant - 6 * 3600);
    let debut_nuit = fin_nuit - 11 * 3600;
    if let Ok(rows) = sqlx::query(
        "SELECT strategie, asset, verdict, r_realise FROM signaux
         WHERE ferme_le BETWEEN ? AND ?",
    )
    .bind(debut_nuit)
    .bind(fin_nuit)
    .fetch_all(db.pool())
    .await
    {
        for r in &rows {
            let rr: Option<f64> = r.get("r_realise");
            nuit.somme_r += rr.unwrap_or(0.0);
            nuit.clotures.push(format!(
                "{} {} — {} ({:+.1} R)",
                r.get::<String, _>("strategie"),
                r.get::<String, _>("asset"),
                r.get::<String, _>("verdict"),
                rr.unwrap_or(0.0)
            ));
        }
    }
    if let Ok(rows) = sqlx::query(
        "SELECT ts, type, titre FROM mobile_alertes WHERE ts BETWEEN ? AND ? ORDER BY ts",
    )
    .bind(debut_nuit)
    .bind(fin_nuit)
    .fetch_all(db.pool())
    .await
    {
        for r in &rows {
            let ts: i64 = r.get("ts");
            let heure = chrono::DateTime::from_timestamp(ts, 0)
                .map(|t| t.with_timezone(&chrono::Local).format("%H:%M").to_string())
                .unwrap_or_default();
            nuit.alertes.push(format!("{} {} — {}", heure, r.get::<String, _>("type"), r.get::<String, _>("titre")));
        }
    }

    // ── Régime du jour (section ④) : F&G + compteurs presse 48 h, via le
    // bandeau sentiment (même source que la planche météo) ──
    let mut regime: Option<String> = None;
    let bandeau = crate::sentiment_bandeau::collecter(db).await;
    if bandeau.fng.is_some() || bandeau.presse.is_some() {
        let mut morceaux: Vec<String> = Vec::new();
        if let Some(fng) = bandeau.fng {
            morceaux.push(format!("Fear & Greed crypto : {}", fng.valeur));
        }
        // Clôtures de la veille (owner 23/10 : détailler le régime) —
        // dernier jour figé de la référence sentiment.
        if let Ok(rows) = sqlx::query(
            "SELECT entite, prix, variation_pct FROM sentiment_marche_veille
             WHERE date = (SELECT MAX(date) FROM sentiment_marche_veille)",
        )
        .fetch_all(db.pool())
        .await
        {
            for r in &rows {
                let entite: String = r.get("entite");
                if ["Or", "Argent", "S&P500", "Nasdaq", "Dow Jones", "Dax", "VIX", "Bitcoin"].contains(&entite.as_str()) {
                    morceaux.push(format!(
                        "{} {:.2} ({:+.1} %)",
                        entite,
                        r.get::<f64, _>("prix"),
                        r.get::<f64, _>("variation_pct")
                    ));
                }
            }
        }
        if let Some(p) = bandeau.presse {
            morceaux.push(format!(
                "Presse 48 h notée : {} haussiers / {} neutres / {} baissiers",
                p.haussier, p.neutre, p.baissier
            ));
        }
        regime = Some(morceaux.join(" · "));
    }

    Briefing { evenements, annonces, positions, presse, nuit, regime, synthese: None, generee_le: maintenant }
}

/// Nom d'affichage d'une stratégie (owner 23/10 : l'identifiant brut
/// kdj_halftrend fuitait dans le briefing).
fn nom_strategie(id: &str) -> &str {
    match id {
        "kdj_halftrend" => "KDJ",
        autre => autre,
    }
}

/// Le texte chiffré passé à l'analyste — un écho fidèle des données.
fn contexte(b: &Briefing, maintenant: i64) -> String {
    let mut l = Vec::new();
    if !b.nuit.clotures.is_empty() || !b.nuit.alertes.is_empty() {
        let mut lignes: Vec<String> = Vec::new();
        // Plafonds : le contexte doit tenir en 60 secondes de lecture —
        // au-delà, on résume (nombre + échantillon).
        let clotures_txt = if b.nuit.clotures.len() <= 10 {
            b.nuit.clotures.join(" ; ")
        } else {
            format!("{} clôtures — exemples : {}", b.nuit.clotures.len(), b.nuit.clotures[..5].join(" ; "))
        };
        lignes.push(format!("Clôtures de la nuit (21h→8h) : {} — ΣR {:+.1}.", clotures_txt, b.nuit.somme_r));
        let alertes_txt = if b.nuit.alertes.len() <= 8 {
            if b.nuit.alertes.is_empty() { "aucune".into() } else { b.nuit.alertes.join(" ; ") }
        } else {
            format!("{} alertes — exemples : {}", b.nuit.alertes.len(), b.nuit.alertes[..5].join(" ; "))
        };
        lignes.push(format!("Alertes de la nuit : {alertes_txt}"));
        l.push(format!("NUIT (21h→8h) :\n{}", lignes.iter().map(|x| format!("  {x}")).collect::<Vec<_>>().join("\n")));
    }
    if let Some(r) = &b.regime {
        l.push(format!("RÉGIME DU MARCHÉ :\n  {r}"));
    }
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
                    let age_h = if p.heure_entree > 0 { (maintenant - p.heure_entree) / 3600 } else { 0 };
                    let age_txt = format!("{} j {:02} h", age_h / 24, age_h % 24);
                    format!("  {} {} {} — entrée {:.2} le {}, R flottant {}, âge {}", nom_strategie(&p.strategie), p.asset, p.direction, p.entree, heure, r, age_txt)
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
