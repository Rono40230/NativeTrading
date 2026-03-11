use common::{Candle, Direction};

/// Structure de marché SMC : détection Haut/Bas significatifs
#[derive(Debug, Clone)]
pub struct NiveauPrix {
    pub index: usize,
    pub prix: f64,
    pub est_haut: bool, // true = Haut local, false = Bas local
}

/// Résultat d'analyse de tendance
#[derive(Debug, Clone)]
pub struct AnalyseTendance {
    pub direction: Direction,
    /// Niveaux pivots : HH, HL, LH, LL
    pub niveaux: Vec<NiveauPrix>,
    /// Force de la tendance (0.0 à 1.0)
    pub force: f64,
}

/// Détecte les hauts/bas locaux sur une fenêtre de `n_voisins` bougies de chaque côté
pub fn detecter_pivots(bougies: &[Candle], n_voisins: usize) -> Vec<NiveauPrix> {
    if bougies.len() < n_voisins * 2 + 1 {
        return vec![];
    }
    let mut niveaux = vec![];
    let debut = n_voisins;
    let fin = bougies.len() - n_voisins;

    for i in debut..fin {
        let haut_i = bougies[i].high;
        let bas_i = bougies[i].low;

        let est_haut_local = (i - n_voisins..i)
            .chain(i + 1..=i + n_voisins)
            .all(|j| bougies[j].high <= haut_i);

        let est_bas_local = (i - n_voisins..i)
            .chain(i + 1..=i + n_voisins)
            .all(|j| bougies[j].low >= bas_i);

        if est_haut_local {
            niveaux.push(NiveauPrix {
                index: i,
                prix: haut_i,
                est_haut: true,
            });
        }
        if est_bas_local {
            niveaux.push(NiveauPrix {
                index: i,
                prix: bas_i,
                est_haut: false,
            });
        }
    }
    niveaux
}

/// Analyse la tendance à partir des pivots (structure HH/HL = haussier, LH/LL = baissier)
pub fn analyser_tendance(bougies: &[Candle], n_voisins: usize) -> AnalyseTendance {
    let niveaux = detecter_pivots(bougies, n_voisins);
    if niveaux.len() < 4 {
        return AnalyseTendance {
            direction: Direction::Both,
            niveaux,
            force: 0.0,
        };
    }

    let hauts: Vec<f64> = niveaux
        .iter()
        .filter(|n| n.est_haut)
        .map(|n| n.prix)
        .collect();
    let bas: Vec<f64> = niveaux
        .iter()
        .filter(|n| !n.est_haut)
        .map(|n| n.prix)
        .collect();

    let hh_count = hauts.windows(2).filter(|w| w[1] > w[0]).count();
    let hl_count = bas.windows(2).filter(|w| w[1] > w[0]).count();
    let lh_count = hauts.windows(2).filter(|w| w[1] < w[0]).count();
    let ll_count = bas.windows(2).filter(|w| w[1] < w[0]).count();

    let score_haussier = hh_count + hl_count;
    let score_baissier = lh_count + ll_count;
    let total = score_haussier + score_baissier;

    let (direction, force) = if total == 0 {
        (Direction::Both, 0.0)
    } else if score_haussier > score_baissier {
        (Direction::Long, score_haussier as f64 / total as f64)
    } else if score_baissier > score_haussier {
        (Direction::Short, score_baissier as f64 / total as f64)
    } else {
        (Direction::Both, 0.5)
    };

    AnalyseTendance {
        direction,
        niveaux,
        force,
    }
}
