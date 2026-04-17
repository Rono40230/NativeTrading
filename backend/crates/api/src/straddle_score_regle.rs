//! Fallback P4 : score de confiance basé sur des règles métier.
//!
//! Utilisé quand le modèle ML Straddle (xgb_straddle) n'est pas encore entraîné.
//! Remplace la gate ML et filtre les contextes faibles avant l'appel Ollama.
//!
//! ## Barème (max 100 pts, seuil 60)
//! - +30 si catégorie = annonce_high ou overlap_lnd_ny
//! - +20 si un créneau historique validé correspond à l'heure actuelle
//! - +20 si kill zone SMC active
//! - +15 si ratio_atr > 2.0
//! - +15 si session London ou New York

use chrono::{DateTime, Timelike, Utc};
use db::straddle::StraddleCreneau;

/// Seuil minimal pour autoriser l'appel Ollama.
pub const SEUIL_SCORE_REGLE: u32 = 60;

/// Contexte nécessaire au calcul du score.
pub struct ContexteScoreRegle<'a> {
    pub categorie: &'a str,
    pub ratio_atr: f64,
    pub now: DateTime<Utc>,
    pub creneaux_valides: &'a [StraddleCreneau],
}

/// Calcule le score de confiance par règles métier (0–100).
pub fn calculer_score(ctx: &ContexteScoreRegle<'_>) -> u32 {
    let mut score: u32 = 0;

    // +30 pts : catégorie à fort potentiel
    if matches!(ctx.categorie, "annonce_high" | "overlap_lnd_ny") {
        score += 30;
    }

    // +20 pts : créneau historique validé actif à cette heure
    let hm = ctx.now.hour() * 60 + ctx.now.minute();
    let creneau_actif = ctx.creneaux_valides.iter().any(|c| {
        heure_dans_creneau(hm, &c.heure_debut, &c.heure_fin)
    });
    if creneau_actif {
        score += 20;
    }

    // +20 pts : kill zone SMC active
    if smc::kill_zone::est_en_kill_zone(ctx.now) {
        score += 20;
    }

    // +15 pts : ATR très élevé (pic fort)
    if ctx.ratio_atr > 2.0 {
        score += 15;
    }

    // +15 pts : session London ou New York (via nom_kill_zone)
    if let Some(kz) = smc::kill_zone::nom_kill_zone(ctx.now) {
        if kz == "London" || kz == "New York" {
            score += 15;
        }
    }

    score
}

/// Formate une ligne de contexte résumant le score pour le prompt Ollama.
pub fn texte_contexte(score: u32) -> String {
    format!(
        "Score confiance règles-métier (sans ML) : {}/100 — autorisé (seuil {})\n",
        score, SEUIL_SCORE_REGLE
    )
}

fn heure_dans_creneau(hm: u32, debut: &str, fin: &str) -> bool {
    fn parse_hm(s: &str) -> u32 {
        let mut it = s.splitn(2, ':');
        let h: u32 = it.next().and_then(|v| v.parse().ok()).unwrap_or(0);
        let m: u32 = it.next().and_then(|v| v.parse().ok()).unwrap_or(0);
        h * 60 + m
    }
    let d = parse_hm(debut);
    let f = parse_hm(fin);
    if d <= f { hm >= d && hm <= f } else { hm >= d || hm <= f }
}
