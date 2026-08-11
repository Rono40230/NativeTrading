//! Formatage des messages Telegram — extrait de telegram_worker.rs.
//! Aucune I/O ici : fonctions pures de mise en forme.

#[allow(clippy::too_many_arguments)]
pub fn formater_signal(
    strategie: &str,
    direction: &str,
    asset: &str,
    score: f64,
    prix_entree: f64,
    stop_loss: f64,
    take_profit: &[f64],
    llm_conviction: Option<f64>,
    llm_raison: Option<&str>,
    taille_pip: f64,
    pip_to_points: f64,
) -> String {
    let strat_lower = strategie.to_lowercase();
    let est_straddle = strat_lower.contains("straddle");
    let est_long = direction.to_uppercase().contains("LONG");

    let emoji = if strat_lower.contains("rocket") {
        "🚀"
    } else if est_straddle {
        "🌪️"
    } else {
        "📊"
    };

    let action = if est_straddle {
        format!("🌪️ 2 positions simultanées sur <b>{}</b>", asset)
    } else if est_long {
        format!("📈 J'achète <b>{}</b>", asset)
    } else {
        format!("📉 Je vends <b>{}</b>", asset)
    };

    let ref_prix = prix_entree;
    let tps = take_profit
        .iter()
        .enumerate()
        .map(|(i, &tp)| {
            let pct = (tp - ref_prix) / ref_prix * 100.0;
            let diff_pips = (tp - ref_prix).abs() / taille_pip;
            let diff_pts = diff_pips * pip_to_points;
            let signe = if pct >= 0.0 { "+" } else { "" };
            format!(
                "🎯 TP{}      {}  ({}{:.2}% | {:.1} pips | {:.0} pts)",
                i + 1,
                crate::telegram::fmt(tp, ref_prix),
                signe,
                pct,
                diff_pips,
                diff_pts
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let sl_pct = (stop_loss - ref_prix) / ref_prix * 100.0;
    let diff_sl_pips = (stop_loss - ref_prix).abs() / taille_pip;
    let diff_sl_pts = diff_sl_pips * pip_to_points;
    let sl_signe = if sl_pct >= 0.0 { "+" } else { "" };

    let mut texte = format!(
        "{emoji} <b>Stratégie {strategie}</b>\n\
        ⭐ Score <b>{score:.0}/100</b>\n\
        \n\
        {action}\n\
        \n\
        <code>📍 Entrée   {entree}\n\
🛑 Stop     {sl}  ({sl_signe}{sl_pct:.2}% | {:.1} pips | {:.0} pts)\n\
{tps}</code>",
        diff_sl_pips,
        diff_sl_pts,
        emoji = emoji,
        strategie = strategie,
        score = score,
        action = action,
        entree = crate::telegram::fmt(ref_prix, ref_prix),
        sl = crate::telegram::fmt(stop_loss, ref_prix),
        sl_signe = sl_signe,
        sl_pct = sl_pct,
        tps = tps,
    );

    if let Some(c) = llm_conviction {
        texte.push_str(&format!("\n💡 Conviction IA : {:.0}%", c));
    }
    if let Some(r) = llm_raison {
        if !r.is_empty() {
            let r_echappe = r.replace("<", "&lt;").replace(">", "&gt;");
            texte.push_str(&format!("\n📝 {}", r_echappe));
        }
    }
    texte
}

#[allow(clippy::too_many_arguments)]
pub fn formater_rocket(
    ticker: &str,
    phase: &str,
    score: i64,
    prix_entree: f64,
    stop_loss: f64,
    tp1: f64,
    tp2: Option<f64>,
    trailing_coeff: f64,
    llm_conviction: Option<i64>,
    llm_raison: Option<&str>,
    entree_limite: Option<f64>,
    entree_stop: Option<f64>,
    niveau_invalidation: Option<f64>,
    type_entree_rec: Option<&str>,
) -> String {
    let conviction = llm_conviction.unwrap_or(0);
    let score_composite = (score as f64 * 0.6 + conviction as f64 * 0.4).round() as i64;

    let phase_label = match phase {
        "breakout"     => "Breakout",
        "prelancement" => "Pré-lancement",
        "compression"  => "Compression",
        other          => other,
    };

    let ref_prix = entree_limite
        .and_then(|el| entree_stop.map(|es| el.min(es)))
        .unwrap_or(prix_entree);

    let entree_ideal_label = match type_entree_rec {
        Some("limite") => "Limite",
        Some(_) => "Stop",
        None => "—",
    };

    let raison = llm_raison.unwrap_or("—").replace("<", "&lt;").replace(">", "&gt;");
    let tp2_str = tp2
        .map(|t| crate::telegram::fmt(t, ref_prix))
        .unwrap_or_else(|| "—".to_string());

    let corps = format!(
        "Entrée Limite  : {el}\n\
Entrée Stop    : {es}\n\
Entrée idéale  : {ei}\n\
SL             : {sl}\n\
TP1            : {tp1}\n\
TP2            : {tp2}\n\
Invalidation   : {inv}\n\
\n\
Trailing TP3   : {trail}×",
        el    = entree_limite.map(|v| crate::telegram::fmt(v, ref_prix)).unwrap_or_else(|| crate::telegram::fmt(prix_entree, ref_prix)),
        es    = entree_stop.map(|v| crate::telegram::fmt(v, ref_prix)).unwrap_or_else(|| "—".to_string()),
        ei    = entree_ideal_label,
        sl    = crate::telegram::fmt(stop_loss, ref_prix),
        tp1   = crate::telegram::fmt(tp1, ref_prix),
        tp2   = tp2_str,
        inv   = niveau_invalidation.map(|v| crate::telegram::fmt(v, ref_prix)).unwrap_or_else(|| "—".to_string()),
        trail = trailing_coeff,
    );

    let mut texte = format!(
        "🚀 <b>Rocket — {ticker} · {phase}</b>\nScore : <b>{score}/100</b>\n\n<code>{corps}</code>\n📝 {raison}",
        ticker = ticker,
        phase  = phase_label,
        score  = score_composite,
        corps  = corps,
        raison = raison,
    );

    if let Some(c) = llm_conviction {
        texte.push_str(&format!("\n💡 Conviction IA : {}/100", c));
    }

    texte
}

/// Formate un message de pré-alerte Telegram.
/// Ton distinct : ⚠️ "setup en formation" vs 🚀 "signal actif".
pub fn formater_prealerte(
    strategie: &str,
    asset: &str,
    raison: &str,
    score_actuel: Option<f64>,
    evenement: Option<&str>,
    minutes_avant: Option<i64>,
) -> String {
    let (emoji, label) = if strategie.to_lowercase().contains("straddle") {
        ("🌪️", "Straddle")
    } else {
        ("📊", "SMC Directionnel")
    };

    let mut texte = format!(
        "⚠️ <b>Pré-alerte {label} — {asset}</b>\n{emoji} Setup en formation\n\n{raison}",
        label = label,
        asset = asset,
        emoji = emoji,
        raison = raison.replace("<", "&lt;").replace(">", "&gt;"),
    );

    if let Some(score) = score_actuel {
        texte.push_str(&format!("\n📈 Score actuel : <b>{:.0}/100</b>", score));
    }
    if let Some(ev) = evenement {
        let min = minutes_avant.unwrap_or(0);
        texte.push_str(&format!(
            "\n📅 Événement : <b>{}</b> dans {} min",
            ev.replace("<", "&lt;").replace(">", "&gt;"),
            min
        ));
    }
    texte.push_str("\n\n<i>Préparez-vous — pas encore d'entrée confirmée.</i>");
    texte
}
