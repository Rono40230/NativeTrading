//! Tests de [`crate::smc_v12_collect`] — extraits du fichier principal
//! (règle < 600 lignes/fichier).

use crate::smc_v12_collect::*;
use smc::v12::{BarInput, KillZone, SmcV12Engine};
use crate::smc_v12_out::{compress_vol, runs_str};

#[test]
fn runs_str_regroupe_plages_contigues_et_ignore_none() {
    // asie(0,1,2) | None(3) | londres(4,5) | None(6)
    let raw: Vec<(i64, Option<&str>)> = vec![
        (0, Some("asie")),
        (1, Some("asie")),
        (2, Some("asie")),
        (3, None),
        (4, Some("londres")),
        (5, Some("londres")),
        (6, None),
    ];
    let out = runs_str(&raw);
    assert_eq!(out, vec![(0, 2, "asie"), (4, 5, "londres")]);
}

#[test]
fn runs_str_change_de_label_relance_une_plage() {
    // bull | bull | bear (pas de None) ⇒ deux plages collées.
    let raw: Vec<(i64, Option<&str>)> =
        vec![(10, Some("bull")), (11, Some("bull")), (12, Some("bear"))];
    let out = runs_str(&raw);
    assert_eq!(out, vec![(10, 11, "bull"), (12, 12, "bear")]);
}

#[test]
fn runs_str_vide_renvoie_vide() {
    let raw: Vec<(i64, Option<&str>)> = vec![(0, None), (1, None)];
    assert!(runs_str(&raw).is_empty());
}

#[test]
fn compress_vol_garde_uniquement_les_plages_fortes() {
    // fort(0,1) | faible(2) | fort(3)
    let raw: Vec<(i64, bool)> = vec![(0, true), (1, true), (2, false), (3, true)];
    let out = compress_vol(&raw);
    assert_eq!(out, vec![(0, 1), (3, 3)]);
}

#[test]
fn compress_vol_aucun_fort_renvoie_vide() {
    let raw: Vec<(i64, bool)> = vec![(0, false), (1, false)];
    assert!(compress_vol(&raw).is_empty());
}

#[test]
fn kz_label_regroupage_ny() {
    assert_eq!(kz_label(KillZone::Asian), Some("asie"));
    assert_eq!(kz_label(KillZone::London), Some("londres"));
    assert_eq!(kz_label(KillZone::NyAm), Some("ny"));
    assert_eq!(kz_label(KillZone::NyPm), Some("ny"));
    assert_eq!(kz_label(KillZone::None), None);
}

/// Spec 08/10 (B2, rév. wave) — l'OTE swing vit dans le moteur et remonte
/// intacte jusqu'au JSON : cas M15 4185 → 4067 (chaîne HH/LL de la wave,
/// mid = 4126,0 exact), ancres et naissance aux bons timestamps.
#[test]
fn swing_ote_collectee_et_serialisee_depuis_le_moteur() {
    // Séquence du cas de référence (cf. v12::swing_ote::tests v6) :
    // H 4165 non-HH (pas de précédent), HH 4185 (bar 5), LL 4067 (bar 8),
    // rebond interne 4145 et creux secondaire — la jambe survit.
    let bars: &[(f64, f64)] = &[
        (4100.0, 4090.0),
        (4110.0, 4095.0),
        (4165.0, 4120.0),
        (4140.0, 4125.0),
        (4140.0, 4130.0),
        (4185.0, 4140.0),
        (4165.0, 4110.0),
        (4138.0, 4072.0),
        (4132.0, 4067.0),
        (4145.0, 4120.0),
        (4130.0, 4110.0),
    ];
    let mut engine = SmcV12Engine::new("XAUUSD", "M15");
    let mut ts_by_idx: Vec<i64> = Vec::new();
    let mut col = BarCollectors::new(bars.len(), 1.0);
    for (i, &(h, l)) in bars.iter().enumerate() {
        let bar = BarInput {
            timestamp: 1_700_000_000i64 + (i as i64) * 900,
            open: l,
            high: h,
            low: l,
            close: h,
            volume: 0.0,
        };
        ts_by_idx.push(bar.timestamp);
        let out = engine.update(&bar);
        col.on_bar(&bar, &out);
    }
    let ext = collect_final_extended(&engine, &ts_by_idx, col);

    let z = ext.swing_ote.clone().expect("OTE swing vivante après replay");
    assert_eq!(z.dir, "bear", "LL plus récent que le HH ⇒ jambe baissière");
    assert_eq!(z.mid, 4126.0, "trait 50 % = 4126,0 exact (capture M15)");
    assert!((z.bot - 4139.924).abs() < 1e-9);
    assert!((z.top - 4159.748).abs() < 1e-9);
    assert_eq!(z.pivot_haut.prix, 4185.0);
    assert_eq!(z.pivot_bas.prix, 4067.0);
    assert_eq!(z.pivot_haut.ts, ts_by_idx[5], "ancre haute = barre HH 5");
    assert_eq!(z.pivot_bas.ts, ts_by_idx[8], "ancre basse = barre LL 8");
    assert_eq!(z.ts_naissance, ts_by_idx[5], "naissance = ancre la plus ancienne (HH)");

    // Clés JSON stables pour le frontend (contract sérialisation).
    let json = serde_json::to_value(&ext).expect("ExtendedOutputs sérialisable");
    let so = json
        .get("swing_ote")
        .and_then(|v| v.as_object())
        .expect("clé swing_ote présente");
    for cle in ["dir", "top", "bot", "mid", "ts_naissance", "pivot_haut", "pivot_bas"] {
        assert!(so.contains_key(cle), "clé {cle} absente du JSON");
    }
    assert_eq!(so["dir"].as_str(), Some("bear"));
    assert_eq!(so["pivot_haut"]["prix"].as_f64(), Some(4185.0));
}

/// Spec 08/10 (B2) — sans jambe complète vivante (plateau : aucun pivot),
/// `swing_ote` vaut `null` dans le JSON.
#[test]
fn swing_ote_null_sans_jambe() {
    let mut engine = SmcV12Engine::new("XAUUSD", "M15");
    let mut ts_by_idx: Vec<i64> = Vec::new();
    let mut col = BarCollectors::new(8, 1.0);
    for i in 0..8usize {
        let bar = BarInput {
            timestamp: 1_700_000_000i64 + (i as i64) * 900,
            open: 4090.0,
            high: 4100.0,
            low: 4090.0,
            close: 4100.0,
            volume: 0.0,
        };
        ts_by_idx.push(bar.timestamp);
        let out = engine.update(&bar);
        col.on_bar(&bar, &out);
    }
    let ext = collect_final_extended(&engine, &ts_by_idx, col);
    assert!(ext.swing_ote.is_none(), "plateau ⇒ aucun pivot ⇒ aucune OTE");
    let json = serde_json::to_value(&ext).unwrap();
    assert!(json.get("swing_ote").map(|v| v.is_null()).unwrap_or(false));
}
