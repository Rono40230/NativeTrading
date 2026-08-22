//! Structs de sérialisation + helpers d'affichage pour `/api/smc/v12/analyse`.
//!
//! Regroupe les types de sortie JSON (`V12AnalyseResponse` + structs par indicateur),
//! les constantes FIFO d'affichage et les utilitaires de troncature/rendu. La
//! collecte des indicateurs étendus (états finaux + séries par barre) vit dans
//! [`crate::smc_v12_collect`].

use serde::{Deserialize, Serialize};
use smc::v12::trade::Verdict;
use smc::v12::{FvgState, ObState};

// ── Limites FIFO d'affichage (Pine « max visible ») ──────────────────────────
pub(crate) const MAX_BOS: usize = 6;
pub(crate) const MAX_STRUCTURE: usize = 12;
pub(crate) const MAX_MSS: usize = 6;
pub(crate) const MAX_CHOCH: usize = 6;
pub(crate) const MAX_SWEEPS: usize = 6;
pub(crate) const MAX_FVG_PAR_SENS: usize = 10;
// Zones-cœur : miroir des boxes LIVE du moteur (supprimées à l'invalidation) —
// bornées par le nombre d'OB actifs, pas de plafond côté collecteur.
// OB déjà plafonnés à MAX_OB=40 par sens, Breaker à 5/sens, Imbalance à 10/sens,
// EQH/EQL pool ≤ 20, NDOG/NWOG 1/type — tous côté moteur.

#[derive(Deserialize)]
pub(crate) struct V12Query {
    pub asset: String,
    pub timeframe: Option<String>,
    pub limit: Option<u32>,
}

// ── Structs de sérialisation (indicateurs originaux) ─────────────────────────

#[derive(Serialize)]
pub(crate) struct PivotOut {
    pub ts: i64,
    #[serde(rename = "type")]
    pub ptype: &'static str,
    pub price: f64,
    pub bar_idx: usize,
}

#[derive(Serialize)]
pub(crate) struct NiveauStructOut {
    pub ts: i64,
    pub pivot_ts: i64,
    pub dir: &'static str,
    pub level: f64,
    pub bar_idx: usize,
    /// Bougie du sweep (pour ancrer l'étiquette dessus) — sweeps uniquement.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub candle_high: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub candle_low: Option<f64>,
}

#[derive(Serialize)]
pub(crate) struct ObOut {
    pub ts: i64,
    pub dir: &'static str,
    pub top: f64,
    pub bot: f64,
    pub state: &'static str,
    pub force: i32,
    pub bar_idx: usize,
    /// Flags actifs au moment du nouveau max du score (diag MQL5 diagFlags).
    pub diag: Option<String>,
}

#[derive(Serialize)]
pub(crate) struct FvgOut {
    pub ts: i64,
    pub dir: &'static str,
    pub top: f64,
    pub bot: f64,
    pub state: &'static str,
    pub bar_idx: usize,
}

#[derive(Serialize)]
pub(crate) struct SignalOut {
    pub ts: i64,
    pub dir: &'static str,
    pub entry: f64,
    pub sl: f64,
    pub tp1: f64,
    pub tp2: f64,
    pub tp3: f64,
    pub force: i32,
    pub source: &'static str,
    pub verdict: &'static str,
    /// Score de sentiment de la classe de l'asset (None si indispo).
    pub sentiment: Option<f64>,
    /// Alignement direction × sentiment ("aligne"|"oppose"|"neutre"|"extreme").
    pub alignement: Option<&'static str>,
}

// ── Structs de sérialisation pour les indicateurs supplémentaires ─────────────

/// Niveau de liquidité précédent (PDH/PDL/PWH/PWL) : prix brut + drapeau actif.
#[derive(Serialize)]
pub(crate) struct LiquiditeLevelOut {
    pub level: &'static str, // "pdh" | "pdl" | "pwh" | "pwl"
    pub price: Option<f64>,
    pub active: bool,
    /// Timestamp où le niveau s'est formé (bord gauche de la ligne, Pine
    /// _prevDayHighTime).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ts_origine: Option<i64>,
}

/// Niveau EQH/EQL du pool de liquidités.
#[derive(Serialize)]
pub(crate) struct EqOut {
    pub dir: &'static str, // "high" (EQH) | "low" (EQL)
    pub price: f64,
    pub touches: u32,
    pub swept: bool,
    pub bar_idx: usize,
    /// Timestamp du 1er pivot (bord gauche de la ligne, comme le Pine
    /// `line.new(_ll.tFirst, ...)` — pas pleine largeur).
    pub ts: i64,
}

/// Breaker block actif (zones déjà plafonnées à 5/sens côté moteur).
/// Propulsion Block actif (MODULE 8c : FVG ∩ OB même sens, FIFO 3/sens).
#[derive(Serialize)]
pub(crate) struct PropulsionOut {
    pub ts: i64,
    pub dir: &'static str,
    pub top: f64,
    pub bot: f64,
}

#[derive(Serialize)]
pub(crate) struct BreakerOut {
    pub ts: i64,
    pub dir: &'static str,
    pub top: f64,
    pub bot: f64,
    pub bar_idx: usize,
}

/// Imbalance active (plafonnée à 10/sens côté moteur).
#[derive(Serialize)]
pub(crate) struct ImbalanceOut {
    pub ts: i64,
    pub dir: &'static str,
    pub top: f64,
    pub bot: f64,
    pub state: &'static str,
    pub bar_idx: usize,
}

/// Box OTE d'affichage (au plus une par sens, Pine `_oteBullBox`/`_oteBearBox`).
#[derive(Serialize)]
pub(crate) struct OteOut {
    pub dir: &'static str,
    pub top: f64,
    pub bot: f64,
    /// Timestamp de la bar du BOS (bord gauche de la box).
    pub ts: i64,
}

/// Zone-cœur (intersection OB ∩ OTE ∩ FVG) collectée pendant le replay.
#[derive(Serialize)]
pub(crate) struct ZoneCoeurOut {
    pub ts: i64,
    /// Timestamp de l'OB parent (bornes X de la zone cœur = celles de l'OB,
    /// Pine sous-box de l'OB).
    pub ob_ts: i64,
    pub dir: &'static str,
    pub top: f64,
    pub bot: f64,
    pub ob_bar: usize,
}

/// État final Premium/Discount (équilibrium ICT + plage du dernier dealing range).
#[derive(Serialize, Default)]
pub(crate) struct PdOut {
    pub pd_range_h: Option<f64>,
    pub pd_range_l: Option<f64>,
    pub equilibrium: Option<f64>,
    pub in_premium: bool,
    pub in_discount: bool,
}

/// Order Block HTF actif (MTF H1/H4/W1/MN).
#[derive(Serialize)]
pub(crate) struct HtfObOut {
    pub timeframe: &'static str,
    pub dir: &'static str,
    pub top: f64,
    pub bot: f64,
    pub ts: i64,
}

/// Plage contiguë de session (Kill Zone) — compression run-length.
#[derive(Serialize)]
pub(crate) struct SessionRange {
    pub start_ts: i64,
    pub end_ts: i64,
    pub session: &'static str, // "asie" | "londres" | "ny"
}

/// Niveaux Asian High/Low (range de la session Asie du jour le plus récent).
#[derive(Serialize)]
pub(crate) struct AsianHlOut {
    pub high: f64,
    pub low: f64,
    pub invalidated_up: bool,
    pub invalidated_down: bool,
    /// 1re bougie de la session Asie (bord gauche des lignes, Pine _ahStartBar).
    pub start_ts: i64,
}

/// Gap NDOG/NWOG actif (FIFO 1 par type côté moteur).
#[derive(Serialize)]
pub(crate) struct GapOut {
    pub ts: i64,
    pub gtype: &'static str, // "ndog" | "nwog"
    pub top: f64,
    pub bot: f64,
    pub mitigated: bool,
    pub bar_idx: usize,
}

/// Plage contiguë de volume fort (volume > volMA) — compression run-length.
#[derive(Serialize)]
pub(crate) struct VolRange {
    pub start_ts: i64,
    pub end_ts: i64,
}

/// Plage contiguë d'impulsion (corps > seuil_ib×ATR) — compression run-length.
#[derive(Serialize)]
pub(crate) struct ImpRange {
    pub start_ts: i64,
    pub end_ts: i64,
    pub impulsion: &'static str, // "bull" | "bear"
}

/// Plage premium/discount (bgcolor par barre, Pine MODULE 4b).
#[derive(Serialize)]
pub(crate) struct PremRange {
    pub start_ts: i64,
    pub end_ts: i64,
    pub dir: &'static str, // "prem" | "disc"
}

/// Plage de tendance (bgcolor vert/rouge par barre, Pine MODULE 1 :
/// tendanceHaussiere = bullCount>=2 → C_BG_BULL, sinon C_BG_BEAR).
#[derive(Serialize)]
pub(crate) struct TrendRange {
    pub start_ts: i64,
    pub end_ts: i64,
    pub dir: &'static str, // "bull" | "bear"
}

/// Box de session complète (Pine MODULE 14 : rectangles du range high/low
/// de la session, heures Europe/Paris, historique 24 h).
#[derive(Serialize)]
pub(crate) struct SessionBox {
    pub start_ts: i64,
    pub end_ts: i64,
    pub session: &'static str, // "asie" | "londres" | "ny"
    pub high: f64,
    pub low: f64,
}

/// Tous les indicateurs étendus (sérialisés à plat dans la réponse via `#[serde(flatten)]`).
#[derive(Serialize, Default)]
pub(crate) struct ExtendedOutputs {
    pub liquidites: Vec<LiquiditeLevelOut>,
    pub eqs: Vec<EqOut>,
    pub breakers: Vec<BreakerOut>,
    pub propulsions: Vec<PropulsionOut>,
    pub imbalances: Vec<ImbalanceOut>,
    pub otes: Vec<OteOut>,
    pub zone_coeur: Vec<ZoneCoeurOut>,
    pub premium_discount: PdOut,
    pub mtf_obs: Vec<HtfObOut>,
    pub sessions: Vec<SessionRange>,
    pub trend_ranges: Vec<TrendRange>,
    pub prem_ranges: Vec<PremRange>,
    pub session_boxes: Vec<SessionBox>,
    pub asian_hl: Option<AsianHlOut>,
    pub gaps: Vec<GapOut>,
    pub vol_fort: Vec<VolRange>,
    pub impulsions: Vec<ImpRange>,
}

#[derive(Serialize)]
pub(crate) struct V12AnalyseResponse {
    pub asset: String,
    pub timeframe: String,
    pub nb_bougies: usize,
    pub pivots: Vec<PivotOut>,
    pub bos: Vec<NiveauStructOut>,
    pub mss: Vec<NiveauStructOut>,
    pub chochs: Vec<NiveauStructOut>,
    pub sweeps: Vec<NiveauStructOut>,
    pub obs: Vec<ObOut>,
    pub fvgs: Vec<FvgOut>,
    pub signals: Vec<SignalOut>,
    pub tendance: &'static str,
    pub atr14: f64,
    #[serde(flatten)]
    pub extended: ExtendedOutputs,
}

// ── Helpers d'affichage partagés ─────────────────────────────────────────────

/// Récupère le timestamp d'une bar par son index global (fallback sécurisé).
// ── Compression run-length (séries par barre du collecteur) ──────────────────

/// Compression run-length d'une série `(ts, label Optionnel)` : renvoie les plages
/// contiguës `(start_ts, end_ts, label)` pour les valeurs non-`None`. Les `None`
/// cassent la contiguïté (saut de session / pas d'impulsion).
pub(crate) fn runs_str(raw: &[(i64, Option<&'static str>)]) -> Vec<(i64, i64, &'static str)> {
    let mut out: Vec<(i64, i64, &'static str)> = Vec::new();
    let mut cur: Option<(&'static str, i64, i64)> = None;
    for &(ts, label) in raw {
        match label {
            Some(v) => match cur {
                Some((c, st, _)) if c == v => cur = Some((c, st, ts)),
                _ => {
                    if let Some((c, st, en)) = cur {
                        out.push((st, en, c));
                    }
                    cur = Some((v, ts, ts));
                }
            },
            None => {
                if let Some((c, st, en)) = cur.take() {
                    out.push((st, en, c));
                }
            }
        }
    }
    if let Some((c, st, en)) = cur {
        out.push((st, en, c));
    }
    out
}

/// Compression run-length d'une série booléenne : renvoie les plages `(start, end)`
/// contiguës où le flag vaut `true` (utilisé pour le volume fort).
pub(crate) fn compress_vol(raw: &[(i64, bool)]) -> Vec<(i64, i64)> {
    let mut out: Vec<(i64, i64)> = Vec::new();
    let mut cur: Option<(i64, i64)> = None;
    for &(ts, fort) in raw {
        if fort {
            match cur {
                Some((st, _)) => cur = Some((st, ts)),
                None => cur = Some((ts, ts)),
            }
        } else if let Some((st, en)) = cur.take() {
            out.push((st, en));
        }
    }
    if let Some((st, en)) = cur {
        out.push((st, en));
    }
    out
}

pub(crate) fn ts_at(ts_by_idx: &[i64], idx: usize, fallback: i64) -> i64 {
    if idx < ts_by_idx.len() {
        ts_by_idx[idx]
    } else {
        fallback
    }
}

/// Garde uniquement les `n` derniers éléments (chronologiques, dernier = le plus récent).
pub(crate) fn garder_derniers<T>(v: &mut Vec<T>, n: usize) {
    if v.len() > n {
        let drain = v.len() - n;
        v.drain(0..drain);
    }
}

/// FVG : limite à `n` par sens (les plus récents = bar_idx le plus grand).
pub(crate) fn garder_derniers_fvg_par_sens(mut fvgs: Vec<FvgOut>, n: usize) -> Vec<FvgOut> {
    fvgs.sort_by_key(|f| (f.dir == "bull", f.bar_idx)); // bull d'abord, puis bar_idx asc
    let mut out: Vec<FvgOut> = Vec::with_capacity(fvgs.len());
    let mut bull_kept = 0usize;
    let mut bear_kept = 0usize;
    for f in fvgs.into_iter().rev() {
        let kept = if f.dir == "bull" {
            &mut bull_kept
        } else {
            &mut bear_kept
        };
        if *kept < n {
            out.push(f);
            *kept += 1;
        }
    }
    out
}

pub(crate) fn ob_state_str(s: ObState) -> &'static str {
    match s {
        ObState::Vierge => "vierge",
        ObState::Partiel => "partiel",
        ObState::Profond => "profond",
    }
}

pub(crate) fn fvg_state_str(s: FvgState) -> &'static str {
    match s {
        FvgState::Fresh => "vierge",
        FvgState::Partial => "partiel",
    }
}

pub(crate) fn verdict_str(v: Verdict) -> &'static str {
    match v {
        Verdict::Tp3 => "TP3",
        Verdict::Tp2 => "TP2",
        Verdict::Tp1 => "TP1",
        Verdict::Sl => "SL",
        Verdict::Be => "BE",
        Verdict::Expire => "Expire",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn garder_derniers_tronque_tete() {
        let mut v: Vec<i32> = (0..10).collect();
        garder_derniers(&mut v, 3);
        assert_eq!(v, vec![7, 8, 9]);
    }

    #[test]
    fn garder_derniers_sous_limite_inchange() {
        let mut v = vec![1, 2];
        garder_derniers(&mut v, 5);
        assert_eq!(v, vec![1, 2]);
    }

    #[test]
    fn fvg_par_sens_limite_a_n() {
        let fvgs = vec![
            FvgOut {
                ts: 0,
                dir: "bull",
                top: 1.0,
                bot: 0.0,
                state: "vierge",
                bar_idx: 1,
            },
            FvgOut {
                ts: 0,
                dir: "bull",
                top: 1.0,
                bot: 0.0,
                state: "vierge",
                bar_idx: 2,
            },
            FvgOut {
                ts: 0,
                dir: "bear",
                top: 1.0,
                bot: 0.0,
                state: "vierge",
                bar_idx: 3,
            },
        ];
        let out = garder_derniers_fvg_par_sens(fvgs, 1);
        let bulls = out.iter().filter(|f| f.dir == "bull").count();
        let bears = out.iter().filter(|f| f.dir == "bear").count();
        assert_eq!(bulls, 1);
        assert_eq!(bears, 1);
        // Le bull conservé doit être le plus récent (bar_idx=2).
        assert!(out.iter().any(|f| f.dir == "bull" && f.bar_idx == 2));
    }

    #[test]
    fn ts_at_fallback_si_hors_plage() {
        let t = ts_at(&[10, 20, 30], 5, 99);
        assert_eq!(t, 99);
        assert_eq!(ts_at(&[10, 20, 30], 1, 99), 20);
    }
}
