use common::Candle;

/// Average Directional Index (Wilder, période standard = 14).
/// NaN pendant la chauffe ; convention de seed identique à calculer_atr
/// (SMA des `periode` premières valeurs, puis lissage Wilder).
/// ADX mesure la FORCE de tendance indépendamment du sens : ≥ 25 =
/// tendance franche, < 20 = range (seuils d'usage courant).
pub fn calculer_adx(bougies: &[Candle], periode: usize) -> Vec<f64> {
    let n = bougies.len();
    let mut adx = vec![f64::NAN; n];
    if periode < 2 || n < 3 * periode {
        return adx;
    }

    // +DM / −DM / TR par barre (indice k = barre k).
    let mut plus_dm = vec![0.0; n];
    let mut minus_dm = vec![0.0; n];
    let mut tr = vec![0.0; n];
    for i in 1..n {
        let up = bougies[i].high - bougies[i - 1].high;
        let down = bougies[i - 1].low - bougies[i].low;
        plus_dm[i] = if up > down && up > 0.0 { up } else { 0.0 };
        minus_dm[i] = if down > up && down > 0.0 { down } else { 0.0 };
        let hl = bougies[i].high - bougies[i].low;
        let hc = (bougies[i].high - bougies[i - 1].close).abs();
        let lc = (bougies[i].low - bougies[i - 1].close).abs();
        tr[i] = hl.max(hc).max(lc);
    }

    // Lissage Wilder de +DM, −DM, TR : seed = somme des `periode` premières
    // valeurs (barres 1..=periode), puis accumulation glissante.
    let lisse = |serie: &[f64]| -> Vec<f64> {
        let mut out = vec![f64::NAN; n];
        let mut somme: f64 = (1..=periode).map(|i| serie[i]).sum();
        out[periode] = somme;
        for i in (periode + 1)..n {
            somme = somme - somme / periode as f64 + serie[i];
            out[i] = somme;
        }
        out
    };
    let s_plus = lisse(&plus_dm);
    let s_minus = lisse(&minus_dm);
    let s_tr = lisse(&tr);

    // DX puis ADX = lissage Wilder du DX (seed = SMA des `periode` premiers DX).
    let mut dx = vec![f64::NAN; n];
    for i in periode..n {
        if s_tr[i] > 0.0 {
            let pdi = 100.0 * s_plus[i] / s_tr[i];
            let mdi = 100.0 * s_minus[i] / s_tr[i];
            let somme_di = pdi + mdi;
            if somme_di > 0.0 {
                dx[i] = 100.0 * (pdi - mdi).abs() / somme_di;
            }
        }
    }
    let debut_dx = 2 * periode; // premier DX utilisable (après seed TR/DM)
    let seed: f64 = (debut_dx..debut_dx + periode).map(|i| dx[i]).sum::<f64>() / periode as f64;
    adx[debut_dx + periode - 1] = seed;
    for i in (debut_dx + periode)..n {
        adx[i] = (adx[i - 1] * (periode as f64 - 1.0) + dx[i]) / periode as f64;
    }
    adx
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn bougie_ts(ts: i64, o: f64, h: f64, l: f64, c: f64) -> Candle {
        Candle {
            timestamp: Utc::now() + chrono::Duration::seconds(ts),
            open: o,
            high: h,
            low: l,
            close: c,
            volume: 1000.0,
        }
    }

    fn rampe(depart: f64, pas: f64, nb: usize, ts: i64) -> Vec<Candle> {
        (0..nb)
            .map(|k| {
                let p = depart + pas * k as f64;
                bougie_ts(ts + k as i64, p, p + 0.5, p - 0.5, p)
            })
            .collect()
    }

    #[test]
    fn tailles_et_chauffe() {
        let b = rampe(100.0, 1.0, 100, 0);
        let adx = calculer_adx(&b, 14);
        assert_eq!(adx.len(), 100);
        // Chauffe : seed +DM/−DM/TR (14), seed DX (14), seed ADX (14) →
        // première valeur à 3×14 − 1 = 41.
        assert!(adx[40].is_nan());
        assert!(adx[41].is_finite());
    }

    #[test]
    fn montee_franche_adx_eleve() {
        // Rampe régulière : direction unique, ADX doit grimper vers 100.
        let b = rampe(100.0, 1.0, 200, 0);
        let adx = calculer_adx(&b, 14);
        assert!(adx[199] > 60.0, "ADX montée = {:.1}", adx[199]);
    }

    #[test]
    fn plage_adx_bas() {
        // Oscillation serrée autour d'un niveau : pas de direction → ADX bas.
        let b: Vec<Candle> = (0..200)
            .map(|k| {
                let p = 100.0 + (k % 4) as f64 - 1.5; // cycle 4 barres
                bougie_ts(k as i64, p, p + 0.5, p - 0.5, p)
            })
            .collect();
        let adx = calculer_adx(&b, 14);
        assert!(adx[199] < 40.0, "ADX range = {:.1}", adx[199]);
    }

    #[test]
    fn periode_trop_courte_nan() {
        let b = rampe(100.0, 1.0, 20, 0);
        assert!(calculer_adx(&b, 14).iter().all(|v| v.is_nan()));
    }
}
