//! Endpoint de notification du service Android (owner 22/10) : le relais
//! en avant-plan du téléphone interroge /api/mobile/nouvelles chaque
//! minute ; le serveur ne renvoie QUE les alertes dont la famille est
//! cochée « mobile » dans la matrice des canaux (table alertes_canaux) et
//! dont l'id dépasse le curseur du service. Extrait de mobile_relay.rs
//! (limite 600 lignes).

use actix_web::{web, HttpResponse, Responder};
use sqlx::Row;

use crate::state::AppState;

#[derive(serde::Deserialize)]
pub struct QueryNouvelles {
    pub limite: Option<i64>,
    /// Curseur du service (dernier id vu). Absent = saut au présent :
    /// aucune alerte renvoyée, juste le curseur courant.
    pub apres_id: Option<i64>,
}

/// Famille de la matrice des canaux pour une ligne du journal. Les
/// signaux portent leur stratégie dans le payload (famille signaux_*).
fn famille_ligne(type_alerte: &str, famille_signal: Option<&str>) -> String {
    match type_alerte {
        "evenement" | "digest" => "briefing".into(),
        "presse" => "presse_100".into(),
        "smc_approche" => "smc_approche".into(),
        "smc_contact" => "smc_contact".into(),
        "prix" => "alerte_prix".into(),
        "sante" => "sante".into(),
        "signal" => format!("signaux_{}", famille_signal.unwrap_or("smc")),
        _ => "briefing".into(),
    }
}

/// GET /api/mobile/nouvelles?limite=&apres_id= — { alertes: […], dernier:
/// max_id }. Les alertes vont de la plus ancienne à la plus récente ;
/// « dernier » est TOUJOURS le max de la table (le curseur saute aussi les
/// id non notifiables, pour ne jamais les revoir).
pub async fn get_nouvelles(
    state: web::Data<AppState>,
    q: web::Query<QueryNouvelles>,
    requete: actix_web::HttpRequest,
) -> impl Responder {
    if !crate::mobile_relay::autorise(&state.db, &requete).await {
        return crate::mobile_relay::non_autorise();
    }
    let pool = state.db.pool();
    let limite = q.limite.unwrap_or(10).clamp(1, 50);
    let dernier: i64 = sqlx::query("SELECT COALESCE(MAX(id), 0) FROM mobile_alertes")
        .fetch_one(pool)
        .await
        .ok()
        .and_then(|r| r.try_get::<i64, _>(0).ok())
        .unwrap_or(0);

    let apres = match q.apres_id {
        Some(a) if a >= 0 => a,
        // Premier appel du service : curseur au présent, rien à notifier.
        _ => return HttpResponse::Ok().json(serde_json::json!({ "alertes": [], "dernier": dernier })),
    };

    // Familles cochées « mobile » — lu UNE fois par appel.
    let actives = crate::alertes_canaux::lire_tous(pool)
        .await
        .into_iter()
        .filter(|c| c.mobile)
        .map(|c| c.famille)
        .collect::<std::collections::HashSet<String>>();

    let champ = "id, ts, type, titre, asset, tf, png, \
                 json_extract(payload,'$.achat') AS achat, \
                 json_extract(payload,'$.famille') AS famille_signal";
    let rows = sqlx::query(&format!(
        "SELECT {champ} FROM mobile_alertes WHERE id > ? ORDER BY id ASC LIMIT ?"
    ))
    .bind(apres)
    .bind(limite * 4) // marge : on filtre par canal APRÈS la lecture
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let mut alertes: Vec<serde_json::Value> = Vec::new();
    for r in rows.iter() {
        let type_alerte = r.try_get::<String, _>("type").unwrap_or_default();
        let famille_signal = r.try_get::<Option<String>, _>("famille_signal").ok().flatten();
        if !actives.contains(&famille_ligne(&type_alerte, famille_signal.as_deref())) {
            continue;
        }
        alertes.push(serde_json::json!({
            "id": r.try_get::<i64, _>("id").unwrap_or_default(),
            "ts": r.try_get::<i64, _>("ts").unwrap_or_default(),
            "type": type_alerte,
            "titre": r.try_get::<String, _>("titre").unwrap_or_default(),
            "asset": r.try_get::<Option<String>, _>("asset").ok().flatten(),
            "tf": r.try_get::<Option<String>, _>("tf").ok().flatten(),
            "achat": r.try_get::<Option<i64>, _>("achat").ok().flatten() == Some(1),
            "png": r.try_get::<i64, _>("png").unwrap_or_default() == 1,
        }));
        if alertes.len() as i64 >= limite {
            break;
        }
    }
    HttpResponse::Ok().json(serde_json::json!({ "alertes": alertes, "dernier": dernier }))
}
