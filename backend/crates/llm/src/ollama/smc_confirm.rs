use common::TradingError;

/// Confirmation LLM d'un signal SMC validé par `SmcDirectionalStrategy`.
/// Retourne le raisonnement si le LLM confirme (score_confiance ≥ 0.5), `None` sinon.
#[allow(clippy::too_many_arguments)]
pub async fn confirmer_signal_smc(
    asset: &str,
    timeframe: &str,
    score_smc: f64,
    direction: &str,
    prix_entree: f64,
    stop_loss: f64,
    take_profit: f64,
    confiance_ml: f64,
    atr: f64,
    kill_zone: bool,
    sweep: bool,
    contexte_historique: &str,
) -> Option<String> {
    let prompt = format!(
        "{contexte}{prompt}\n\nAsset: {asset} {tf}\nPrix actuel: {entree:.5} | ATR: {atr:.5}\n\
        kill_zone_active: {kz} | sweep_detecte: {sw}\n\
        Direction SMC: {dir} | Score SMC: {score:.1}/100\n\
        ML confiance: {ml:.1}% | SL: {sl:.5} | TP1: {tp:.5}",
        contexte = contexte_historique,
        prompt = crate::prompt_effectif("smc_signal"),
        asset = asset,
        tf = timeframe,
        entree = prix_entree,
        atr = atr,
        kz = kill_zone,
        sw = sweep,
        dir = direction,
        score = score_smc,
        ml = confiance_ml * 100.0,
        sl = stop_loss,
        tp = take_profit,
    );

    let texte = match super::interroger(&prompt).await {
        Ok(t) => t,
        Err(e) => {
            tracing::warn!("Ollama indisponible pour confirmation SMC: {}", e);
            return None;
        }
    };

    #[derive(serde::Deserialize)]
    struct ConfirmBrut {
        direction: String,
        score_confiance: f64,
        raisonnement: String,
    }

    let debut = texte.find('{').unwrap_or(0);
    let fin = texte.rfind('}').map(|i| i + 1).unwrap_or(texte.len());
    let Ok(brut) = serde_json::from_str::<ConfirmBrut>(&texte[debut..fin]) else {
        tracing::debug!(
            "LLM réponse non parsable: {}",
            &texte[..texte.len().min(200)]
        );
        return None;
    };

    if brut.direction == "Neutre" || brut.score_confiance < 0.5 {
        return None;
    }
    Some(brut.raisonnement)
}

/// Appelle Ollama pour confirmer/enrichir un signal SMC validé.
/// Timeout 45s — retourne "SMC Directionnel" si Ollama est indisponible.
pub async fn enrichir_signal_avec_ollama(
    asset: &str,
    timeframe: &str,
    signal: &strategies::Signal,
    bougies: &[common::Candle],
    contexte_historique: &str,
) -> &'static str {
    let atr_vals = indicators::calculer_atr(bougies, 14);
    let atr_val = atr_vals.last().copied().unwrap_or(0.0);
    let (score_total, kill_zone, sweep) = match smc::scorer(bougies) {
        Some(s) => (s.total, s.kill_zone_active, s.sweep_detecte),
        None => (signal.confiance * 100.0, false, false),
    };
    let dir = format!("{:?}", signal.direction);

    let confirmation = tokio::time::timeout(
        std::time::Duration::from_secs(45),
        confirmer_signal_smc(
            asset,
            timeframe,
            score_total,
            &dir,
            signal.prix_entree,
            signal.stop_loss,
            signal.take_profit,
            signal.confiance,
            atr_val,
            kill_zone,
            sweep,
            contexte_historique,
        ),
    )
    .await;

    match confirmation {
        Ok(Some(r)) => {
            tracing::debug!(
                "LLM confirmé {}/{}: {}",
                asset,
                timeframe,
                &r[..r.len().min(100)]
            );
            "SMC+IA"
        }
        Ok(None) => "SMC Directionnel",
        Err(_) => {
            tracing::warn!(
                "Timeout Ollama (45s) {}/{} — signal SMC conservé",
                asset,
                timeframe
            );
            "SMC Directionnel"
        }
    }
}

/// Erreur intentionnellement non utilisée pour satisfaire TradingError dans la signature.
#[allow(dead_code)]
fn _use_trading_error(_: TradingError) {}
