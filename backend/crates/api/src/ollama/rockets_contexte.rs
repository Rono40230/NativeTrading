//! Formatage du contexte LLM pour le filtre Rockets.
//! Séparé de rockets_filtre.rs pour respecter la limite de 300 lignes.
use super::rockets_filtre::SignalCandidat;
use db::rockets::RocketSignal;
use db::rockets_feedback::RocketsFeedbackRow;

/// Formate le bloc signal courant + historique ticker pour le prompt LLM.
pub fn formater_contexte(candidat: &SignalCandidat, historique: &[RocketSignal]) -> String {
    // Contexte marché global
    let (nb_48h, wr_48h, pnl_48h) = candidat.tendance_marche_48h;
    let contexte_marche = if nb_48h >= 5 {
        let alerte = if wr_48h < 0.40 {
            " ⚠️ MARCHÉ DÉFAVORABLE : relever seuil de conviction"
        } else if wr_48h >= 0.65 {
            " ✅ MARCHÉ FAVORABLE : conditions optimales pour Rockets"
        } else {
            ""
        };
        format!(
            "Marché global 48h : {} trades | WR={:.0}% | PnL moyen={:.2}R{}",
            nb_48h,
            wr_48h * 100.0,
            pnl_48h,
            alerte,
        )
    } else {
        "Marché global 48h : données insuffisantes (<5 trades)".to_string()
    };

    let mut ctx = format!(
        "=== SIGNAL CANDIDAT : {} ===\n\
        Session : {} | {}\n\
        Phase: {} | Score: {}/100\n\
        Prix entrée: {:.6} | SL: {:.6} | TP1: {:.6}\n\
        Entrée limite: {:.6} | Entrée stop: {:.6} | Invalidation: {:.6}\n\
        Type entrée algo: {} \n\
        ATR14: {:.6} | ATR ratio (accélération): {:.2} | ATR50 (référence): {:.6}\n\
        Volume ratio: {:.2}× | RSI: {:.1} | Change 1h: {:.2}%\n\
        Ratio corps/mèche bougie: {:.2} (1.0=pleine, <0.3=rejet probable)\n\
        Tendance préalable (EMA20>EMA50): {} | Compression: {} bougies | Hauteur base: {:.6}\n\
        VCP — Volume assèchement: {:.2} (<0.75=✅) | Qualité contraction: {:.2} (>0.70=✅)\n\n",
        candidat.ticker,
        candidat.session,
        contexte_marche,
        candidat.phase,
        candidat.score,
        candidat.prix_entree,
        candidat.stop_loss,
        candidat.tp1,
        candidat.entree_limite,
        candidat.entree_stop,
        candidat.niveau_invalidation,
        candidat.type_entree_rec_algo,
        candidat.atr14,
        candidat.atr_ratio,
        candidat.atr50,
        candidat.ratio_volume,
        candidat.rsi,
        candidat.change1h,
        candidat.ratio_corps,
        if candidat.tendance_haussiere { "✅ oui" } else { "❌ non" },
        candidat.nb_bougies_compression,
        candidat.hauteur_base,
        candidat.volume_seche,
        candidat.contraction_qualite,
    );

    // Série des swings : permet au LLM de vérifier la décroissance réelle du VCP
    if candidat.swing_amplitudes.len() >= 2 {
        let serie: Vec<String> = candidat
            .swing_amplitudes
            .iter()
            .map(|a| format!("{:.6}", a))
            .collect();
        let premier = candidat.swing_amplitudes.first().copied().unwrap_or(0.0);
        let dernier = candidat.swing_amplitudes.last().copied().unwrap_or(0.0);
        let vraiment_decroissant = candidat
            .swing_amplitudes
            .windows(2)
            .all(|w| w[1] <= w[0] * 0.92);
        ctx.push_str(&format!(
            "Structure compression (swing amplitudes, ancienne→récente) : [{}]\n\
            Réduction totale : {:.0}% | VCP décroissant strict : {}\n\n",
            serie.join(" → "),
            if premier > 0.0 { (1.0 - dernier / premier) * 100.0 } else { 0.0 },
            if vraiment_decroissant { "✅ oui" } else { "⚠️ partiel" },
        ));
    }

    // Cohérence SL / Invalidation : alerte si la configuration est illogique
    {
        let sl = candidat.stop_loss;
        let inv = candidat.niveau_invalidation;
        let coherent = inv < sl; // invalidation doit être SOUS le stop-loss
        let distance_sl_pct = if candidat.prix_entree > 0.0 {
            (candidat.prix_entree - sl) / candidat.prix_entree * 100.0
        } else {
            0.0
        };
        ctx.push_str(&format!(
            "Cohérence SL/Invalidation : {} (invalidation={:.6} {} stop-loss={:.6}) | Distance entrée→SL : {:.2}%\n\n",
            if coherent { "✅ logique" } else { "⚠️ INCOHÉRENT (invalider)" },
            inv,
            if coherent { "<" } else { "≥" },
            sl,
            distance_sl_pct,
        ));
    }

    if historique.is_empty() {
        return ctx;
    }

    ctx.push_str(&format!(
        "=== HISTORIQUE {} ({} trades clôturés) ===\n",
        candidat.ticker,
        historique.len()
    ));

    let gains = historique
        .iter()
        .filter(|s| {
            s.verdict
                .as_deref()
                .map(|v| matches!(v, "TP1" | "TP2" | "TP3" | "confirme"))
                .unwrap_or(false)
        })
        .count();
    let winrate = gains * 100 / historique.len();
    let rs: Vec<f64> = historique
        .iter()
        .filter_map(|s| {
            let pv = s.prix_verdict?;
            let risk = s.prix_entree - s.stop_loss;
            if risk <= 0.0 { return None; }
            Some((pv - s.prix_entree) / risk)
        })
        .collect();
    let r_moyen = if rs.is_empty() { 0.0 } else { rs.iter().sum::<f64>() / rs.len() as f64 };

    ctx.push_str(&format!("Winrate: {}% | R moyen: {:.2}R\n", winrate, r_moyen));
    ctx.push_str("Derniers résultats :\n");
    for s in historique.iter().take(5) {
        let verdict = s.verdict.as_deref().unwrap_or("?");
        let r_str = s
            .prix_verdict
            .and_then(|pv| {
                let risk = s.prix_entree - s.stop_loss;
                if risk > 0.0 { Some(format!("{:.2}R", (pv - s.prix_entree) / risk)) } else { None }
            })
            .unwrap_or_else(|| "?R".to_string());
        ctx.push_str(&format!(
            "  • {} | phase={} score={} RSI={:.0} vol={:.1}× → {} ({})\n",
            s.ticker, s.phase, s.score, s.rsi, s.ratio_volume, verdict, r_str
        ));
    }
    ctx
}

/// Construit le bloc "leçons passées" few-shot injecté dans le prompt.
pub fn construire_few_shot(feedbacks: &[RocketsFeedbackRow]) -> String {
    if feedbacks.is_empty() {
        return String::new();
    }
    let mut bloc = String::from("=== LEÇONS PASSÉES (signaux similaires clôturés) ===\n");
    for fb in feedbacks {
        let resultat = if fb.gagnant == Some(1) { "✅ GAGNANT" } else { "❌ PERDANT" };
        let pnl = fb.pnl_r.map(|r| format!("{:.2}R", r)).unwrap_or_default();
        let duree = fb.duree_trade_min.map(|d| format!("{}min", d)).unwrap_or_else(|| "-".into());
        let session_out = fb.session_sortie.as_deref().unwrap_or("-");
        let notes = fb.notes_trader.as_deref().map(|n| format!(" | note: {}", n)).unwrap_or_default();
        bloc.push_str(&format!(
            "  \u{2022} {} ticker={} | score={} conviction={} RSI={:.0} vol={:.1}\u{00d7} atr_ratio={:.2} | durée={} session_sortie={} \u{2192} {} {}{}\n",
            fb.verdict.as_deref().unwrap_or("?"),
            fb.ticker,
            fb.score_scan,
            fb.conviction_llm,
            fb.rsi,
            fb.ratio_volume,
            fb.atr_ratio,
            duree,
            session_out,
            resultat,
            pnl,
            notes,
        ));
    }
    bloc
}
