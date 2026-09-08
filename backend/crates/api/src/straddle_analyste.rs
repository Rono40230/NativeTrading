//! §3 Rôles IA — analyste des passes straddle (08/09).
//!
//! La gate 3 est journalisée (chaque passe close : annonce → fill → jambe →
//! verdict → R net). L'analyste lit TOUT l'historique des passes avec ses
//! KPI consolidés et explique ce qui marche / ce qui coince, avec des
//! propositions chiffrées pour la décision de passage Officielle (point à
//! ≥ 30 passes). Constitution : l'IA lit, juge, propose — elle ne décide
//! rien et ne filtre rien.
//!
//! Deux vitesses de fraîcheur : les **chiffres du dossier de décision**
//! (ΣR, n passes, par source) sont calculés EN DIRECT à chaque consultation
//! (SQL immédiat — une passe clôturée à l'instant apparaît aussitôt) ; seul
//! le **texte de l'analyste** vit en cache du jour (table `analyse_cache`,
//! pattern du Rapport d'activité — boot + premier accès + bouton ↻).

use std::collections::BTreeMap;
use std::sync::Arc;

use actix_web::{web, HttpResponse, Responder};
use chrono::TimeZone;
use sqlx::Row;

use crate::state::AppState;
use db::Database;

/// Jour courant (heure Paris) — clé du cache.
fn jour_paris() -> String {
    chrono::Utc::now()
        .with_timezone(&chrono_tz::Europe::Paris)
        .format("%Y-%m-%d")
        .to_string()
}

/// Agrégat des passes closes (source unique des KPI et du prompt).
struct Kpis {
    n: i64,
    somme_r: f64,
    gagnantes: i64,
    cout_tampon: f64,
    passes_tampon: i64,
    par_source: BTreeMap<String, (i64, f64, i64)>,
    par_asset: BTreeMap<String, (i64, f64, i64)>,
    par_verdict: BTreeMap<String, i64>,
    lignes: Vec<String>,
}

/// Charge les passes closes (vérité fraîche).
async fn charger_passes(db: &Database) -> Vec<sqlx::sqlite::SqliteRow> {
    sqlx::query(
        "SELECT asset, cle_moteur, direction, verdict, r_realise, heure_entree, ferme_le
         FROM signaux
         WHERE strategie = 'straddle' AND statut = 'Fermé' AND r_realise IS NOT NULL
         ORDER BY ferme_le",
    )
    .fetch_all(db.pool())
    .await
    .unwrap_or_default()
}

/// Compte une passe dans un regroupement (n, ΣR, gagnantes).
fn incr(m: &mut BTreeMap<String, (i64, f64, i64)>, cle: String, r: f64) {
    let e = m.entry(cle).or_insert((0, 0.0, 0));
    e.0 += 1;
    e.1 += r;
    if r > 0.0 {
        e.2 += 1;
    }
}

/// Agrège les passes — pur, réutilisé par la consultation (direct) et le
/// prompt de l'analyste.
fn agreger(rows: &[sqlx::sqlite::SqliteRow]) -> Kpis {
    let mut k = Kpis {
        n: rows.len() as i64, somme_r: 0.0, gagnantes: 0, cout_tampon: 0.0, passes_tampon: 0,
        par_source: BTreeMap::new(), par_asset: BTreeMap::new(), par_verdict: BTreeMap::new(),
        lignes: Vec::new(),
    };
    for r in rows {
        let asset: String = r.get("asset");
        let cle: String = r.get("cle_moteur");
        let verdict: String = r.get("verdict");
        let r_net = r.try_get::<f64, _>("r_realise").ok().unwrap_or(0.0);
        let direction: String = r.get("direction");
        let ferme = r.try_get::<Option<i64>, _>("ferme_le").ok().flatten().unwrap_or(0);

        // ts de l'annonce = 3ᵉ segment de la clé moteur `straddle-{asset}-{ts}-{jambe}`
        let ts = cle
            .strip_prefix("straddle-")
            .and_then(|s| s.split('-').nth(1))
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or(0);
        let paris = chrono::Utc
            .timestamp_opt(ts, 0)
            .single()
            .map(|d| d.with_timezone(&chrono_tz::Europe::Paris));
        let (date_s, heure, source) = match paris {
            Some(p) => {
                let h = p.format("%H%M").to_string();
                let src = if h.starts_with("14") {
                    "annonce US 14h30".to_string()
                } else if asset == "DAX" && (h.starts_with("08") || h.starts_with("09") || h.starts_with("10")) {
                    "ouverture DAX".to_string()
                } else {
                    format!("créneau {}h", p.format("%H"))
                };
                (p.format("%d/%m").to_string(), p.format("%Hh%M").to_string(), src)
            }
            None => ("?".to_string(), "?".to_string(), "?".to_string()),
        };
        let entree = r.try_get::<Option<i64>, _>("heure_entree").ok().flatten().unwrap_or(ts);
        let duree_min = if ferme > entree { (ferme - entree) / 60 } else { 0 };
        k.lignes.push(format!(
            "{} {} {} · {} · jambe {} · {} · {:+.2}R · {} min",
            date_s, heure, asset, source, direction, verdict, r_net, duree_min
        ));

        k.somme_r += r_net;
        if r_net > 0.0 {
            k.gagnantes += 1;
        }
        if verdict.eq_ignore_ascii_case("sl") && r_net < -1.0 {
            k.cout_tampon += -1.0 - r_net;
            k.passes_tampon += 1;
        }
        incr(&mut k.par_source, source, r_net);
        incr(&mut k.par_asset, asset, r_net);
        *k.par_verdict.entry(verdict.to_lowercase()).or_insert(0) += 1;
    }
    k
}

/// JSON du dossier de décision (chiffres).
fn json_dossier(k: &Kpis) -> serde_json::Value {
    serde_json::json!({
        "n_passes": k.n,
        "somme_r": k.somme_r,
        "gagnantes": k.gagnantes,
        "kpis": {
            "somme_r": k.somme_r,
            "gagnantes": k.gagnantes,
            "tampon_r": k.cout_tampon,
            "par_source": k.par_source.iter().map(|(c, v)| (c.clone(), serde_json::json!({"n": v.0, "r": v.1, "g": v.2}))).collect::<BTreeMap<_, _>>(),
        },
    })
}

/// Contexte textuel du prompt de l'analyste.
fn contexte_prompt(k: &Kpis) -> String {
    let fmt = |m: &BTreeMap<String, (i64, f64, i64)>| {
        m.iter().map(|(c, (nb, sr, g))| format!("{c}: {nb} passes, Σ{sr:+.2}R, {g} gagnantes")).collect::<Vec<_>>().join(" · ")
    };
    format!(
        "Effectif : {n} passes closes (règle des 30 trades : {regle}).\n\
         Global : Σ{somme_r:+.2}R nets, {gagnantes} gagnantes ({pct:.0} %), tampon payé {tampon:.2}R sur {pt} passe(s) SL.\n\
         Par source : {src}.\n\
         Par asset : {ast}.\n\
         Par verdict : {ver}.\n\
         Passes (date, heure, asset, source, jambe, verdict, R net, durée) :\n{lignes}",
        n = k.n,
        regle = if k.n >= 30 { "ATTEINTE" } else { "PAS ENCORE atteinte — rester descriptif" },
        somme_r = k.somme_r, gagnantes = k.gagnantes,
        pct = if k.n > 0 { 100.0 * k.gagnantes as f64 / k.n as f64 } else { 0.0 },
        tampon = k.cout_tampon, pt = k.passes_tampon,
        src = fmt(&k.par_source), ast = fmt(&k.par_asset),
        ver = k.par_verdict.iter().map(|(c, v)| format!("{c}:{v}")).collect::<Vec<_>>().join(" · "),
        lignes = k.lignes.join("\n"),
    )
}

/// Analyse LLM complète (KPI + avis) — pour le cache et le bouton ↻.
async fn calculer(db: &Database) -> serde_json::Value {
    let rows = charger_passes(db).await;
    if rows.is_empty() {
        return serde_json::json!({
            "n_passes": 0, "analyse": null,
            "message": "Aucune passe close — l'analyste vivra dès la première clôture"
        });
    }
    let k = agreger(&rows);
    let prompt = format!("{}\n\nContexte des passes closes :\n{}", llm::prompt_effectif("straddle_analyste"), contexte_prompt(&k));
    let analyse = match llm::ollama::interroger(&prompt).await {
        Ok(texte) => {
            let debut = texte.find('{').unwrap_or(0);
            let fin = texte.rfind('}').map(|i| i + 1).unwrap_or(texte.len());
            serde_json::from_str::<serde_json::Value>(&texte[debut..fin])
                .ok()
                .filter(|v| v.is_object())
                .unwrap_or_else(|| serde_json::json!({ "synthese": texte.trim(), "points_forts": [], "points_faibles": [], "recommandations": [], "confiance": null }))
        }
        Err(_) => serde_json::Value::Null,
    };
    let mut v = json_dossier(&k);
    v["analyse"] = analyse;
    v
}

/// Lit l'avis LLM du jour en cache : (analyse, calcule_le).
async fn lire_cache(db: &Database, jour: &str) -> Option<(serde_json::Value, i64)> {
    let row: Option<(String, i64)> = sqlx::query_as(
        "SELECT avis, calcule_le FROM analyse_cache WHERE strategie = 'straddle' AND jour = ?",
    )
    .bind(jour)
    .fetch_optional(db.pool())
    .await
    .ok()
    .flatten();
    row.and_then(|(a, le)| {
        let v: serde_json::Value = serde_json::from_str(&a).ok()?;
        Some((v.get("analyse").cloned().unwrap_or(serde_json::Value::Null), le))
    })
}

/// Calcule (LLM) puis écrit le cache du jour.
async fn calculer_et_cacher(db: &Database, jour: &str) -> serde_json::Value {
    let avis = calculer(db).await;
    let _ = sqlx::query(
        "INSERT INTO analyse_cache (strategie, jour, n_passes, avis, calcule_le)
         VALUES ('straddle', ?, ?, ?, strftime('%s','now'))
         ON CONFLICT(strategie, jour) DO UPDATE SET
            n_passes = excluded.n_passes, avis = excluded.avis, calcule_le = excluded.calcule_le",
    )
    .bind(jour)
    .bind(avis.get("n_passes").and_then(|v| v.as_i64()).unwrap_or(0))
    .bind(avis.to_string())
    .execute(db.pool())
    .await;
    avis
}

/// GET /api/straddle/analyste — chiffres EN DIRECT + avis du jour en cache.
/// Cache absent → calcul lancé en arrière-plan (les chiffres sont déjà là).
pub async fn consulter(state: web::Data<AppState>) -> impl Responder {
    let rows = charger_passes(&state.db).await;
    let mut v = if rows.is_empty() {
        serde_json::json!({ "n_passes": 0 })
    } else {
        json_dossier(&agreger(&rows))
    };
    let jour = jour_paris();
    match lire_cache(&state.db, &jour).await {
        Some((analyse, le)) => {
            v["analyse"] = analyse;
            v["calcule_le"] = le.into();
        }
        None => {
            v["analyse"] = serde_json::Value::Null;
            v["en_calcul"] = true.into();
            v["message"] = "Analyse du jour en calcul (~2 min) — les chiffres sont déjà à jour".into();
            let db = state.db.clone();
            let j = jour;
            tokio::spawn(async move {
                calculer_et_cacher(&db, &j).await;
            });
        }
    }
    HttpResponse::Ok().json(v)
}

/// POST /api/straddle/analyste/rafraichir — force le recalcul (bouton ↻).
pub async fn rafraichir(state: web::Data<AppState>) -> impl Responder {
    let jour = jour_paris();
    let mut avis = calculer_et_cacher(&state.db, &jour).await;
    avis["calcule_le"] = chrono::Utc::now().timestamp().into();
    HttpResponse::Ok().json(avis)
}

/// Au boot : s'assure que l'avis du jour existe (arrière-plan, non bloquant).
pub async fn assurer_cache(db: Arc<Database>) {
    let jour = jour_paris();
    if lire_cache(&db, &jour).await.is_none() {
        calculer_et_cacher(&db, &jour).await;
    }
}
