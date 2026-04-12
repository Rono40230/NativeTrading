/// Formate les signaux passés en bloc de contexte compact injecté dans les prompts LLM.
///
/// Le LLM reçoit ainsi le "vécu" récent de la stratégie sur l'asset :
/// directions répétées, scores, prix d'entrée — pour éviter les doublons
/// et affiner son raisonnement en fonction des signaux déjà actifs.
pub fn formater_contexte_historique(
    asset: &str,
    strategie: &str,
    signaux: &[serde_json::Value],
) -> String {
    if signaux.is_empty() {
        return String::new();
    }

    let mut lignes = Vec::with_capacity(signaux.len());
    let maintenant = chrono::Utc::now().timestamp();

    for s in signaux {
        let direction = s["direction"].as_str().unwrap_or("?");
        let timeframe = s["timeframe"].as_str().unwrap_or("?");
        let score = s["score"].as_f64().unwrap_or(0.0);
        let entree = s["prix_entree"].as_f64().unwrap_or(0.0);
        let statut = s["statut"].as_str().unwrap_or("Actif");
        let cree_le = s["cree_le"].as_i64().unwrap_or(maintenant);

        let age_h = (maintenant - cree_le).max(0) / 3600;
        let age_label = if age_h == 0 {
            "< 1h".to_string()
        } else if age_h < 24 {
            format!("{}h", age_h)
        } else {
            format!("{}j", age_h / 24)
        };

        lignes.push(format!(
            "  • {} {} score={:.0} entrée={:.5} [{}  il y a {}]",
            direction, timeframe, score, entree, statut, age_label
        ));
    }

    format!(
        "\n=== HISTORIQUE RÉCENT {asset} ({strategie}) ===\n\
        {lignes}\n\
        Tiens compte de ces signaux passés : évite de répéter la même direction \
        si plusieurs sont encore actifs, et ajuste ta conviction en conséquence.\n\
        ===\n",
        asset = asset,
        strategie = strategie,
        lignes = lignes.join("\n"),
    )
}
