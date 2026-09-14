//! KdjEngine — moteur live KDJ/Halftrend (7.E), miroir du rejeur.
//!
//! Modèle (docs/reference/definition_kdj_halftrend.md) : évaluation à la
//! CLÔTURE seule, exécution à l'OPEN de la barre suivante (piège 1 : aucun
//! ordre limit/stop — TP/SL = seuils de détection), fige des niveaux à la
//! barre d'entrée (piège 3 : valuewhen), crosses (piège 4).
//!
//! Implémentation 100 % `on_close` : les exécutions différées (entrée,
//! clôture) se déclenchent au prix `open` de la clôture SUIVANTE — fidèle au
//! rejeur ET naturellement compatible du cold start (le replay ne fournit
//! que des bougies clôturées). Les mêmes fonctions indicateurs que le
//! rejeur sont rappelées à chaque clôture (fenêtre glissante) : parité par
//! construction, comme l'EA MT5 validé en 7.D.
//!
//! Le `SignalBrut` part à la clôture de la barre de signal N (alerte
//! immédiate, comme TradingView) avec des niveaux ESTIMÉS sur N ; la gestion
//! interne (crosses, R) vit sur les niveaux RÉELS figés à la barre d'entrée
//! N+1 (`open(N+1)`, `EMA200(N+1)`).

use common::{Asset, Candle, Direction, Timeframe};
use engine::{ContexteCloture, Engine, EvenementTrade, SignalBrut, SortieMoteur, TypeEvenementTrade};
use indicators::{calculer_adx, calculer_ema, calculer_halftrend, calculer_kdj, calculer_sma};

/// Nom sur le bus — doit matcher le manifeste du registre.
pub const NOM: &str = "kdj_halftrend";

/// Barres en mémoire (même convention que l'EA MT5 : InpFenetre = 3000).
const FENETRE: usize = 3000;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Verdict {
    Tp,
    Sl,
    Retournement,
}

impl Verdict {
    fn libelle(&self) -> &'static str {
        match self {
            Verdict::Tp => "TP",
            Verdict::Sl => "SL",
            Verdict::Retournement => "Retournement",
        }
    }
}

/// Position en cours — niveaux figés à la clôture de la barre d'entrée.
struct PositionLive {
    direction: i32,
    cle: String,
    /// Début (epoch) de la barre d'entrée N+1.
    ts_barre_entree: i64,
    /// open(N+1) — le prix d'entrée réel.
    prix_entree: f64,
    fige: bool,
    sl_niveau: f64,
    tp_niveau: f64,
    risque: f64,
}

/// Exécution décidée à la clôture M, effective au open de M+1 (dans l'ordre
/// du vecteur : clôture d'abord, entrée ensuite — retournement TV).
enum Differe {
    Entree { direction: i32, cle: String },
    Cloture { verdict: Verdict, cle: String, prix_entree: f64, risque: f64, direction: i32 },
}

pub struct KdjEngine {
    asset: Asset,
    tf: Timeframe,
    params: crate::ParamsKdj,
    historique: Vec<Candle>,
    position: Option<PositionLive>,
    differes: Vec<Differe>,
}

impl KdjEngine {
    pub fn nouveau(asset: Asset, tf: Timeframe) -> Self {
        Self {
            asset,
            tf,
            params: crate::ParamsKdj::default(),
            historique: Vec::new(),
            position: None,
            differes: Vec::new(),
        }
    }

    pub fn avec_params(mut self, params: crate::ParamsKdj) -> Self {
        self.params = params;
        self
    }

    /// Exécution des différés au `open` de la bougie clôturée courante
    /// (= l'open de la barre qui suivait la décision).
    fn executer_differes(&mut self, bougie: &Candle, sortie: &mut SortieMoteur) {
        for differe in std::mem::take(&mut self.differes) {
            match differe {
                Differe::Cloture { verdict, cle, prix_entree, risque, direction } => {
                    let r = if risque > 0.0 {
                        direction as f64 * (bougie.open - prix_entree) / risque
                    } else {
                        0.0
                    };
                    sortie.evenements.push(EvenementTrade {
                        moteur: NOM.to_string(),
                        asset: self.asset.clone(),
                        tf: self.tf,
                        cle_trade: cle,
                        evenement: TypeEvenementTrade::Cloture,
                        detail: format!("{}|{:.4}", verdict.libelle(), r),
                        prix: bougie.open,
                        debut_barre: bougie.timestamp.timestamp(),
                        emis_le: chrono::Utc::now(),
                    });
                    self.position = None;
                }
                Differe::Entree { direction, cle } => {
                    self.position = Some(PositionLive {
                        direction,
                        cle: cle.clone(),
                        ts_barre_entree: bougie.timestamp.timestamp(),
                        prix_entree: bougie.open,
                        fige: false,
                        sl_niveau: f64::NAN,
                        tp_niveau: f64::NAN,
                        risque: 0.0,
                    });
                    sortie.evenements.push(EvenementTrade {
                        moteur: NOM.to_string(),
                        asset: self.asset.clone(),
                        tf: self.tf,
                        cle_trade: cle,
                        evenement: TypeEvenementTrade::Fill,
                        detail: "open".into(),
                        prix: bougie.open,
                        debut_barre: bougie.timestamp.timestamp(),
                        emis_le: chrono::Utc::now(),
                    });
                }
            }
        }
    }

    /// Signal à la clôture de la barre `i` : niveaux ESTIMÉS sur `i` (alerte
    /// immédiate) ; la gestion interne vivra sur les niveaux réels figés.
    fn emettre_signal(&self, direction: i32, i: usize, ema200: &[f64], close: f64, ts: i64, adx: Option<f64>) -> SignalBrut {
        let ema = ema200[i];
        let distance = close - ema;
        let tp = if direction == 1 {
            close + self.params.ratio_risk * distance
        } else {
            close - self.params.ratio_risk * (ema - close)
        };
        let score = adx.map(|a| a.clamp(1.0, 100.0) as i32).unwrap_or(50);
        SignalBrut::avec_cle(
            NOM,
            self.asset.clone(),
            self.tf,
            if direction == 1 { Direction::Long } else { Direction::Short },
            close,
            ema,
            vec![tp],
            score,
            format!(
                "HalfTrend {} + KDJ {} + {}",
                if direction == 1 { "retourné haussier" } else { "retourné baissier" },
                if direction == 1 { "J>D" } else { "J<D" },
                if direction == 1 { "close>EMA200" } else { "close<EMA200" }
            ),
            ts,
            format!("kdj-{}-{}", self.asset.as_str(), ts),
        )
    }
}

impl Engine for KdjEngine {
    fn nom(&self) -> &str {
        NOM
    }

    /// Évaluation à la clôture — tout le cycle vit ici (cf. module doc).
    fn on_close(&mut self, ctx: &ContexteCloture) -> SortieMoteur {
        let mut sortie = SortieMoteur::vide();
        let bougie = ctx.bougie.clone();

        // 1) Exécutions différées au open de cette barre.
        self.executer_differes(&bougie, &mut sortie);

        // 2) Accumulation (fenêtre glissante).
        self.historique.push(bougie);
        if self.historique.len() > FENETRE {
            let excedent = self.historique.len() - FENETRE;
            self.historique.drain(..excedent);
        }
        let n = self.historique.len();
        if n < 260 {
            return sortie; // warm-up EMA200 + ATR100 + marge.
        }
        let i = n - 1;

        // 3) Indicateurs recalculés sur la fenêtre (parité rejeur/EA).
        let kdj = calculer_kdj(&self.historique, self.params.period, self.params.signal);
        let ht = calculer_halftrend(&self.historique, self.params.amplitude);
        let sma100 = calculer_sma(&self.historique, 100);
        let ema200 = calculer_ema(&self.historique, 200);
        let adx = self.params.adx_min.map(|_| calculer_adx(&self.historique, 14));

        // 4) Fige à la clôture de la barre d'entrée (valuewhen — piège 3).
        if let Some(p) = self.position.as_mut() {
            if !p.fige
                && p.ts_barre_entree == self.historique[i].timestamp.timestamp()
                && ema200[i].is_finite()
            {
                let e = p.prix_entree;
                let ema_e = ema200[i];
                let distance = e - ema_e;
                p.tp_niveau = if p.direction == 1 {
                    e + self.params.ratio_risk * distance
                } else {
                    e - self.params.ratio_risk * (ema_e - e)
                };
                p.sl_niveau = ema_e;
                p.risque = distance.abs();
                p.fige = true;
            }
        }

        // 5) Signaux à la clôture de la barre i.
        let cond_ok = sma100[i].is_finite()
            && ema200[i].is_finite()
            && kdj.j[i].is_finite()
            && kdj.d[i].is_finite();
        let long_ok = cond_ok
            && sma100[i] > ema200[i]
            && kdj.j[i] > kdj.d[i]
            && self.historique[i].close > ema200[i];
        let short_ok = cond_ok
            && sma100[i] < ema200[i]
            && kdj.j[i] < kdj.d[i]
            && self.historique[i].close < ema200[i];
        let tendance_ok = match self.params.adx_min {
            None => true,
            Some(min) => adx.as_ref().is_some_and(|a| a[i].is_finite() && a[i] >= min),
        };
        let buy = ht.arrow_up[i] && long_ok && tendance_ok;
        let sell = ht.arrow_down[i] && short_ok && tendance_ok;

        let ts_i = self.historique[i].timestamp.timestamp();
        let close_i = self.historique[i].close;
        let adx_i = adx.as_ref().map(|a| a[i]);

        if let Some(p) = self.position.as_ref() {
            // 6) Position ouverte : crosses TP/SL (piège 4) puis retournement.
            if !p.fige || !self.differes.is_empty() {
                return sortie;
            }
            let (h_prec, l_prec) = (self.historique[i - 1].high, self.historique[i - 1].low);
            let (h, l) = (self.historique[i].high, self.historique[i].low);
            let (tp, sl) = if p.direction == 1 {
                (
                    h_prec <= p.tp_niveau && h > p.tp_niveau,
                    l_prec >= p.sl_niveau && l < p.sl_niveau,
                )
            } else {
                (
                    l_prec >= p.tp_niveau && l < p.tp_niveau,
                    h_prec <= p.sl_niveau && h > p.sl_niveau,
                )
            };
            let inverse = (p.direction == 1 && sell) || (p.direction == -1 && buy);
            if tp || sl || inverse {
                self.differes.push(Differe::Cloture {
                    verdict: if tp { Verdict::Tp } else if sl { Verdict::Sl } else { Verdict::Retournement },
                    cle: p.cle.clone(),
                    prix_entree: p.prix_entree,
                    risque: p.risque,
                    direction: p.direction,
                });
                if inverse {
                    // Retournement TV : la nouvelle position ouvre au MÊME open.
                    let direction = -p.direction;
                    sortie.signaux.push(
                        self.emettre_signal(direction, i, &ema200, close_i, ts_i, adx_i),
                    );
                    self.differes.push(Differe::Entree {
                        direction,
                        cle: format!("kdj-{}-{}", self.asset.as_str(), ts_i),
                    });
                }
            }
            return sortie;
        }

        // 7) À plat : entrée différée + signal (alerte immédiate).
        if self.differes.is_empty() && (buy || sell) {
            let direction = if buy { 1 } else { -1 };
            let signal = self.emettre_signal(direction, i, &ema200, close_i, ts_i, adx_i);
            self.differes.push(Differe::Entree { direction, cle: signal.cle.clone() });
            sortie.signaux.push(signal);
        }
        sortie
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ParamsKdj;
    use chrono::Utc;

    fn bougie_ts(ts: i64, p: f64) -> Candle {
        Candle {
            timestamp: Utc::now() + chrono::Duration::seconds(ts),
            open: p,
            high: p + 0.4,
            low: p - 0.4,
            close: p,
            volume: 1000.0,
        }
    }

    fn rampe(depart: f64, pas: f64, nb: usize, ts_debut: i64) -> Vec<Candle> {
        (0..nb)
            .map(|k| bougie_ts(ts_debut + k as i64, depart + pas * k as f64))
            .collect()
    }

    /// PARITÉ live ↔ rejeur : le scénario « V violent » du rejeur, piloté
    /// bougie par bougie à travers on_close — mêmes trade, prix et verdict.
    #[test]
    fn parite_avec_le_rejeur() {
        let mut b = rampe(100.0, 1.0, 300, 0);
        b.extend(rampe(400.0, -2.0, 60, 300));
        b.extend(rampe(280.0, 2.0, 150, 360));
        b.extend(rampe(578.0, -2.0, 40, 510));
        b.extend(rampe(500.0, 40.0, 12, 550));

        // Référence : le rejeur batch.
        let trades = crate::rejouer(&b, &ParamsKdj::default());
        assert!(!trades.is_empty());
        let t = &trades[0];

        // Le moteur live, clôture par clôture.
        let asset = Asset::try_from("XAUUSD").expect("asset connu");
        let mut moteur = KdjEngine::nouveau(asset.clone(), Timeframe::H1);
        let mut signaux = Vec::new();
        let mut evenements = Vec::new();
        for (k, bougie) in b.iter().enumerate() {
            let ctx = ContexteCloture { asset: &asset, tf: Timeframe::H1, bougie, index_barre: k };
            let s = moteur.on_close(&ctx);
            signaux.extend(s.signaux);
            evenements.extend(s.evenements);
        }

        // Un signal (Buy), un Fill à l'open de la barre d'entrée, une
        // clôture TP avec le MÊME R que le rejeur.
        assert_eq!(signaux.len(), 1, "un signal exactement");
        assert_eq!(signaux[0].direction, Direction::Long);
        let fills: Vec<_> = evenements
            .iter()
            .filter(|e| e.evenement == TypeEvenementTrade::Fill)
            .collect();
        assert_eq!(fills.len(), 1);
        assert!((fills[0].prix - t.prix_entree).abs() < 1e-9, "entrée au centime");
        let clotures: Vec<_> = evenements
            .iter()
            .filter(|e| e.evenement == TypeEvenementTrade::Cloture)
            .collect();
        assert_eq!(clotures.len(), 1, "une clôture");
        assert_eq!(clotures[0].detail, format!("TP|{:.4}", t.r_brut.unwrap_or(0.0)));
        assert!((clotures[0].prix - t.prix_sortie.unwrap_or(0.0)).abs() < 1e-9);
        // Le signal (estimation) porte le close de la barre de signal N
        // (le scénario de test avance d'1 seconde par bougie).
        let idx_signal = b
            .iter()
            .position(|c| c.timestamp.timestamp() + 1 == t.ts_entree.timestamp())
            .expect("barre de signal présente");
        assert!((signaux[0].prix_entree - b[idx_signal].close).abs() < 1e-9);
    }
}
