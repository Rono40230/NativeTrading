use actix_web::{web, HttpResponse, Responder};
use chrono::DateTime;
use serde::Deserialize;

use crate::state::AppState;

// ── Types de requête ────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ExportFiltres {
    pub limit: Option<i64>,
    pub strategie: Option<String>, // "Straddle" | "SmcDirectional" | "Rockets" | ""
    pub statut: Option<String>,    // "en_cours" | "clotures" | ""
    pub direction: Option<String>, // "LONG" | "SHORT" | ""
    pub asset: Option<String>,     // ex: "BTCUSDT" | ""
    pub verdict: Option<String>,   // "TP1" | "TP2" | "TP3" | "SL" | "expire" | ""
    pub depuis_ts: Option<i64>,    // timestamp unix — date début
    pub jusqu_ts: Option<i64>,     // timestamp unix — date fin
    pub separateur: Option<String>, // "," | ";" — défaut ","
}

// ── Helpers ─────────────────────────────────────────────────────────────────

fn ts_vers_date(ts: i64) -> String {
    DateTime::from_timestamp(ts, 0)
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_default()
}

fn echapper_csv(val: &str) -> String {
    if val.contains(',') || val.contains('"') || val.contains('\n') {
        format!("\"{}\"", val.replace('"', "\"\""))
    } else {
        val.to_string()
    }
}

fn reponse_csv(contenu: String, nom_fichier: &str) -> HttpResponse {
    // BOM UTF-8 pour compatibilité Excel français
    let mut body = "\u{FEFF}".to_string();
    body.push_str(&contenu);
    HttpResponse::Ok()
        .content_type("text/csv; charset=utf-8")
        .append_header((
            "Content-Disposition",
            format!("attachment; filename=\"{nom_fichier}\""),
        ))
        .body(body)
}

// ── POST /api/signaux/export ─────────────────────────────────────────────────

pub async fn exporter_signaux_csv(
    state: web::Data<AppState>,
    body: web::Json<ExportFiltres>,
) -> impl Responder {
    let sep = body.separateur.as_deref().unwrap_or(",");
    let strategie = body.strategie.as_deref().unwrap_or("");

    // Rockets → table séparée
    if strategie == "Rockets" {
        return exporter_rockets(&state, &body, sep).await;
    }

    let limit = body.limit.unwrap_or(2000).min(10000);
    let signaux = match state.db.obtenir_signaux(limit).await {
        Ok(s) => s,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": e.to_string() }))
        }
    };

    let colonnes = [
        "id",
        "asset",
        "timeframe",
        "direction",
        "score",
        "prix_entree",
        "stop_loss",
        "tp1",
        "tp2",
        "tp3",
        "strategie",
        "statut",
        "verdict",
        "prix_verdict",
        "llm_conviction",
        "date_signal",
    ];
    let mut csv = colonnes.join(sep) + "\n";

    for s in &signaux {
        // Filtres
        if !strategie.is_empty() && s["strategie"].as_str().unwrap_or("") != strategie {
            continue;
        }
        if let Some(ref dir) = body.direction {
            if !dir.is_empty() && s["direction"].as_str().unwrap_or("") != dir.as_str() {
                continue;
            }
        }
        if let Some(ref asset) = body.asset {
            if !asset.is_empty() && s["asset"].as_str().unwrap_or("") != asset.as_str() {
                continue;
            }
        }
        let verdict_val = s["verdict"].as_str().unwrap_or("");
        let statut_db = if verdict_val.is_empty() {
            "en_cours"
        } else {
            "clotures"
        };
        if let Some(ref st) = body.statut {
            if !st.is_empty() && statut_db != st.as_str() {
                continue;
            }
        }
        if let Some(ref v) = body.verdict {
            if !v.is_empty() && verdict_val != v.as_str() {
                continue;
            }
        }
        let cree_le = s["cree_le"].as_i64().unwrap_or(0);
        if let Some(dep) = body.depuis_ts {
            if cree_le < dep {
                continue;
            }
        }
        if let Some(jus) = body.jusqu_ts {
            if cree_le > jus {
                continue;
            }
        }

        let tp = s["take_profit"]
            .as_array()
            .and_then(|a| a.first())
            .and_then(|v| v.as_f64());
        let tp2 = s["take_profit"]
            .as_array()
            .and_then(|a| a.get(1))
            .and_then(|v| v.as_f64());
        let tp3 = s["take_profit"]
            .as_array()
            .and_then(|a| a.get(2))
            .and_then(|v| v.as_f64());

        let champs: Vec<String> = vec![
            echapper_csv(s["id"].as_str().unwrap_or("")),
            echapper_csv(s["asset"].as_str().unwrap_or("")),
            echapper_csv(s["timeframe"].as_str().unwrap_or("")),
            echapper_csv(s["direction"].as_str().unwrap_or("")),
            format!("{:.2}", s["score"].as_f64().unwrap_or(0.0)),
            format!("{:.5}", s["prix_entree"].as_f64().unwrap_or(0.0)),
            format!("{:.5}", s["stop_loss"].as_f64().unwrap_or(0.0)),
            tp.map_or(String::new(), |v| format!("{v:.5}")),
            tp2.map_or(String::new(), |v| format!("{v:.5}")),
            tp3.map_or(String::new(), |v| format!("{v:.5}")),
            echapper_csv(s["strategie"].as_str().unwrap_or("")),
            statut_db.to_string(),
            echapper_csv(verdict_val),
            s["prix_verdict"]
                .as_f64()
                .map_or(String::new(), |v| format!("{v:.5}")),
            s["llm_conviction"]
                .as_i64()
                .map_or(String::new(), |v| v.to_string()),
            ts_vers_date(cree_le),
        ];
        csv.push_str(&(champs.join(sep) + "\n"));
    }

    reponse_csv(csv, "signaux.csv")
}

// ── Export Rockets ───────────────────────────────────────────────────────────

async fn exporter_rockets(
    state: &web::Data<AppState>,
    body: &ExportFiltres,
    sep: &str,
) -> HttpResponse {
    let limit = body.limit.unwrap_or(2000).min(10000);
    let rockets = match state.db.lister_rockets_historique(limit).await {
        Ok(r) => r,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": e.to_string() }))
        }
    };

    let colonnes = [
        "id",
        "ticker",
        "phase",
        "score",
        "prix_entree",
        "stop_loss",
        "tp1",
        "tp2",
        "tp3",
        "statut",
        "verdict",
        "prix_verdict",
        "prix_peak",
        "llm_conviction",
        "date_signal",
    ];
    let mut csv = colonnes.join(sep) + "\n";

    for r in &rockets {
        // Filtres communs applicables aux Rockets
        if let Some(ref dir) = body.direction {
            if !dir.is_empty() {
                continue;
            } // Rockets n'ont pas de direction LONG/SHORT
        }
        let verdict_val = r.verdict.as_deref().unwrap_or("");
        let statut_str = if r.statut == "ferme" {
            "clotures"
        } else {
            "en_cours"
        };
        if let Some(ref st) = body.statut {
            if !st.is_empty() && statut_str != st.as_str() {
                continue;
            }
        }
        if let Some(ref v) = body.verdict {
            if !v.is_empty() && verdict_val != v.as_str() {
                continue;
            }
        }
        let cree_ts = DateTime::parse_from_rfc3339(&r.cree_le)
            .map(|dt| dt.timestamp())
            .unwrap_or(0);
        if let Some(dep) = body.depuis_ts {
            if cree_ts < dep {
                continue;
            }
        }
        if let Some(jus) = body.jusqu_ts {
            if cree_ts > jus {
                continue;
            }
        }

        let champs_r: Vec<String> = vec![
            r.id.to_string(),
            echapper_csv(&r.ticker),
            echapper_csv(&r.phase),
            format!("{:.2}", r.score),
            format!("{:.5}", r.prix_entree),
            format!("{:.5}", r.stop_loss),
            format!("{:.5}", r.target),
            r.target2.map_or(String::new(), |v| format!("{v:.5}")),
            r.target3.map_or(String::new(), |v| format!("{v:.5}")),
            statut_str.to_string(),
            echapper_csv(verdict_val),
            r.prix_verdict.map_or(String::new(), |v| format!("{v:.5}")),
            r.prix_peak.map_or(String::new(), |v| format!("{v:.5}")),
            r.llm_conviction.map_or(String::new(), |v| v.to_string()),
            r.cree_le.clone(),
        ];
        csv.push_str(&(champs_r.join(sep) + "\n"));
    }

    reponse_csv(csv, "rockets.csv")
}
