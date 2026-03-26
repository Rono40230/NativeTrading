/// Formate les résultats d'un backtest en bloc de contexte compact pour les prompts LLM.
///
/// Le LLM dispose ainsi d'une référence chiffrée sur les performances historiques
/// de la stratégie (win rate, Sharpe, drawdown, profit factor) pour calibrer
/// sa conviction lors du filtrage des signaux.
#[allow(clippy::too_many_arguments)]
pub fn formater_contexte_backtest(
    win_rate: f64,
    roi_pct: f64,
    sharpe_ratio: f64,
    max_drawdown_pct: f64,
    profit_factor: f64,
    total_trades: u32,
    asset: &str,
    strategie: &str,
) -> String {
    let qualite = if win_rate >= 55.0 && sharpe_ratio >= 1.0 {
        "STRATÉGIE PERFORMANTE"
    } else if win_rate >= 45.0 {
        "STRATÉGIE CORRECTE"
    } else {
        "STRATÉGIE À AMÉLIORER"
    };

    format!(
        "\n=== BACKTEST {asset} ({strategie}) — {qualite} ===\n\
        Trades: {total_trades} | WinRate: {win_rate:.1}% | ROI: {roi_pct:.2}%\n\
        Sharpe: {sharpe_ratio:.2} | MaxDD: {max_drawdown_pct:.1}% | PF: {profit_factor:.2}\n\
        Calibre ta conviction en conséquence : WinRate élevé → confiance possible, \
        WinRate faible ou MaxDD élevé → sois TRÈS conservateur.\n\
        ===\n"
    )
}

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
