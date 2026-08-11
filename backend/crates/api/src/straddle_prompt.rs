//! Construction du prompt Straddle few-shot (injection de la mémoire historique
//! des feedbacks dans le prompt système). La constante `PROMPT_SIGNAL_STRADDLE`
//! (prompt système brut) a été déplacée vers le crate `llm` (phase 1.6b) — elle
//! n'était consommée que par `llm::prompt_effectif`.

// ── Prompt dynamique few-shot ─────────────────────────────────────────────────

/// Construit le prompt complet en injectant l'historique des feedbacks clôturés.
///
/// Structure :
///   [Prompt système]
///   === MÉMOIRE HISTORIQUE ===   (si >= 3 feedbacks disponibles)
///   [Lignes feedbacks]
///   === CONTEXTE ACTUEL ===
///   [ctx habituel]
pub fn construire_prompt_few_shot(
    prompt_systeme: &str,
    ctx: &str,
    feedbacks: &[db::straddle_feedback::StraddleFeedbackRow],
    asset: &str,
    categorie: &crate::straddle_categorisation::CategoriePic,
) -> String {
    if feedbacks.len() < 3 {
        // Pas assez d'historique → prompt classique sans mémoire
        return format!("{}\n\n{ctx}", prompt_systeme);
    }

    let nb = feedbacks.len();
    let nb_gagnants = feedbacks.iter().filter(|f| f.gagnant == Some(1)).count();
    let win_rate_pct = (nb_gagnants as f64 / nb as f64 * 100.0).round() as u64;

    let score_moyen_win = moyenne_scores(feedbacks, true);
    let score_moyen_lose = moyenne_scores(feedbacks, false);

    let mut lignes = String::new();
    for f in feedbacks {
        let date = chrono::DateTime::from_timestamp(f.timestamp_signal, 0)
            .unwrap_or_default()
            .format("%Y-%m-%d %H:%M")
            .to_string();
        let evt = f.evenement_nom.as_deref().unwrap_or("-");
        let verdict = f.verdict.as_deref().unwrap_or("?");
        let amplitude = f
            .amplitude_reelle_pct
            .map(|a| format!("{:+.1}%", a))
            .unwrap_or_else(|| "-".into());
        let duree = f
            .duree_trade_min
            .map(|d| format!("{}min", d))
            .unwrap_or_else(|| "-".into());
        let pnl = f
            .pnl_r
            .map(|r| format!("{:+.2}R", r))
            .unwrap_or_else(|| "-".into());
        let session_out = f.session_sortie.as_deref().unwrap_or("-");
        let notes = f.notes_trader.as_deref().map(|n| format!(" | note: {}", n)).unwrap_or_default();
        lignes.push_str(&format!(
            "  {} | {} | ratio={:.2} | score={:.1} → {} ({}) | {} | {} | session_sortie={}{}\n",
            date, evt, f.ratio_atr, f.score_llm, verdict, amplitude, duree, pnl, session_out, notes
        ));
    }

    let mut avertissement = String::new();
    if win_rate_pct < 40 {
        avertissement = format!(
            "\n⚠️  Catégorie en dérive ({win_rate_pct}% WR sur {} derniers) — seuil score minimum : 8.0/10\n",
            nb
        );
    }

    format!(
        "{prompt_systeme}\n\n\
=== MÉMOIRE HISTORIQUE ({nb} derniers signals \"{cat}\" sur {asset}) ===\n\
{lignes}\
Taux de succès : {nb_gagnants}/{nb} ({win_rate_pct}%) | \
Score LLM moyen gagnants : {score_moyen_win:.1} | perdants : {score_moyen_lose:.1}\
{avertissement}\n\
=== CONTEXTE ACTUEL ===\n\
{ctx}",
        cat = categorie.as_str(),
    )
}

fn moyenne_scores(feedbacks: &[db::straddle_feedback::StraddleFeedbackRow], gagnants: bool) -> f64 {
    let filtre: Vec<f64> = feedbacks
        .iter()
        .filter(|f| f.gagnant == Some(if gagnants { 1 } else { 0 }))
        .map(|f| f.score_llm)
        .collect();
    if filtre.is_empty() {
        0.0
    } else {
        filtre.iter().sum::<f64>() / filtre.len() as f64
    }
}
