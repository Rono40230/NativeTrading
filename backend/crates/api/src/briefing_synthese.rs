//! Synthèse du briefing (refonte owner 23/10) : structures de réponse +
//! parseur des SIX sections titrées produites par le prompt
//! `briefing_session`. Extrait de briefing_ia.rs (limite 600 lignes).

use serde::Serialize;

#[derive(Debug, Clone, Serialize, Default)]
pub struct SectionBriefing {
    pub titre: String,
    pub texte: String,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct SyntheseBriefing {
    pub resume: String,
    pub ce_qui_compte: Vec<String>,
    pub confiance: u32,
    /// Refonte owner 23/10 : les six sections titrées du briefing (le
    /// resume/ce_qui_compte restent remplis pour le digest mobile 8h30).
    #[serde(default)]
    pub sections: Vec<SectionBriefing>,
}



/// Extraction des SIX sections titrées (refonte owner 23/10). Le titre se
/// présente en gras markdown (**LA NUIT EN BREF**) ou en majuscules nues ;
/// les lignes « 1. » de LA PRIORITÉ nourrissent ce_qui_compte (digest
/// mobile 8h30) et le corps complet devient resume. Repli : si le modèle
/// répond en JSON (ancien format), l'ancien parseur s'applique ; en dernier
/// recours le texte brut — jamais de synthèse inventée.
pub(crate) fn parser_synthese(texte: &str) -> SyntheseBriefing {
    let n = texte.trim();
    let (d, f) = (n.find('{'), n.rfind('}'));
    if let (Some(d), Some(f)) = (d, f) {
        if f > d {
            #[derive(serde::Deserialize)]
            struct Rep {
                #[serde(default)] resume: String,
                #[serde(default)] ce_qui_compte: Vec<String>,
                #[serde(default)] confiance: f64,
            }
            if let Ok(v) = serde_json::from_str::<Rep>(&n[d..=f]) {
                return SyntheseBriefing {
                    resume: v.resume,
                    ce_qui_compte: v.ce_qui_compte,
                    confiance: crate::analyses_ia::confiance_normalisee(v.confiance),
                    sections: vec![],
                };
            }
        }
    }

    // Découpe par titres de section (**TITRE** ou TITRE en majuscules seul).
    let mut sections: Vec<SectionBriefing> = Vec::new();
    for ligne in n.lines() {
        let l = ligne.trim();
        let titre = l
            .strip_prefix("**").and_then(|x| x.strip_suffix("**")).map(|x| x.trim())
            .filter(|x| x.len() > 3 && x.chars().all(|c| c.is_uppercase() || c.is_whitespace() || c == '&'))
            .map(|x| x.to_string())
            .or_else(|| {
                (l.len() > 3 && !l.contains(':') && l.chars().all(|c| c.is_uppercase() || c.is_whitespace() || c == '&'))
                    .then(|| l.to_string())
            });
        match titre {
            Some(t) => sections.push(SectionBriefing { titre: t, texte: String::new() }),
            None => {
                if let Some(derniere) = sections.last_mut() {
                    if !l.is_empty() {
                        if !derniere.texte.is_empty() {
                            derniere.texte.push('\n');
                        }
                        derniere.texte.push_str(l);
                    }
                }
            }
        }
    }

    if sections.is_empty() {
        return SyntheseBriefing { resume: n.chars().take(600).collect(), ce_qui_compte: vec![], confiance: 0, sections: vec![] };
    }
    // resume = corps de LA PRIORITÉ (dernier recours : dernière section) ;
    // ce_qui_compte = ses lignes « 1. 2. 3. ».
    let priorites = sections
        .iter()
        .rev()
        .find(|s| s.titre.contains("PRIORIT"))
        .map(|s| s.texte.clone())
        .unwrap_or_else(|| sections.last().map(|s| s.texte.clone()).unwrap_or_default());
    let ce_qui_compte: Vec<String> = priorites
        .lines()
        .filter(|l| {
            let s = l.trim();
            (s.starts_with('1') || s.starts_with('2') || s.starts_with('3')) && (s.contains('.') || s.contains(')'))
        })
        .map(|l| l.trim().to_string())
        .collect();
    SyntheseBriefing {
        resume: priorites.chars().take(600).collect(),
        ce_qui_compte,
        confiance: 0,
        sections,
    }
}

// ── Relais mobile : rapport d'activité + signaux en cours (owner 21/10) ─────
