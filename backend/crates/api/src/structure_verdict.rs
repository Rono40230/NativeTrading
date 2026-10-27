//! STRUCT P1 (owner 27/10) — verdict d'alignement MTF, échelle ACTIONNABLE
//! (owner 27/10 soir) : direction (biais W1→D1→H4) → zone (premium/discount
//! H1) → confirmation (déclencheur LTF frais). Une INTENTION jamais un ordre.
//! Pur et testé — vit dans son module pour le garde-fou 600 lignes.

use super::{EvenementStructure, LigneStructure};

#[derive(Debug, Clone, serde::Serialize, PartialEq)]
pub struct VerdictStructure {
    /// Sens de référence = état du TF le plus haut disponible.
    pub biais: String,
    /// H4 face au biais : "aligne" | "repli" | "transition" | "conflit".
    pub contexte: String,
    /// Dernier événement M15→H1, condensé ("MSS bear M15") — None si rien.
    pub declencheur: Option<String>,
    /// Lecture factuelle en sous-ligne (biais · zone · déclencheur).
    pub texte: String,
    /// Décision lisible, échelle fermée owner 27/10 — une INTENTION jamais
    /// un ordre : « Setups d'ACHAT actifs · cible BSL … », « ACHAT :
    /// attendre le repli en discount », « Attendre — structure en
    /// transition », « Pas de biais — rester à l'écart »…
    pub action: String,
    /// Clé de couleur du pied de table : actif_haussier | actif_baissier |
    /// attente_achat | attente_vente | transition | neutre.
    pub niveau: String,
}

/// Lecture d'alignement ICT : biais = TF le plus haut ; contexte = H4 face
/// au biais (« repli » = H4 contre le biais SANS MSS H4 contre le biais —
/// simple correction ; « transition » = MSS H4 contre le biais, transfert
/// de contrôle en cours ; « conflit » = H4 opposé installé) ; déclencheur =
/// dernier événement LTF (M15→H1) dans ou contre le sens du biais.
pub fn verdict(lignes: &[LigneStructure]) -> VerdictStructure {
    let etat = |tf: &str| {
        lignes
            .iter()
            .find(|l| l.tf == tf)
            .map(|l| l.etat.as_str())
            .unwrap_or("neutre")
    };
    let dernier_evt = |tf: &str| lignes.iter().find(|l| l.tf == tf).and_then(|l| l.dernier_evenement.clone());

    let biais = if etat("W1") != "neutre" {
        etat("W1")
    } else if etat("D1") != "neutre" {
        etat("D1")
    } else {
        etat("H4")
    }
    .to_string();

    let mss_h4 = dernier_evt("H4").filter(|e| e.genre == "MSS");
    let contexte = if etat("H4") == biais || biais == "neutre" {
        "aligne"
    } else if mss_h4
        .as_ref()
        .map(|e| contre(e, &biais))
        .unwrap_or(false)
    {
        "transition"
    } else {
        "repli"
    }
    .to_string();

    // Dernier événement LTF par horodatage (M15→H1 confondus : le plus récent).
    let declencheur_evt = ["M15", "M30", "H1"]
        .iter()
        .filter_map(|tf| dernier_evt(tf))
        .max_by_key(|e| e.ts);
    let declencheur = declencheur_evt
        .as_ref()
        .map(|e| format!("{} {} {}", e.genre, dir_fr(&e.dir), tf_depuis_ts(e.ts, lignes)));

    // Confirmation = déclencheur LTF dans le sens du biais ET récent (moins
    // de 5 barres de son TF — un MSS M30 d'il y a deux jours ne confirme rien).
    let confirmation = declencheur_evt
        .as_ref()
        .map(|e| dans_sens(e, &biais) && est_frais(e, lignes))
        .unwrap_or(false);
    let (action, niveau) = action_verdict(&biais, &contexte, confirmation, lignes);
    let texte = texte_verdict(&biais, &contexte, declencheur_evt.as_ref(), lignes);
    VerdictStructure { biais, contexte, declencheur, texte, action, niveau }
}

fn contre(e: &EvenementStructure, biais: &str) -> bool {
    if biais == "haussiere" {
        e.dir == "bear"
    } else if biais == "baissiere" {
        e.dir == "bull"
    } else {
        false
    }
}

fn dans_sens(e: &EvenementStructure, biais: &str) -> bool {
    if biais == "haussiere" {
        e.dir == "bull"
    } else {
        e.dir == "bear"
    }
}

fn duree_tf_s(tf: &str) -> i64 {
    match tf {
        "M15" => 900,
        "M30" => 1800,
        _ => 3600, // H1 ; « LTF » (TF d'origine introuvable) prudent
    }
}

/// Fraîcheur du déclencheur : moins de 5 barres du TF d'origine se sont
/// écoulées depuis l'événement. Ancrage absent (tests sans rejeu) = frais.
fn est_frais(e: &EvenementStructure, lignes: &[LigneStructure]) -> bool {
    let duree = duree_tf_s(&tf_depuis_ts(e.ts, lignes));
    let ancre = lignes
        .iter()
        .find(|l| l.dernier_evenement.as_ref().map(|x| x.ts) == Some(e.ts))
        .map(|l| l.ts_derniere)
        .unwrap_or(0);
    e.ts >= ancre - 5 * duree
}

/// Entonnoir ICT → action fermée (owner 27/10) : direction (biais) → zone
/// (premium/discount H1) → confirmation (déclencheur LTF frais dans le
/// sens). Une INTENTION, jamais un ordre — on dit où CHERCHER :
/// achat en premium = cher, on attend le repli quelle que soit l'agitation
/// LTF ; équilibre = ni cher ni donné, la confirmation suffit.
fn action_verdict(
    biais: &str,
    contexte: &str,
    confirmation: bool,
    lignes: &[LigneStructure],
) -> (String, String) {
    if biais == "neutre" {
        return ("Pas de biais — rester à l'écart".into(), "neutre".into());
    }
    if contexte == "transition" || contexte == "conflit" {
        return ("Attendre — structure en transition".into(), "transition".into());
    }
    let haussier = biais == "haussiere";
    let h1 = lignes.iter().find(|l| l.tf == "H1");
    let premium = h1.map(|l| l.in_premium).unwrap_or(false);
    let discount = h1.map(|l| l.in_discount).unwrap_or(false);
    if (haussier && premium) || (!haussier && discount) {
        return if haussier {
            ("ACHAT : attendre le repli en discount".into(), "attente_achat".into())
        } else {
            ("VENTE : attendre le rallye en premium".into(), "attente_vente".into())
        };
    }
    if confirmation {
        let cible = h1.and_then(|l| if haussier { l.bsl } else { l.ssl });
        let suffixe = cible
            .map(|p| format!(" · cible {} {:.2}", if haussier { "BSL" } else { "SSL" }, p))
            .unwrap_or_default();
        return if haussier {
            (format!("Setups d'ACHAT actifs{suffixe}"), "actif_haussier".into())
        } else {
            (format!("Setups de VENTE actifs{suffixe}"), "actif_baissier".into())
        };
    }
    if haussier {
        ("ACHAT : attendre confirmation LTF".into(), "attente_achat".into())
    } else {
        ("VENTE : attendre confirmation LTF".into(), "attente_vente".into())
    }
}

fn dir_fr(dir: &str) -> &'static str {
    if dir == "bull" { "haussier" } else { "baissier" }
}

/// TF d'origine d'un événement par élimination (le déclencheur vient de
/// M15/M30/H1 — on nomme celui dont l'état/événement correspond au ts).
fn tf_depuis_ts(ts: i64, lignes: &[LigneStructure]) -> String {
    for l in lignes {
        if let Some(e) = &l.dernier_evenement {
            if e.ts == ts {
                return l.tf.clone();
            }
        }
    }
    "LTF".into()
}

fn texte_verdict(
    biais: &str,
    contexte: &str,
    declencheur: Option<&EvenementStructure>,
    lignes: &[LigneStructure],
) -> String {
    let biais_fr = if biais == "haussiere" { "haussier" } else if biais == "baissiere" { "baissier" } else { "neutre" };
    let ctx = match contexte {
        "aligne" => format!("Biais {biais_fr} confirmé H4"),
        "transition" => format!("Biais {biais_fr} · MSS H4 contre — transfert de contrôle en cours"),
        "conflit" => format!("Biais {biais_fr} · H4 opposé installé — conflit"),
        _ => format!("Biais {biais_fr} · H4 en repli"),
    };
    let trig = match declencheur {
        Some(e) => format!(" · dernier déclencheur {} {}", e.genre, dir_fr(&e.dir)),
        None => String::new(),
    };
    // Discount = zone d'achat ICT sous l'équilibre (biais haussier) ; premium
    // = zone de vente au-dessus. La cible BSL/SSL vit dans `action` (visible
    // seulement quand les setups sont actifs — pas de promesse en attente).
    let h1 = lignes.iter().find(|l| l.tf == "H1");
    let zone = h1
        .map(|l| {
            if l.in_discount {
                Some(" · H1 en discount".to_string())
            } else if l.in_premium {
                Some(" · H1 en premium".to_string())
            } else {
                None
            }
        })
        .flatten()
        .unwrap_or_default();
    format!("{ctx}{zone}{trig}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ligne(tf: &str, etat: &str, evt: Option<EvenementStructure>, prem: bool, disc: bool, bsl: Option<f64>, ssl: Option<f64>) -> LigneStructure {
        LigneStructure {
            tf: tf.into(),
            etat: etat.into(),
            dernier_evenement: evt,
            dernier_pivot: None,
            in_premium: prem,
            in_discount: disc,
            bsl,
            ssl,
            ts_derniere: 0,
        }
    }

    fn evt(genre: &str, dir: &str, ts: i64) -> EvenementStructure {
        EvenementStructure { genre: genre.into(), dir: dir.into(), level: 100.0, ts }
    }

    /// Biais haussier W1, H4 aligné, H1 en discount, déclencheur BOS
    /// haussier frais : setups d'ACHAT ACTIFS avec cible BSL.
    #[test]
    fn verdict_aligne_avec_cible() {
        let lignes = vec![
            ligne("M15", "baissiere", Some(evt("MSS", "bear", 100)), false, false, None, None),
            ligne("M30", "baissiere", None, false, false, None, None),
            ligne("H1", "haussiere", Some(evt("BOS", "bull", 300)), false, true, Some(4145.0), Some(3988.0)),
            ligne("H4", "haussiere", Some(evt("BOS", "bull", 50)), false, false, None, None),
            ligne("D1", "haussiere", None, false, false, None, None),
            ligne("W1", "haussiere", None, false, false, None, None),
        ];
        let v = verdict(&lignes);
        assert_eq!(v.biais, "haussiere");
        assert_eq!(v.contexte, "aligne");
        assert!(v.declencheur.as_deref().unwrap_or("").contains("BOS"));
        assert_eq!(v.niveau, "actif_haussier");
        assert!(v.action.contains("Setups d'ACHAT actifs"));
        assert!(v.action.contains("cible BSL 4145"));
        assert!(v.texte.contains("discount"));
        assert!(!v.texte.contains("cible"), "la cible vit dans action, pas en sous-ligne");
    }

    /// H4 contre le biais SANS MSS H4 = repli ; MSS H4 contre = transition.
    /// Repli sans déclencheur = attendre confirmation ; transition = attendre.
    #[test]
    fn verdict_repli_vs_transition() {
        let repli = vec![
            ligne("H1", "baissiere", None, false, false, None, None),
            ligne("H4", "baissiere", Some(evt("BOS", "bear", 10)), false, false, None, None),
            ligne("D1", "haussiere", None, false, false, None, None),
            ligne("W1", "haussiere", None, false, false, None, None),
            ligne("M15", "baissiere", None, false, false, None, None),
            ligne("M30", "baissiere", None, false, false, None, None),
        ];
        let v = verdict(&repli);
        assert_eq!(v.contexte, "repli");
        assert_eq!(v.niveau, "attente_achat");
        assert!(v.action.contains("attendre confirmation LTF"));

        let mut transition = repli.clone();
        if let Some(l) = transition.iter_mut().find(|l| l.tf == "H4") {
            l.dernier_evenement = Some(evt("MSS", "bear", 10));
        }
        let v = verdict(&transition);
        assert_eq!(v.contexte, "transition");
        assert_eq!(v.niveau, "transition");
        assert!(v.action.contains("Attendre — structure en transition"));
    }

    /// La zone prime sur la confirmation : biais haussier + H1 en PREMIUM =
    /// attendre le repli, même avec un déclencheur haussier frais.
    #[test]
    fn verdict_zone_defavorable() {
        let lignes = vec![
            ligne("M15", "haussiere", None, false, false, None, None),
            ligne("M30", "haussiere", None, false, false, None, None),
            ligne("H1", "haussiere", Some(evt("BOS", "bull", 300)), true, false, Some(4145.0), None),
            ligne("H4", "haussiere", None, false, false, None, None),
            ligne("D1", "haussiere", None, false, false, None, None),
            ligne("W1", "haussiere", None, false, false, None, None),
        ];
        let v = verdict(&lignes);
        assert_eq!(v.niveau, "attente_achat");
        assert!(v.action.contains("attendre le repli en discount"));
    }

    /// Déclencheur haussier mais PÉRIMÉ (6 bougies H1 plus tard) : pas de
    /// confirmation — on attend, cible non promises.
    #[test]
    fn verdict_confirmation_perimee() {
        let mut lignes = vec![
            ligne("M15", "baissiere", None, false, false, None, None),
            ligne("M30", "baissiere", None, false, false, None, None),
            ligne("H1", "haussiere", Some(evt("BOS", "bull", 300)), false, true, Some(4145.0), None),
            ligne("H4", "haussiere", None, false, false, None, None),
            ligne("D1", "haussiere", None, false, false, None, None),
            ligne("W1", "haussiere", None, false, false, None, None),
        ];
        if let Some(l) = lignes.iter_mut().find(|l| l.tf == "H1") {
            l.ts_derniere = 300 + 6 * 3600; // 6 bougies H1 plus loin
        }
        let v = verdict(&lignes);
        assert_eq!(v.niveau, "attente_achat");
        assert!(v.action.contains("attendre confirmation LTF"));
    }

    /// W1 neutre → le biais tombe sur D1 ; tout neutre → biais neutre sans
    /// panique.
    #[test]
    fn biais_par_defaut() {
        let lignes = vec![
            ligne("H1", "haussiere", None, false, false, None, None),
            ligne("H4", "haussiere", None, false, false, None, None),
            ligne("D1", "haussiere", None, false, false, None, None),
            ligne("W1", "neutre", None, false, false, None, None),
            ligne("M15", "neutre", None, false, false, None, None),
            ligne("M30", "neutre", None, false, false, None, None),
        ];
        assert_eq!(verdict(&lignes).biais, "haussiere");
        let tout_neutre: Vec<LigneStructure> = ["M15", "M30", "H1", "H4", "D1", "W1"]
            .iter()
            .map(|tf| ligne(tf, "neutre", None, false, false, None, None))
            .collect();
        let v = verdict(&tout_neutre);
        assert_eq!(v.biais, "neutre");
        assert_eq!(v.niveau, "neutre");
        assert!(v.action.contains("Pas de biais"));
    }
}
