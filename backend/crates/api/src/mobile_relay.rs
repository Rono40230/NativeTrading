//! §Relais mobile (MOB 17-18/10) — le journal serveur de l'app APK.
//! PHILOSOPHIE : relayer l'information (opportunités à VENIR), zéro action,
//! zéro post-mortem — la tour de contrôle reste l'app principale.
//!
//! Les alertes : ① digest 8h30 (High + briefing + nuit ; = heartbeat)
//! ② rappel T-10 min d'un
//! High ③ presse notée 100 (news fonda) ④ SMC approche de zone armée
//! (moteur alertes_zones, seuil 0,5 ATR, une par épisode, hystérésis 1) —
//! émis par alertes_zones::verifier ⑤ SMC contact (idem) ⑥ alertes prix —
//! émis par alertes_prix::verifier ⑦ santé fournisseurs — émis par
//! fraicheur_pipeline. La MUETTE 21h→8h est côté CLIENT : le serveur
//! écrit TOUT (journal = 24 h glissantes).

use std::collections::HashSet;
use std::sync::{Arc, Mutex, OnceLock};

use actix_web::{web, HttpResponse, Responder};
use chrono::{Timelike, TimeZone as _};
use sqlx::Row;

use crate::state::AppState;

// ── Émission ─────────────────────────────────────────────────────────────────

/// Écrit une alerte au journal et retourne son id (pour la capture PNG).
/// Les erreurs n'interrompent JAMAIS le moteur appelant.
#[allow(clippy::too_many_arguments)]
pub async fn emettre(
    db: &db::Database,
    type_alerte: &str,
    titre: &str,
    asset: Option<&str>,
    tf: Option<&str>,
    payload: serde_json::Value,
    png: bool,
) -> Option<i64> {
    let sql = "INSERT INTO mobile_alertes (ts, type, titre, asset, tf, payload, png)
               VALUES (strftime('%s','now'), ?, ?, ?, ?, ?, ?)";
    let res = sqlx::query(sql)
        .bind(type_alerte)
        .bind(titre)
        .bind(asset)
        .bind(tf)
        .bind(payload.to_string())
        .bind(if png { 1 } else { 0 })
        .execute(db.pool())
        .await;
    match res {
        Ok(r) => Some(r.last_insert_rowid()),
        Err(e) => {
            tracing::warn!("📱 Relais mobile — écriture {type_alerte} : {e}");
            None
        }
    }
}

/// Rend et range la capture PNG d'une alerte SMC (approche ④ / contact ⑤)
/// dans data/mobile_png/{id}.png — l'état du graphique AU MOMENT de
/// l'alerte, figé. Échec silencieux côté moteur (l'alerte reste valable).
pub async fn capturer_smc(db: &db::Database, id: i64, asset: &str, tf: &str, payload: &serde_json::Value) {
    let tf_code = crate::utils::parse_timeframe(tf);
    let bougies_db = db
        .obtenir_bougies(
            &common::Asset::nouveau(asset),
            &tf_code,
            120,
        )
        .await
        .unwrap_or_default();
    let bougies: Vec<crate::mobile_png::BougiePng> = bougies_db
        .iter()
        .map(|c| (c.timestamp.timestamp(), c.open, c.high, c.low, c.close))
        .collect();
    let params = crate::mobile_png::ParamsCapture {
        bougies,
        zone_haut: payload["zone_haut"].as_f64().unwrap_or_default(),
        zone_bas: payload["zone_bas"].as_f64().unwrap_or_default(),
        achat: payload["achat"].as_bool().unwrap_or(true),
        prix: payload["prix"].as_f64().unwrap_or_default(),
    };
    let Some(octets) = crate::mobile_png::rendre(&params) else {
        tracing::warn!("📱 Relais mobile — capture {id} : bougies insuffisantes");
        return;
    };
    let dossier = "data/mobile_png";
    if std::fs::create_dir_all(dossier).is_err() {
        tracing::warn!("📱 Relais mobile — dossier {dossier} illisible");
        return;
    }
    if let Err(e) = std::fs::write(format!("{dossier}/{id}.png"), &octets) {
        tracing::warn!("📱 Relais mobile — écriture capture {id} : {e}");
    }
}

// ── Cycle minute (digest + rappels) — appelé au tick 60 s ────────────────────

/// Digest du matin : 8h30 Paris, une fois par jour (redémarrage tardif =
/// servi en retard ; aussi le heartbeat du canal).
pub(crate) fn dans_fenetre_digest(heure_paris: u32, minute: u32) -> bool {
    heure_paris > 8 || (heure_paris == 8 && minute >= 30)
}

/// Rappel T-10 : l'annonce dans [now+8 ; now+12] min — le tick 60 s
/// garantit une visite dans la fenêtre.
pub(crate) fn dans_fenetre_rappel(ts_annonce: i64, maintenant: i64) -> bool {
    let d = ts_annonce - maintenant;
    (480..=720).contains(&d)
}

/// Le cycle minute : digest 8h30 + rappels T-10 des annonces High.
pub async fn cycle(db: &Arc<db::Database>) {
    let maintenant = chrono::Utc::now().timestamp();
    let paris = chrono::Utc::now().with_timezone(&chrono_tz::Europe::Paris);
    let debut_jour = paris
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .and_then(|n| chrono_tz::Europe::Paris.from_local_datetime(&n).single())
        .map(|d| d.timestamp())
        .unwrap_or(maintenant - 86_400);

    if dans_fenetre_digest(paris.hour(), paris.minute()) {
        peut_etre_digest(db, debut_jour).await;
    }
    rappels_t10(db, maintenant).await;
}

/// Ligne « pendant la nuit » — « Straddle · XAUUSD · Double » (+ heure digest).
fn titre_position_nuit(id_strategie: &str, asset: &str, direction: &str, he: i64, avec_heure: bool) -> String {
    let nom = crate::registre_strategies::MANIFESTES
        .iter()
        .find(|m| m.id == id_strategie)
        .map(|m| m.nom)
        .unwrap_or(id_strategie);
    let sens = match direction {
        "Long" => "Achat",
        "Short" => "Vente",
        _ => "Double",
    };
    let mut t = format!("{nom} · {asset} · {sens}");
    if let Some(d) = chrono::DateTime::from_timestamp(he, 0).filter(|_| avec_heure) {
        t.push_str(&format!(" · {}", d.with_timezone(&chrono_tz::Europe::Paris).format("%Hh%M")));
    }
    t
}

/// Positions du vécu ouvertes dans la fenêtre nuit, par stratégie (19/10).
async fn positions_nuit(
    pool: &sqlx::SqlitePool,
    debut: i64,
    fin: i64,
    avec_heure: bool,
) -> Vec<serde_json::Value> {
    sqlx::query(
        "SELECT strategie, asset, direction, COALESCE(heure_entree, cree_le, 0) AS he
         FROM signaux
         WHERE prix_entree IS NOT NULL
           AND COALESCE(heure_entree, cree_le, 0) >= ? AND COALESCE(heure_entree, cree_le, 0) < ?
         ORDER BY strategie, he",
    )
    .bind(debut)
    .bind(fin)
    .fetch_all(pool)
    .await
    .unwrap_or_default()
    .iter()
    .map(|r| {
        let he: i64 = r.try_get("he").unwrap_or_default();
        serde_json::json!({
            "ts": he,
            "titre": titre_position_nuit(
                &r.try_get::<String, _>("strategie").unwrap_or_default(),
                &r.try_get::<String, _>("asset").unwrap_or_default(),
                &r.try_get::<String, _>("direction").unwrap_or_default(),
                he,
                avec_heure,
            ),
        })
    })
    .collect()
}

/// Assemble et écrit le digest si absent du jour.
async fn peut_etre_digest(db: &Arc<db::Database>, debut_jour: i64) {
    let deja: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM mobile_alertes WHERE type = 'digest' AND ts >= ?",
    )
    .bind(debut_jour)
    .fetch_one(db.pool())
    .await
    .unwrap_or(0);
    if deja > 0 {
        return;
    }

    // Briefing factuel du jour (annonces High 24h + presse marquante) et
    // synthèse 🌍 — le digest la fait générer à 8h30 (déclencheur matinal).
    let briefing = crate::briefing_ia::briefing_du_jour(db).await;
    let synthese_cache = crate::briefing_ia::synthese_cache_du_jour();
    let annonces: Vec<serde_json::Value> = briefing
        .annonces
        .iter()
        .map(|a| {
            serde_json::json!({ "ts": a.ts, "heure_paris": a.heure_paris, "devise": a.devise, "titre": a.titre })
        })
        .collect();
    let presse: Vec<serde_json::Value> = briefing
        .presse
        .iter()
        .map(|p| serde_json::json!({ "titre": p.titre, "score": p.score, "theme": p.theme }))
        .collect();

    // Ligne « pendant la nuit » : uniquement les positions ouvertes
    // depuis 21h la veille, heure dans le titre (le détail du digest
    // n'affiche que le titre).
    let nuit = positions_nuit(
        db.pool(),
        debut_jour - 3 * 3600, // hier 21:00 Paris
        debut_jour + 8 * 3600 + 1800,
        true,
    )
    .await;

    let payload = serde_json::json!({
        "annonces": annonces,
        "presse": presse,
        "synthese": synthese_cache,
        "nuit": nuit,
        "date_paris": chrono::Utc::now().with_timezone(&chrono_tz::Europe::Paris).format("%d/%m/%Y").to_string(),
    });
    let titre = format!(
        "Briefing du matin {}",
        chrono::Utc::now().with_timezone(&chrono_tz::Europe::Paris).format("%d/%m")
    );
    emettre(db, "digest", &titre, None, None, payload, false).await;
    tracing::info!("📱 Relais mobile — briefing du matin servi (heartbeat)");
    crate::alertes_canaux::telegram_si_actif(
        db.pool(), "briefing",
        &format!("🌅 {titre} — briefing du jour prêt, ouvert dans l'app.")).await;
}

/// Rappels T-10 des annonces High du jour (dédoublonnés par titre du jour).
async fn rappels_t10(db: &Arc<db::Database>, maintenant: i64) {
    static EMIS: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    let emis = EMIS.get_or_init(|| Mutex::new(HashSet::new()));

    let rows = sqlx::query(
        "SELECT id, date_heure, devise, titre FROM calendrier_cache
         WHERE impact = 'High' AND fetched_at > strftime('%s','now') - 86400
         ORDER BY date_heure",
    )
    .fetch_all(db.pool())
    .await
    .unwrap_or_default();

    for r in rows {
        let Some(Ok(dt)) = r
            .try_get::<String, _>("date_heure")
            .ok()
            .map(|s| chrono::DateTime::parse_from_rfc3339(&s))
        else {
            continue;
        };
        let ts = dt.timestamp();
        if !dans_fenetre_rappel(ts, maintenant) {
            continue;
        }
        let titre: String = r.try_get("titre").unwrap_or_default();
        let cle = format!("{ts}|{titre}");
        {
            let g = emis.lock().unwrap_or_else(|p| p.into_inner());
            if g.contains(&cle) {
                continue;
            }
        }
        // Double garde DB (redémarrage dans la fenêtre de 4 min).
        let deja: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM mobile_alertes WHERE type = 'evenement' AND titre = ? AND ts > ?",
        )
        .bind(&format!("T-10 min · {titre}"))
        .bind(maintenant - 1800)
        .fetch_one(db.pool())
        .await
        .unwrap_or(0);
        if deja > 0 {
            continue;
        }
        let devise: String = r.try_get("devise").unwrap_or_default();
        let paris = dt.with_timezone(&chrono_tz::Europe::Paris);
        let payload = serde_json::json!({
            "ts": ts,
            "heure_paris": paris.format("%H:%M").to_string(),
            "devise": devise,
            "titre": titre,
            "rappel_min": 10,
        });
        emettre(
            db,
            "evenement",
            &format!("T-10 min · {titre} {}", paris.format("%H:%M")),
            None,
            None,
            payload,
            false,
        )
        .await;
        crate::alertes_canaux::telegram_si_actif(
            db.pool(), "evenement_t10",
            &format!("⏰ T-10 min · {titre} à {}", paris.format("%H:%M"))).await;
        emis.lock().unwrap_or_else(|p| p.into_inner()).insert(cle);
        tracing::info!("📱 Relais mobile — rappel T-10 : {titre}");
    }
}

// ── Endpoints (lecture seule — l'app ne commande rien) ───────────────────────

/// Token d'accès : vide = ouvert (réseau Tailscale privé par conception).
pub(crate) async fn autorise(db: &Arc<db::Database>, requete: &actix_web::HttpRequest) -> bool {
    let Ok(Some(token)) = db.lire_config("mobile_token").await else {
        return true;
    };
    if token.is_empty() {
        return true;
    }
    requete
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .map(|v| v == format!("Bearer {token}"))
        .unwrap_or(false)
}

pub(crate) fn non_autorise() -> HttpResponse {
    HttpResponse::Unauthorized().json(serde_json::json!({ "erreur": "token mobile requis" }))
}

/// GET /api/mobile/etat — test de connexion des Réglages + contrat muette.
pub async fn get_etat(state: web::Data<AppState>, requete: actix_web::HttpRequest) -> impl Responder {
    if !autorise(&state.db, &requete).await {
        return non_autorise();
    }
    HttpResponse::Ok().json(serde_json::json!({
        "ok": true,
        "heure_serveur": chrono::Utc::now().timestamp(),
        "muette": { "debut": 21, "fin": 8, "tz": "Europe/Paris" },
    }))
}

/// POST /api/mobile/brief — bouton « ⚡ Générer » du détail presse : produit
/// le MÊME contexte de marché que la page 📰 Revue de presse de l'app
/// principale (owner 17/10) — même génération, même archive, pas le
/// briefing de session de l'onglet Aujourd'hui.
pub async fn post_brief_presse(
    state: web::Data<AppState>,
    requete: actix_web::HttpRequest,
) -> impl Responder {
    if !autorise(&state.db, &requete).await {
        return non_autorise();
    }
    use crate::presse_handlers::{generer_et_archiver_brief, ErreurBrief};
    match generer_et_archiver_brief(&state.db).await {
        Ok((id, contenu, nb)) => HttpResponse::Ok().json(serde_json::json!({
            "id": id, "contenu": contenu, "nb_articles": nb,
        })),
        Err(ErreurBrief::AucunArticle) => HttpResponse::BadRequest().json(
            serde_json::json!({"erreur": "aucun article dans les dernières 24 h"}),
        ),
        Err(ErreurBrief::OllamaIndisponible) => HttpResponse::ServiceUnavailable().json(
            serde_json::json!({"erreur": "Ollama indisponible — réessayer plus tard"}),
        ),
        Err(ErreurBrief::Interne(e)) => {
            HttpResponse::InternalServerError().json(serde_json::json!({ "erreur": e }))
        }
    }
}

#[derive(serde::Deserialize)]
pub struct QueryJournal {
    limite: Option<i64>,
    avant_id: Option<i64>,
    /// Filtre par famille, une ou plusieurs séparées par virgules
    /// (« smc_approche,smc_contact ») — sinon toutes.
    #[serde(rename = "type")]
    type_alerte: Option<String>,
    /// Fenêtre glissante en minutes (défaut 24 h ; page SMC = 60).
    fenetre: Option<i64>,
}

/// Les 7 familles valides du filtre `type`.
const TYPES_ALERTES: &[&str] = &[
    "digest", "evenement", "presse", "signal", "smc_approche", "smc_contact", "prix", "sante",
];

/// GET /api/mobile/journal?limite=&avant_id=&type=&fenetre= — récentes
/// d'abord, fenêtre glissante, familles optionnelles ; asset/tf/achat (+
/// famille pour les signaux) rendus pour les pages et le notificateur.
pub async fn get_journal(
    state: web::Data<AppState>,
    q: web::Query<QueryJournal>,
    requete: actix_web::HttpRequest,
) -> impl Responder {
    if !autorise(&state.db, &requete).await {
        return non_autorise();
    }
    let limite = q.limite.unwrap_or(100).clamp(1, 500);
    let fenetre = q.fenetre.unwrap_or(1440).clamp(1, 1440) * 60;
    let types: Vec<String> = q
        .type_alerte
        .as_deref()
        .map(|s| {
            s.split(',')
                .map(str::trim)
                .filter(|t| TYPES_ALERTES.contains(t))
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    let in_sql = if types.is_empty() { String::new() }
        else { format!(" AND type IN ({})", vec!["?"; types.len()].join(",")) };
    let champ_type = "id, ts, type, titre, asset, tf, png, json_extract(payload,'$.achat') AS achat, json_extract(payload,'$.famille') AS famille_signal";
    let sql = format!(
        "SELECT {champ_type} FROM mobile_alertes
         WHERE (? IS NULL OR id < ?) AND ts > strftime('%s','now') - ?{in_sql}
         ORDER BY id DESC LIMIT ?"
    );
    let mut rq = sqlx::query(&sql).bind(q.avant_id).bind(q.avant_id).bind(fenetre);
    for t in &types {
        rq = rq.bind(t);
    }
    let rows = rq.bind(limite).fetch_all(state.db.pool()).await;
    let alertes: Vec<serde_json::Value> = rows
        .unwrap_or_default()
        .iter()
        .map(|r| {
            serde_json::json!({
                "id": r.try_get::<i64, _>("id").unwrap_or_default(),
                "ts": r.try_get::<i64, _>("ts").unwrap_or_default(),
                "type": r.try_get::<String, _>("type").unwrap_or_default(),
                "titre": r.try_get::<String, _>("titre").unwrap_or_default(),
                "asset": r.try_get::<Option<String>, _>("asset").ok().flatten(),
                "tf": r.try_get::<Option<String>, _>("tf").ok().flatten(),
                "achat": r.try_get::<Option<i64>, _>("achat").ok().flatten() == Some(1),
                "famille": r.try_get::<Option<String>, _>("famille_signal").ok().flatten(),
                "png": r.try_get::<i64, _>("png").unwrap_or_default() == 1,
            })
        })
        .collect();
    HttpResponse::Ok().json(serde_json::json!({ "alertes": alertes }))
}

/// GET /api/mobile/alerte/{id} — le payload complet d'une alerte.
pub async fn get_alerte(
    state: web::Data<AppState>,
    chemin: web::Path<i64>,
    requete: actix_web::HttpRequest,
) -> impl Responder {
    if !autorise(&state.db, &requete).await {
        return non_autorise();
    }
    let id = chemin.into_inner();
    let Ok(row) = sqlx::query(
        "SELECT id, ts, type, titre, asset, tf, payload, png FROM mobile_alertes WHERE id = ?",
    )
    .bind(id)
    .fetch_one(state.db.pool())
    .await
    else {
        return HttpResponse::NotFound().json(serde_json::json!({ "erreur": "alerte inconnue" }));
    };
    let payload: serde_json::Value = row
        .try_get::<String, _>("payload")
        .ok()
        .and_then(|p| serde_json::from_str(&p).ok())
        .unwrap_or(serde_json::Value::Null);
    HttpResponse::Ok().json(serde_json::json!({
        "id": id,
        "ts": row.try_get::<i64, _>("ts").unwrap_or_default(),
        "type": row.try_get::<String, _>("type").unwrap_or_default(),
        "titre": row.try_get::<String, _>("titre").unwrap_or_default(),
        "asset": row.try_get::<Option<String>, _>("asset").ok().flatten(),
        "tf": row.try_get::<Option<String>, _>("tf").ok().flatten(),
        "png": row.try_get::<i64, _>("png").unwrap_or_default() == 1,
        "payload": payload,
    }))
}

/// GET /api/mobile/aujourdhui — le contenu de l'onglet : High du jour
/// (countdown côté client) + positions ouvertes pendant la nuit. La
/// presse a son onglet dédié (/api/mobile/presse, owner 19/10). AUCUN
/// appel LLM ici (le digest 8h30 l'a faite générer).
pub async fn get_aujourdhui(state: web::Data<AppState>, requete: actix_web::HttpRequest) -> impl Responder {
    if !autorise(&state.db, &requete).await {
        return non_autorise();
    }
    let maintenant = chrono::Utc::now().timestamp();
    let paris = chrono::Utc::now().with_timezone(&chrono_tz::Europe::Paris);
    let debut_jour = paris
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .and_then(|n| chrono_tz::Europe::Paris.from_local_datetime(&n).single())
        .map(|d| d.timestamp())
        .unwrap_or(maintenant - 86_400);
    let fin_jour = debut_jour + 86_400;

    // High du jour ( passé proche inclus — owner 14/10 : journée complète ).
    let annonces: Vec<serde_json::Value> = sqlx::query(
        "SELECT date_heure, devise, titre FROM calendrier_cache
         WHERE impact = 'High' AND fetched_at > strftime('%s','now') - 86400
         ORDER BY date_heure",
    )
    .fetch_all(state.db.pool())
    .await
    .unwrap_or_default()
    .iter()
    .filter_map(|r| {
        let s = r.try_get::<String, _>("date_heure").ok()?;
        let dt = chrono::DateTime::parse_from_rfc3339(&s).ok()?;
        let ts = dt.timestamp();
        if ts < debut_jour || ts >= fin_jour {
            return None;
        }
        Some(serde_json::json!({
            "ts": ts,
            "heure_paris": dt.with_timezone(&chrono_tz::Europe::Paris).format("%H:%M").to_string(),
            "devise": r.try_get::<String, _>("devise").unwrap_or_default(),
            "titre": r.try_get::<String, _>("titre").unwrap_or_default(),
        }))
    })
    .collect();

    // Presse marquante : SUPPRIMÉE de l'onglet (owner 19/10) — vit
    // désormais dans l'onglet « Revue de presse » (/api/mobile/presse).

    // Ligne nuit : positions ouvertes 21h la veille → 8h30 (owner 19/10),
    // sans heure dans le titre car l'app l'affiche en colonne dédiée.
    let nuit = positions_nuit(
        state.db.pool(),
        debut_jour - 3 * 3600,
        debut_jour + 8 * 3600 + 1800,
        false,
    )
    .await;

    HttpResponse::Ok().json(serde_json::json!({
        "date_paris": paris.format("%d/%m/%Y").to_string(),
        "annonces": annonces,
        "nuit": nuit,
    }))
}

/// GET /api/mobile/alerte/{id}/png — la capture rendue à l'émission.
pub async fn get_alerte_png(
    state: web::Data<AppState>,
    chemin: web::Path<i64>,
    requete: actix_web::HttpRequest,
) -> impl Responder {
    if !autorise(&state.db, &requete).await {
        return non_autorise();
    }
    let id = chemin.into_inner();
    match std::fs::read(format!("data/mobile_png/{id}.png")) {
        Ok(octets) => HttpResponse::Ok()
            .content_type("image/png")
            .body(octets),
        Err(_) => HttpResponse::NotFound().json(serde_json::json!({ "erreur": "capture absente" })),
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fenetre_digest_8h30_paris() {
        assert!(!dans_fenetre_digest(8, 29), "avant 8h30 : rien");
        assert!(dans_fenetre_digest(8, 30), "8h30 pile : service");
        assert!(dans_fenetre_digest(12, 0), "midi (redémarrage tardif) : service en retard");
        assert!(!dans_fenetre_digest(7, 59));
    }

    #[test]
    fn titre_position_nuit_nom_sens_heure() {
        // 1_792_360_800 = 19/10/2026 00h00 Paris (passe straddle).
        assert_eq!(titre_position_nuit("straddle", "XAUUSD", "Both", 1_792_360_800, true),
            "Straddle · XAUUSD · Double · 00h00");
        assert_eq!(titre_position_nuit("SMC", "BTC", "Short", 0, false), "SMC · BTC · Vente");
        assert_eq!(titre_position_nuit("inconnue", "XRP", "Long", 0, false), "inconnue · XRP · Achat");
    }

    #[test]
    fn fenetre_rappel_t10() {
        let maintenant = 1_000_000i64;
        assert!(dans_fenetre_rappel(maintenant + 600, maintenant), "T-10 pile");
        assert!(dans_fenetre_rappel(maintenant + 481, maintenant), "T-8:01");
        assert!(!dans_fenetre_rappel(maintenant + 479, maintenant), "T-7:59 : trop tôt");
        assert!(!dans_fenetre_rappel(maintenant + 721, maintenant), "T-12:01 : passé");
        assert!(!dans_fenetre_rappel(maintenant - 60, maintenant), "événement déjà passé");
    }
}
