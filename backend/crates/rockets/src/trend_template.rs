//! Trend Template Minervini — pré-screen d'entrée dans l'univers de scan
//! actions (étape A2, 31/08). Les 8 conditions canoniques (Minervini,
//! « Trade Like a Stock Market Wizard ») :
//!
//! 1. prix > MM150 et prix > MM200
//! 2. MM150 > MM200
//! 3. MM200 en hausse depuis ≥ 1 mois
//! 4. MM50 > MM150 et MM50 > MM200
//! 5. prix > MM50
//! 6. prix ≥ 30 % au-dessus du plus bas 52 semaines
//! 7. prix à ≤ 25 % du plus haut 52 semaines
//! 8. force relative : surperformance du marché de référence sur 4 semaines
//!
//! Ce module est PUR (testé sans réseau) : il reçoit les clôtures D1 d'une
//! action et la perf 4 semaines du marché de référence (QQQ via Tiingo,
//! étape B). Le classement /10 du scanner reste inchangé — ce template
//! décide seulement qui ENTRE dans le périmètre scanné chaque semaine.

/// Résultat du trend template d'un ticker.
#[derive(Debug, Clone, PartialEq)]
pub struct TrendTemplate {
    pub reussi: bool,
    /// Conditions passées (sur 8).
    pub conditions: u8,
    pub prix: f64,
    pub mm50: Option<f64>,
    pub mm150: Option<f64>,
    pub mm200: Option<f64>,
    pub haut_52s: f64,
    pub bas_52s: f64,
    pub perf_4s: f64,
}

fn mma(clotures: &[f64], periode: usize) -> Option<f64> {
    if clotures.len() < periode || periode == 0 {
        return None;
    }
    Some(clotures[clotures.len() - periode..].iter().sum::<f64>() / periode as f64)
}

/// Évalue le trend template. `clotures` = D1 en ordre chronologique ASC
/// (≥ 260 jours requis pour MM200+1 mois et 52 semaines) ;
/// `perf_marche_4s` = performance 4 semaines du marché de référence.
pub fn trend_template(clotures: &[f64], perf_marche_4s: f64) -> Option<TrendTemplate> {
    // 220 : MM200 courante + MM200 d'il y a un mois ; 261 : fenêtre 52 s.
    if clotures.len() < 261 {
        return None;
    }

    let prix = *clotures.last()?;
    let mm50 = mma(clotures, 50);
    let mm150 = mma(clotures, 150);
    let mm200 = mma(clotures, 200);
    // MM200 d'il y a ~21 séances (1 mois) : fenêtre décalée de la fin.
    let mm200_mois = mma(&clotures[..clotures.len() - 21], 200);

    let fenetre_52s = &clotures[clotures.len() - 261..];
    let haut_52s = fenetre_52s.iter().cloned().fold(f64::MIN, f64::max);
    let bas_52s = fenetre_52s.iter().cloned().fold(f64::MAX, f64::min);

    // Perf 4 semaines (21 séances) de l'action.
    let vieux = clotures[clotures.len() - 22];
    let perf_4s = if vieux > 0.0 { prix / vieux - 1.0 } else { 0.0 };

    let mut conditions = 0u8;
    let passe = |ok: bool, compteur: &mut u8| {
        if ok {
            *compteur += 1;
        }
    };

    // MM150/MM200 : None (historique insuffisant) → condition échouée.
    let c1 = mm150.zip(mm200).is_some_and(|(m150, m200)| prix > m150 && prix > m200);
    passe(c1, &mut conditions);
    let c2 = mm150.zip(mm200).is_some_and(|(m150, m200)| m150 > m200);
    passe(c2, &mut conditions);
    let c3 = mm200.zip(mm200_mois).is_some_and(|(m200, vieux)| m200 > vieux);
    passe(c3, &mut conditions);
    let c4 = mm50
        .zip(mm150)
        .and_then(|(m50, m150)| mm200.map(|m200| (m50, m150, m200)))
        .is_some_and(|(m50, m150, m200)| m50 > m150 && m50 > m200);
    passe(c4, &mut conditions);
    let c5 = mm50.is_some_and(|m50| prix > m50);
    passe(c5, &mut conditions);
    let c6 = bas_52s > 0.0 && prix >= bas_52s * 1.30;
    passe(c6, &mut conditions);
    let c7 = haut_52s > 0.0 && prix >= haut_52s * 0.75;
    passe(c7, &mut conditions);
    let c8 = perf_4s > perf_marche_4s;
    passe(c8, &mut conditions);

    Some(TrendTemplate {
        reussi: conditions == 8,
        conditions,
        prix,
        mm50,
        mm150,
        mm200,
        haut_52s,
        bas_52s,
        perf_4s,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Série de clôtures en uptrend franc : progression régulière 100 → 200
    /// sur 300 séances (MM50 > MM150 > MM200, prix au-dessus de tout).
    fn uptrend() -> Vec<f64> {
        (0..300).map(|i| 100.0 + i as f64 * 0.35).collect()
    }

    /// Downtrend : 200 → 100 — échoue prix > MM et proximité du haut.
    fn downtrend() -> Vec<f64> {
        (0..300).map(|i| 200.0 - i as f64 * 0.35).collect()
    }

    #[test]
    fn uptrend_passe_les_8_conditions() {
        // Rampe +0.35/j ≈ +3.9 % sur 4 semaines : le marché doit faire moins.
        let r = trend_template(&uptrend(), 0.02).unwrap();
        assert!(r.reussi);
        assert_eq!(r.conditions, 8);
        assert!(r.mm200.unwrap() < r.prix);
    }

    #[test]
    fn downtrend_echoue() {
        let r = trend_template(&downtrend(), 0.0).unwrap();
        assert!(!r.reussi);
        // prix sous les MM, loin du haut 52 s, sous +30 % du bas tardivement.
        assert!(r.conditions <= 3);
    }

    #[test]
    fn surperformance_relative_exigee() {
        // Même uptrend, mais le marché fait mieux (+200 %) → condition 8 échoue.
        let r = trend_template(&uptrend(), 2.0).unwrap();
        assert!(!r.reussi);
        assert_eq!(r.conditions, 7);
    }

    #[test]
    fn historique_insuffisant_rend_none() {
        assert!(trend_template(&uptrend()[..200], 0.0).is_none());
    }

    #[test]
    fn proximite_haut_52s() {
        // Uptrend puis repli de 40 % du plus haut → condition 7 échoue
        // (prix < 75 % du haut) même si la structure de MM tient.
        let mut serie = uptrend();
        for i in 0..30 {
            let base = *serie.last().unwrap();
            serie.push(base * (1.0 - 0.40 * (i + 1) as f64 / 30.0));
        }
        let r = trend_template(&serie, -1.0).unwrap();
        assert!(!r.reussi);
        assert!(r.prix < r.haut_52s * 0.75);
    }
}
