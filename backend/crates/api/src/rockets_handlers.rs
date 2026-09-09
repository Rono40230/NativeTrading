//! Handlers Rockets (positions, historique). Les ex-handlers v1
//! (historique/actifs/suppression sur `rockets_signaux`, table vide) ont été
//! supprimés le 05/09 (§10) ; la config fantôme `rockets_config` le 09/09
//! (écrite par personne de vivant — purge ROADMAP §6) : les réglages réels
//! vivent dans `rockets_params` (`/api/rockets/params`, RocketsParamsCard).

use actix_web::{web, HttpResponse, Responder};
use crate::state::AppState;

/// GET /api/rockets/positions — poste d'observation des positions ouvertes
/// (décision 05/09 : LECTURE SEULE, le moteur décide à la clôture D1).
/// Deux sections — à risque et neutralisées — avec les métadonnées de
/// pilotage du Journal de Trading : qty au lot officiel, capital d'époque,
/// R1, trailing actif, prix de la vente partielle. Cours LIVE : les cryptos
/// sont rafraîchies côté front (Binance — CSP autorisé) ; pour les actions
/// (Tiingo D1), dernier close et close de la veille sont fournis ici.
pub async fn get_positions(state: web::Data<AppState>) -> impl Responder {
    use sqlx::Row as _;

    let db = &state.db;
    let params = crate::rockets_verticale::lire_params(db).await;
    let capital_courant =
        crate::capital_simule::capital_actuel(db, "rockets").await.unwrap_or(2000.0);

    let rows = match sqlx::query(
        "SELECT cle, symbole, entree, stop, r1, neutralise, trailing, prix_r1,
                ts_entree, qty, capital_epoque
         FROM rockets_positions WHERE fermee = 0 ORDER BY ts_entree",
    )
    .fetch_all(db.pool())
    .await
    {
        Ok(r) => r,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": e.to_string() }))
        }
    };

    // Cours live des positions actions (les cryptos sont rafraîchies côté
    // front).
    let tickers_actions: Vec<String> = rows
        .iter()
        .filter_map(|r| {
            let s: String = r.get("symbole");
            (!s.ends_with("USDT")).then_some(s)
        })
        .collect();
    let quotes_yahoo = crate::yahoo_quotes::quotes(&tickers_actions).await;

    let mut risque = Vec::new();
    let mut neutralisees = Vec::new();
    for r in rows {
        let symbole: String = r.get("symbole");
        let entree: f64 = r.get("entree");
        let stop: f64 = r.get("stop");
        let r1: f64 = r.get("r1");
        let neutralise = r.get::<i64, _>("neutralise") != 0;
        let dist = entree - stop;
        // Lot officiel : enregistré à l'ouverture ; repli pour les positions
        // antérieures à la colonne (dérivé au capital courant, ~identique —
        // aucune clôture entre-temps).
        let qty: f64 = r
            .try_get::<Option<f64>, _>("qty")
            .ok()
            .flatten()
            .filter(|q| *q > 0.0)
            .unwrap_or_else(|| {
                let capital = r
                    .try_get::<Option<f64>, _>("capital_epoque")
                    .ok()
                    .flatten()
                    .unwrap_or(capital_courant);
                let mut q = if dist > 0.0 {
                    capital * params.profil.fraction() / dist
                } else {
                    0.0
                };
                let plafond = capital * params.plafond_position_pct / 100.0;
                if entree > 0.0 {
                    q = q.min(plafond / entree);
                }
                q
            });
        let capital_epoque: f64 = r
            .try_get::<Option<f64>, _>("capital_epoque")
            .ok()
            .flatten()
            .unwrap_or(capital_courant);

        // Actions US : cours LIVE Yahoo (décision 06/09 — même source que le
        // Journal de Trading). La tendance est servie via la veille dérivée
        // (prix / (1 + variation du jour)) — le front calcule comme avant.
        // Les cryptos vivent en live côté front (Binance).
        let mut dernier_close: Option<f64> = None;
        let mut close_veille: Option<f64> = None;
        if !symbole.ends_with("USDT") {
            if let Some(q) = quotes_yahoo.get(&symbole) {
                dernier_close = Some(q.prix);
                if q.variation_jour_pct > -100.0 {
                    close_veille =
                        Some(q.prix / (1.0 + q.variation_jour_pct / 100.0));
                }
            }
        }

        let commune = serde_json::json!({
            "cle": r.get::<String, _>("cle"),
            "symbole": symbole,
            "univers": if symbole.ends_with("USDT") { "crypto" } else { "action" },
            "ouvert_le": r.get::<i64, _>("ts_entree"),
            "entree": entree,
            "stop": stop,
            "r1": r1,
            "qty": qty,
            "capital_epoque": capital_epoque,
            "risque_pct": params.profil.fraction() * 100.0,
            "montant": qty * entree,
            "dernier_close": dernier_close,
            "close_veille": close_veille,
        });
        let mut j = commune;
        if neutralise {
            if let Some(o) = j.as_object_mut() {
                o.insert(
                    "trailing".into(),
                    serde_json::json!(r.try_get::<Option<f64>, _>("trailing").ok().flatten()),
                );
                o.insert(
                    "prix_r1".into(),
                    serde_json::json!(r.try_get::<Option<f64>, _>("prix_r1").ok().flatten()),
                );
                o.insert("qty_restante".into(), serde_json::json!(qty / 2.0));
                o.insert("montant_restant".into(), serde_json::json!(qty / 2.0 * entree));
            }
            neutralisees.push(j);
        } else {
            risque.push(j);
        }
    }

    HttpResponse::Ok().json(serde_json::json!({
        "risque": risque,
        "neutralisees": neutralisees,
        "trailing_pct": params.trailing_pct,
    }))
}

/// GET /api/rockets/historique — trades clôturés dans la logique rocket :
/// positions fermées (verdict TS/SL, R réalisé, R1 encaissé, trailing final,
/// sommet, qty) jointes au signal officiel (classement /10, dates, journal
/// de bord par id). P/L $ = réalisé du trade (partie R1 + solde).
pub async fn get_historique(state: web::Data<AppState>) -> impl Responder {
    use sqlx::Row as _;

    let db = &state.db;
    let params = crate::rockets_verticale::lire_params(db).await;
    let capital_courant =
        crate::capital_simule::capital_actuel(db, "rockets").await.unwrap_or(2000.0);

    let rows = match sqlx::query(
        "SELECT p.cle, p.symbole, p.entree, p.stop, p.r1, p.neutralise, p.trailing,
                p.prix_r1, p.sommet, p.qty, p.capital_epoque, p.verdict, p.r_realise,
                p.prix_sortie, p.ts_entree,
                s.id AS signal_id, s.score, s.ferme_le AS ferme_signal
         FROM rockets_positions p
         LEFT JOIN signaux s ON s.cle_moteur = p.cle
         WHERE p.fermee = 1
         ORDER BY COALESCE(s.ferme_le, p.ts_entree / 1000) DESC",
    )
    .fetch_all(db.pool())
    .await
    {
        Ok(r) => r,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": e.to_string() }))
        }
    };

    let trades: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            let symbole: String = r.get("symbole");
            let entree: f64 = r.get("entree");
            let stop: f64 = r.get("stop");
            let neutralise = r.get::<i64, _>("neutralise") != 0;
            let verdict: Option<String> =
                r.try_get::<Option<String>, _>("verdict").ok().flatten();
            let prix_sortie: Option<f64> =
                r.try_get::<Option<f64>, _>("prix_sortie").ok().flatten();
            let prix_r1: Option<f64> = r.try_get::<Option<f64>, _>("prix_r1").ok().flatten();
            let dist = entree - stop;
            let qty: f64 = r
                .try_get::<Option<f64>, _>("qty")
                .ok()
                .flatten()
                .filter(|q| *q > 0.0)
                .unwrap_or_else(|| {
                    let capital = r
                        .try_get::<Option<f64>, _>("capital_epoque")
                        .ok()
                        .flatten()
                        .unwrap_or(capital_courant);
                    let mut q = if dist > 0.0 {
                        capital * params.profil.fraction() / dist
                    } else {
                        0.0
                    };
                    let plafond = capital * params.plafond_position_pct / 100.0;
                    if entree > 0.0 {
                        q = q.min(plafond / entree);
                    }
                    q
                });
            let gain_r1 = if neutralise {
                qty / 2.0 * (prix_r1.unwrap_or(r.get::<f64, _>("r1")) - entree)
            } else {
                0.0
            };
            let gain_solde = prix_sortie.map(|s| qty / 2.0 * (s - entree)).unwrap_or(0.0);
            let pl_dollars = if neutralise {
                gain_r1 + gain_solde
            } else {
                prix_sortie.map(|s| qty * (s - entree)).unwrap_or(0.0)
            };
            serde_json::json!({
                "cle": r.get::<String, _>("cle"),
                "signal_id": r.try_get::<Option<String>, _>("signal_id").ok().flatten(),
                "symbole": symbole,
                "univers": if symbole.ends_with("USDT") { "crypto" } else { "action" },
                "classement": r.try_get::<Option<f64>, _>("score").ok().flatten(),
                "ouvert_le": r.get::<i64, _>("ts_entree"),
                "ferme_le": r.try_get::<Option<i64>, _>("ferme_signal").ok().flatten(),
                "entree": entree,
                "stop": stop,
                "r1": r.get::<f64, _>("r1"),
                "qty": qty,
                "montant": qty * entree,
                "r1_encaisse": if neutralise { prix_r1 } else { None::<f64> },
                "trailing_final": r.try_get::<Option<f64>, _>("trailing").ok().flatten(),
                "sommet": r.try_get::<Option<f64>, _>("sommet").ok().flatten(),
                "prix_sortie": prix_sortie,
                "verdict": verdict,
                "r_realise": r.try_get::<Option<f64>, _>("r_realise").ok().flatten(),
                "pl_dollars": pl_dollars,
            })
        })
        .collect();

    HttpResponse::Ok().json(serde_json::json!({ "trades": trades }))
}
