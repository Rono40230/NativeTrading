//! Endpoints d'activité du relais mobile (owner 21/10) : le « Rapport
//! d'activité » (miroir exact de la carte 📊 du dashboard) + le bandeau
//! marché du téléphone. Extraits de briefing_ia.rs (limite 600 lignes).


/// GET /api/mobile/activite — le « Rapport d'activité » (owner 21/10) :
/// EXACTEMENT la carte 📊 du dashboard (DashboardRapportActivite.vue) —
/// journal 24 h : Émis / Clôturés / Σ±R par stratégie sur les 150 derniers
/// signaux — + les positions ouvertes calculées À VIF (jamais le cache du
/// briefing — source de désynchro, owner 21/10).
pub async fn get_activite_mobile(
    state: actix_web::web::Data<crate::state::AppState>,
    requete: actix_web::HttpRequest,
) -> actix_web::HttpResponse {
    use sqlx::Row;
    if !crate::mobile_relay::autorise(&state.db, &requete).await {
        return crate::mobile_relay::non_autorise();
    }
    let pool = state.db.pool();

    // ① Rapport = le journal 24 h de la carte 📊 « Rapport d'activité » du
    // dashboard (DashboardRapportActivite.vue). MÊME source exacte :
    // obtenir_signaux(150) (= GET /api/signaux?limit=150) — r_distance y
    // est le R officiel calculé (paliers), jamais la colonne brute.
    // Émis : cree_le ≥ now−24 h ; Clôturés : verdict ≠ null && ferme_le ≥
    // now−24 h ; ±R : Σ r_distance des clôtures de la fenêtre.
    let journal = state.db.obtenir_signaux(150).await.unwrap_or_default();
    let maintenant = chrono::Utc::now().timestamp();
    let limite = maintenant - 86_400;
    let mut rapports: Vec<serde_json::Value> = Vec::new();
    for (id, nom, icone) in [
        ("SMC", "SMC", "📐"),
        ("straddle", "Straddle", "⚡"),
        ("rockets", "Rockets", "🚀"),
        ("kdj_halftrend", "KDJ", "📈"),
    ] {
        let mut emis = 0u32;
        let mut clots = 0u32;
        let mut r = 0.0;
        for s in &journal {
            if s["strategie"].as_str() != Some(id) {
                continue;
            }
            if s["cree_le"].as_i64().unwrap_or(0) >= limite {
                emis += 1;
            }
            if !s["verdict"].is_null() && s["ferme_le"].as_i64().unwrap_or(0) >= limite {
                clots += 1;
                r += s["r_distance"].as_f64().unwrap_or(0.0);
            }
        }
        rapports.push(serde_json::json!({
            "id": id, "nom": nom, "icone": icone,
            "emis": emis, "clotures": clots,
            "r": (r * 10.0).round() / 10.0,
        }));
    }

    // ② Positions ouvertes À VIF (statut Actif, remplies, sans verdict).
    // Règle EXACTE de l'app principale (useSignauxTableau.estEngage) :
    // statut ≠ Fermé ET engagé — SMC/KDJ : heure_entree posée ; Straddle :
    // heure E atteinte ; Rockets : dès le signal.
    let lignes = sqlx::query(
        "SELECT strategie, asset, direction, prix_entree AS e,
                COALESCE(sl_effectif, stop_loss) AS sl,
                sl_long_effectif AS sl_l, sl_short_effectif AS sl_s
         FROM signaux
         WHERE statut != 'Fermé'
           AND id IN (SELECT id FROM signaux ORDER BY cree_le DESC LIMIT 500) -- même fenêtre que l'app (getSignaux 500)
           AND ((strategie IN ('SMC','kdj_halftrend') AND heure_entree IS NOT NULL)
             OR (strategie = 'straddle' AND COALESCE(heure_entree, 0) <= strftime('%s','now'))
             OR strategie = 'rockets')
         ORDER BY strategie, asset",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    // Dernier prix par asset — chaîne complète (owner 22/10 : « vérifie
    // que toutes les stratégies sont branchées ») : bougies du pipeline
    // (KDJ/SMC/straddle) → Binance live (rockets USDT) → Yahoo live
    // (rockets actions US — même source que la gestion rockets). Le repli
    // silencieux sur le prix d'entrée est ce qui affichait P/L = 0.
    let mut derniers: std::collections::HashMap<String, f64> = std::collections::HashMap::new();
    for r in &lignes {
        let a: String = r.get("asset");
        if !derniers.contains_key(&a) {
            if let Ok(p) = sqlx::query("SELECT close FROM bougies WHERE asset = ? ORDER BY timestamp DESC LIMIT 1")
                .bind(&a)
                .fetch_one(pool)
                .await
            {
                derniers.insert(a.clone(), p.get::<f64, _>("close"));
            }
        }
    }
    let manquants: Vec<String> = lignes
        .iter()
        .map(|r| r.get::<String, _>("asset"))
        .filter(|a| !derniers.contains_key(a))
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    for a in &manquants {
        let prix = if a.ends_with("USDT") {
            crate::prix_utils::fetch_binance(&crate::http_client::HTTP_CLIENT, a).await
        } else {
            crate::yahoo_quotes::quotes(&[a.clone()]).await.get(a).map(|q| q.prix)
        };
        if let Some(p) = prix {
            derniers.insert(a.clone(), p);
        }
    }
    let risque_pct: f64 = state
        .db
        .lire_config("risque_trade")
        .await
        .ok()
        .flatten()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0.01);
    let mut capitaux: std::collections::HashMap<String, f64> = std::collections::HashMap::new();
    for m in crate::registre_strategies::MANIFESTES {
        let a = crate::analyses::analyser(&state.db, m.id).await;
        capitaux.insert(m.id.to_string(), a.capital_actuel.max(0.0));
    }

    // R flottant NET par position (owner 22/10 : colonne R + P/L pour
    // toutes les stratégies). Double (straddle) = SOMME DES JAMBES avec
    // le stop PROPRE à chaque jambe (doctrine du R net d'une passe) ;
    // dollars = capitaux × risque × R net.
    let en_cours: Vec<serde_json::Value> = lignes
        .iter()
        .map(|r| {
            let strat: String = r.get("strategie");
            let asset: String = r.get("asset");
            let dir: String = r.get("direction");
            let e: f64 = r.get("e");
            let sl: f64 = r.get("sl");
            let sl_l: Option<f64> = r.get("sl_l");
            let sl_s: Option<f64> = r.get("sl_s");
            let prix = *derniers.get(&asset).unwrap_or(&e);
            let ru = (e - sl).abs().max(1e-9);
            let r_net = match dir.as_str() {
                "Short" => (e - prix) / ru,
                "Both" => {
                    let ru_l = sl_l.map(|s| (e - s).abs()).filter(|x| *x > 1e-9).unwrap_or(ru);
                    let ru_s = sl_s.map(|s| (s - e).abs()).filter(|x| *x > 1e-9).unwrap_or(ru);
                    (prix - e) / ru_l + (e - prix) / ru_s
                }
                _ => (prix - e) / ru,
            };
            let d = (capitaux.get(&strat).copied().unwrap_or(0.0) * risque_pct * r_net * 100.0).round() / 100.0;
            serde_json::json!({
                "strategie": strat,
                "asset": asset,
                "sens": if dir == "Long" { "Achat" } else if dir == "Short" { "Vente" } else { "Double" },
                "dollars": d,
                "r": (r_net * 100.0).round() / 100.0,
            })
        })
        .collect();
    actix_web::HttpResponse::Ok().json(serde_json::json!({ "rapports": rapports, "en_cours": en_cours }))
}

/// GET /api/mobile/marche — bandeau du téléphone (owner 21→22/10) : MÊME
/// source que le ticker marchés du dashboard (PlancheMétéo → Yahoo Finance
/// via yahoo_live — décision owner : « les badges reflètent la réalité du
/// jour », donc S&P500/Nasdaq/Dow/pétrole servis en direct). Bitcoin reste
/// sur nos bougies Bybit (même source que le dashboard).
pub async fn get_marche_mobile(
    state: actix_web::web::Data<crate::state::AppState>,
    requete: actix_web::HttpRequest,
) -> actix_web::HttpResponse {
    use crate::sentiment_handlers::sentiment::yahoo_live;
    if !crate::mobile_relay::autorise(&state.db, &requete).await {
        return crate::mobile_relay::non_autorise();
    }
    let client = &*crate::http_client::HTTP_CLIENT;
    // (symbole Yahoo, nom affiché) — aligné sur SOURCES_YAHOO du dashboard.
    const SOURCES: &[(&str, &str)] = &[
        ("%5EGSPC", "S&P500"),
        ("%5EIXIC", "Nasdaq"),
        ("%5EDJI", "Dow Jones"),
        ("%5EGDAXI", "Dax"),
        ("GC%3DF", "Or"),
        ("SI%3DF", "Argent"),
        ("CL%3DF", "Pétrole"),
    ];
    let mut out: Vec<serde_json::Value> = Vec::new();
    for (symbole, nom) in SOURCES {
        if let Some(e) = yahoo_live(client, symbole, nom).await {
            out.push(serde_json::json!({
                "asset": e.nom,
                "prix": (e.prix * 100.0).round() / 100.0,
                "variation": (e.variation_pct * 100.0).round() / 100.0,
            }));
        }
    }
    // Bitcoin : nos bougies (Bybit), variation 24 h locale.
    let pool = state.db.pool();
    let dernier: Option<(f64,)> = sqlx::query_as(
        "SELECT close FROM bougies WHERE asset = 'BTC' AND timeframe = 'M5' ORDER BY timestamp DESC LIMIT 1",
    )
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();
    let veille: Option<(f64,)> = sqlx::query_as(
        "SELECT close FROM bougies WHERE asset = 'BTC' AND timeframe = 'H1' AND timestamp <= strftime('%s','now') - 82800 ORDER BY timestamp DESC LIMIT 1",
    )
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();
    if let (Some((prix,)), Some((ref_,))) = (dernier, veille) {
        let var = if ref_ != 0.0 { (prix / ref_ - 1.0) * 100.0 } else { 0.0 };
        out.push(serde_json::json!({
            "asset": "Bitcoin",
            "prix": (prix * 100.0).round() / 100.0,
            "variation": (var * 100.0).round() / 100.0,
        }));
    }
    actix_web::HttpResponse::Ok().json(serde_json::json!({ "actifs": out }))
}
