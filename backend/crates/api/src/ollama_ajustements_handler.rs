use actix_web::{web, HttpResponse, Responder};

use crate::ollama;

/// Body — résultats backtest + paramètres actuels.
#[derive(serde::Deserialize)]
pub struct RequeteAjustements {
    pub asset: String,
    pub roi_pct: f64,
    pub win_rate: f64,
    pub max_drawdown_pct: f64,
    pub profit_factor: f64,
    pub sharpe_ratio: f64,
    pub tp_mult_1: Option<f64>,
    pub tp_mult_2: Option<f64>,
    pub tp_mult_3: Option<f64>,
    pub sl_mult: Option<f64>,
    pub seuil_atr: Option<f64>,
}

#[derive(serde::Deserialize)]
struct ReponseAjustements {
    tp_mult_1: f64,
    tp_mult_2: f64,
    tp_mult_3: f64,
    sl_mult: f64,
    seuil_atr: f64,
    raison: String,
}

// ─── POST /api/ia/ajustements ─────────────────────────────────────────────────
/// Demande au LLM d'optimiser les paramètres Straddle à partir des résultats backtest.
/// Réponse : `{ tp_mult_1, tp_mult_2, tp_mult_3, sl_mult, seuil_atr, raison }`
pub async fn ajustements(body: web::Json<RequeteAjustements>) -> impl Responder {
    let prompt = format!(
        "Tu es un quant expert en stratégies Straddle ATR pour le trading algorithmique.\n\
        Analyse les résultats de backtest suivants et propose des ajustements PRÉCIS des paramètres.\n\n\
        ## Résultats Backtest — {asset}\n\
        - ROI : {roi:.2}% (objectif ≥ 15%)\n\
        - Win Rate : {wr:.1}% (objectif ≥ 55%)\n\
        - Max Drawdown : {dd:.2}% (limite ≤ 20%)\n\
        - Profit Factor : {pf:.2} (objectif ≥ 1.5)\n\
        - Sharpe Ratio : {sharpe:.2} (objectif ≥ 1.5)\n\n\
        ## Paramètres actuels\n\
        - seuil_atr : {seuil_atr:.2} (ratio ATR/moyenne pour déclencher le straddle)\n\
        - tp_mult_1 : {tp1:.2} × ATR (premier take-profit = ⅓ position)\n\
        - tp_mult_2 : {tp2:.2} × ATR (deuxième take-profit = ⅓ position)\n\
        - tp_mult_3 : {tp3:.2} × ATR (troisième take-profit = ⅓ position)\n\
        - sl_mult   : {sl:.2} × ATR (stop-loss initial)\n\n\
        ## Règles d'ajustement\n\
        - seuil_atr : plage [1.2, 3.0] — augmenter si trop de faux signaux, baisser si trop peu de trades\n\
        - tp_mult_1 : plage [1.0, 4.0] — doit être < tp_mult_2\n\
        - tp_mult_2 : plage [2.0, 6.0] — doit être entre tp_mult_1 et tp_mult_3\n\
        - tp_mult_3 : plage [3.0, 10.0] — objectif max, doit être > tp_mult_2\n\
        - sl_mult   : plage [0.2, 1.5] — plus petit = SL plus serré = plus de SL touchés\n\
        - CONSERVER l'ordre : tp_mult_1 < tp_mult_2 < tp_mult_3\n\n\
        Réponds UNIQUEMENT avec un objet JSON valide, sans markdown, sans commentaires :\n\
        {{\"tp_mult_1\": X.X, \"tp_mult_2\": X.X, \"tp_mult_3\": X.X, \"sl_mult\": X.X, \"seuil_atr\": X.X, \"raison\": \"explication concise en 1-2 phrases\"}}",
        asset = body.asset,
        roi = body.roi_pct,
        wr = body.win_rate,
        dd = body.max_drawdown_pct,
        pf = body.profit_factor,
        sharpe = body.sharpe_ratio,
        seuil_atr = body.seuil_atr.unwrap_or(1.5),
        tp1 = body.tp_mult_1.unwrap_or(2.0),
        tp2 = body.tp_mult_2.unwrap_or(3.5),
        tp3 = body.tp_mult_3.unwrap_or(5.0),
        sl = body.sl_mult.unwrap_or(0.5),
    );

    let modele = std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "qwen2.5:14b".to_string());

    match ollama::interroger(&prompt).await {
        Ok(texte) => {
            let json_start = texte.find('{');
            let json_end = texte.rfind('}');
            let json_str = match (json_start, json_end) {
                (Some(s), Some(e)) if e > s => &texte[s..=e],
                _ => {
                    return HttpResponse::Ok().json(serde_json::json!({
                        "tp_mult_1": 2.0, "tp_mult_2": 3.5, "tp_mult_3": 5.0,
                        "sl_mult": 0.5, "seuil_atr": 1.5,
                        "raison": "Réponse LLM non parseable — paramètres par défaut conservés.",
                        "modele": modele,
                    }));
                }
            };

            match serde_json::from_str::<ReponseAjustements>(json_str) {
                Ok(mut params) => {
                    params.tp_mult_1 = params.tp_mult_1.clamp(1.0, 4.0);
                    params.tp_mult_2 = params.tp_mult_2.clamp(params.tp_mult_1 + 0.5, 6.0);
                    params.tp_mult_3 = params.tp_mult_3.clamp(params.tp_mult_2 + 0.5, 10.0);
                    params.sl_mult = params.sl_mult.clamp(0.2, 1.5);
                    params.seuil_atr = params.seuil_atr.clamp(1.2, 3.0);
                    HttpResponse::Ok().json(serde_json::json!({
                        "tp_mult_1": params.tp_mult_1,
                        "tp_mult_2": params.tp_mult_2,
                        "tp_mult_3": params.tp_mult_3,
                        "sl_mult":   params.sl_mult,
                        "seuil_atr": params.seuil_atr,
                        "raison":    params.raison,
                        "modele":    modele,
                    }))
                }
                Err(_) => HttpResponse::Ok().json(serde_json::json!({
                    "tp_mult_1": 2.0, "tp_mult_2": 3.5, "tp_mult_3": 5.0,
                    "sl_mult": 0.5, "seuil_atr": 1.5,
                    "raison": format!(
                        "Réponse LLM non structurée — paramètres par défaut conservés. (raw: {})",
                        &texte[..texte.len().min(200)]
                    ),
                    "modele": modele,
                })),
            }
        }
        Err(e) => HttpResponse::ServiceUnavailable().json(serde_json::json!({
            "error": format!("{}", e),
            "aide": "Lancez Ollama: ollama serve && ollama pull qwen2.5:14b"
        })),
    }
}
