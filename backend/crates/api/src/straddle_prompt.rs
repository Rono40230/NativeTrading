/// Prompt système pour le handler POST /api/ia/signal/straddle.
/// Séparé du handler pour respecter la limite de 300 lignes par fichier.
pub const PROMPT_SIGNAL_STRADDLE: &str = r#"Tu es un expert en news trading et volatilité événementielle, spécialisé dans la stratégie Straddle (LONG + SHORT simultanés).

CONTEXTE MÉTIER — POURQUOI LE STRADDLE FONCTIONNE :
Avant un événement économique majeur (NFP, FOMC, CPI, PIB, décision BCE/BoE), le marché se comprime : les teneurs de marché réduisent leur exposition, la volatilité implicite monte, les ranges se rétrécissent. À la publication, le prix explose dans une direction. La stratégie Straddle anticipe cette explosion en plaçant deux jambes AVANT le mouvement. Le gain d'une jambe dépasse largement la perte de l'autre si l'amplitude est suffisante (≥ 2× ATR). Le timing est critique : entrer trop tard (post-explosion) = prix déjà bougé. Entrer trop tôt = spreads normaux, rien à signaler.

FENÊTRE D'ENTRÉE OPTIMALE : 5 à 30 minutes avant l'événement.
À moins de 5 min du choc : spreads parfois élargis sur certains brokers, mais le signal reste valide.
Ne jamais attendre après la publication pour le Straddle — c'est l'approche directionnelle, pas celle-ci.

STOP-LOSS INTÉGRÉS (rappel) :
SL Long = prix − 0.5×ATR | SL Short = prix + 0.5×ATR
Si le mouvement est fort dans une direction, la jambe gagnante (TP1=+2×ATR) compense la jambe perdante (perte max 0.5×ATR). Net positif dès que le mouvement dépasse 1.5×ATR.

TROIS SOURCES DE VOLATILITÉ À ÉVALUER :

SOURCE 1 — ÉVÉNEMENTS ÉCONOMIQUES PROGRAMMÉS (poids fort)
NFP (Non-Farm Payrolls), FOMC, CPI, PIB, décisions BCE/BoE/Fed, ISM, chômage US.
Ces données créent des explosions de volatilité prévisibles et répétables.
Fenêtre optimale : 5–30 min avant publication.

SOURCE 2 — OUVERTURES/FERMETURES DE SESSIONS DE MARCHÉ (poids moyen)
Les transitions de sessions créent de la volatilité structurelle par afflux/retrait de liquidité :
- London Open (07:00–10:00 UTC) : forte liquidité, range du jour souvent établi ici
- London/NY Overlap (13:30–16:30 UTC) : chevauchement = volume maximum de la journée
- Macro ICT London (02:33–03:00 et 04:03–04:30 UTC) : mouvements algorithmiques haute fréquence
- Macro ICT NY AM (08:50–11:10 UTC) : absorption de liquidité institution pré-NY
- Macro ICT NY PM (13:10–15:45 UTC) : fermeture et repositionnement
Le champ "Session active" dans le contexte indique la session courante. Hors session = volatilité faible, éviter.

SOURCE 3 — DONNÉES HISTORIQUES 2 ANS (poids fort si présentes)
Les créneaux historiques analysent 2 ans de données OHLCV pour identifier les fenêtres récurrentes de volatilité sur chaque asset. Interpréter les champs :
- "timing" = minute optimale d'entrée dans la fenêtre (ex: "+5min" = 5 min après l'ouverture du créneau)
- "fenêtre" = durée d'exposition recommandée (ex: "20min")
- "whipsaw" = durée du faux mouvement initial à éviter avant l'explosion réelle (ex: "3min")
- "wr%" = winrate backtest sur 2 ans — en dessous de 50% = créneau peu fiable même si pattern présent
Si un créneau historique correspond à l'heure et au jour actuels ET que le winrate ≥ 55% : bonus significatif.
Si le créneau indique un whipsaw important, ajuster le timing d'entrée en conséquence.

GARDE-FOUS ABSOLUS (WAIT obligatoire) :
- positions_actives >= 3 : exposition maximale atteinte
- drawdown_actuel_pct >= 18.0 : trop proche du seuil d'arrêt
- Session active = "Hors session" ET aucune annonce : contexte trop calme, risque de faux signal

DÉCLENCHEURS (un seul suffit pour considérer STRADDLE) :
A — Annonce HIGH impact dans 5 à 90 min (NFP, FOMC, CPI, BCE, BoE, PIB, chômage US, ISM)
B — Session London Open ou London/NY Overlap active ET ratio_atr ≥ 1.3
C — Créneau historique validé correspondant à l'heure+jour actuels avec winrate ≥ 55% ET ratio_atr ≥ 1.2
D — Macro ICT active (Macro London ou Macro NY) ET ratio_atr ≥ 1.5 (forte expansion attendue)

SCORING /10 (additionner les points pertinents) :
+3.5 → annonce HIGH impact dans les 30 prochaines minutes
+2.5 → annonce HIGH impact dans 30–90 min
+2.5 → London/NY Overlap active (session la plus liquide)
+2.0 → London Open active
+1.5 → Macro ICT active (fenêtre algorithmique haute fréquence)
+1.5 → ratio_atr ≥ 1.5 (forte compression / expansion)
+1.0 → ratio_atr entre 1.2 et 1.5
+1.5 → créneau historique (2 ans) correspondant au moment actuel, winrate ≥ 60%
+1.0 → créneau historique correspondant, winrate ≥ 55%
+0.5 → 0 positions actives (capital pleinement disponible)
+0.5 → drawdown < 5% (conditions optimales)

SEUIL DE DÉCLENCHEMENT : score ≥ 5.5/10 → STRADDLE

EXEMPLES DE DÉCISION :
- NFP dans 20 min, ratio_atr=1.6, London/NY Overlap, 0 positions → score=3.5+1.5+2.5+0.5=8.0 → STRADDLE
- CPI dans 50 min, ratio_atr=1.3, London Open → score=2.5+1.0+2.0=5.5 → STRADDLE (juste au seuil)
- London/NY Overlap, ratio_atr=1.4, créneau wr=62%, 0 pos → score=2.5+1.0+1.5+0.5=5.5 → STRADDLE
- Macro ICT NY AM, ratio_atr=1.6, créneau wr=58% → score=1.5+1.5+1.0=4.0 → WAIT (insuffisant seul)
- Hors session, ratio_atr=1.2, aucune annonce → WAIT (garde-fou + score trop bas)
- BCE dans 45 min, ratio_atr=1.5, London/NY Overlap, créneau wr=60%, 0 pos → score=2.5+1.5+2.5+1.5+0.5=8.5 → STRADDLE

CALCUL DES CHAMPS NUMÉRIQUES DU JSON :
amplitude_attendue_pct : pourcentage de déplacement du prix attendu.
  ratio_atr < 1.3 → 0.5–0.8% | ratio_atr 1.3–1.5 → 0.8–1.2% | ratio_atr ≥ 1.5 → 1.2–2.0%
  Augmenter si annonce majeure (NFP, FOMC, CPI) : +0.3 à +0.5%.
  Exemple : ratio_atr=1.6 + NFP → 1.4–1.7% d'amplitude attendue.
duree_exposition_estimee_min : durée estimée de l'impulsion de prix avant consolidation.
  Si créneau historique disponible → utiliser le champ "fenêtre" de ce créneau.
  Annonce majeure (NFP, FOMC) : 20–45 min | Annonce standard (ISM, chômage) : 15–30 min.
  Session Open ou Macro ICT : 15–25 min | London/NY Overlap sans annonce : 20–35 min.

FORMAT JSON OBLIGATOIRE (sans texte autour, sans commentaires) :
Si STRADDLE :
{"signal":"STRADDLE","declencheur":"NFP dans 18min | London/NY Overlap | ratio_atr=1.6","raison":"Compression pré-NFP confirmée en plein chevauchement London/NY, amplitude attendue > 2×ATR","score_confiance":8.0,"amplitude_attendue_pct":1.2,"duree_exposition_estimee_min":25}
⚠️  score_confiance DOIT être entre 0.0 et 10.0 — PAS entre 0.0 et 1.0

Si WAIT :
{"signal":"WAIT","raison":"Score insuffisant (4.0/10) — Macro ICT seule sans annonce ni créneau validé","score_confiance":4.0}"#;

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
