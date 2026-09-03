//! Lifecycle des trades v12 : extraction des signaux jamais émis + diff
//! d'états (fill/BE/TP/clôture) entre évaluations successives.
//! Extrait de lib.rs — limite 600 lignes.

use super::*;
use smc::v12::trade::{CloseReason, Trade};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct EtatVu {
    rempli: bool,
    tp1_touche: bool,
    tp2_touche: bool,
    tp3_touche: bool,
    be_force: bool,
    sl_bits: u64,
    clture_raison: Option<u8>,
}

impl EtatVu {
    fn depuis(t: &Trade) -> Self {
        Self {
            rempli: t.state != TradeState::Pending,
            tp1_touche: t.tp1_price_touched,
            tp2_touche: t.tp2_ts > 0,
            tp3_touche: t.tp3_touched,
            be_force: t.be_forced,
            sl_bits: t.sl.to_bits(),
            clture_raison: t.close_reason.map(raison_discriminant),
        }
    }
}

/// Discriminant stable de la raison de clôture (sérialisation compacte).
pub(crate) fn raison_discriminant(r: CloseReason) -> u8 {
    match r {
        CloseReason::Sl => 1,
        CloseReason::Be => 2,
        CloseReason::Tp2Sl => 3,
        CloseReason::Tp3 => 4,
        CloseReason::Expire => 5,
        CloseReason::Cancel => 6,
        CloseReason::Ts => 7,
    }
}

/// Clé d'unicité d'un trade — STABLE entre redémarrages : `open_ts`
/// (horodatage Unix de la barre de création) plutôt que l'index de barre,
/// qui glisse quand la fenêtre de replay avance. La clôture d'un trade
/// ouvert avant un redémarrage retrouve ainsi sa ligne en base.
pub(crate) fn cle_du_trade(t: &Trade) -> CleTrade {
    (
        t.open_ts,
        match t.side {
            Side::Buy => 0,
            Side::Sell => 1,
        },
        match t.source {
            TradeSource::Ob => 0,
            TradeSource::BsZones => 1,
        },
        t.entry.to_bits(),
    )
}

/// Sérialisation canonique de la clé — PARTAGÉE par le signal (cle_moteur en
/// base) et les événements (cle_trade) : un seul format, jamais de Debug Rust
/// (incident 23/08 : `(2018, 0, 0, …)` vs `2018:0:0:…` → clôtures perdues).
pub(crate) fn cle_vers_string(cle: &CleTrade) -> String {
    format!("{}:{}:{}:{}", cle.0, cle.1, cle.2, cle.3)
}

/// Verdict canonique en base (TP1/TP2/TP3/SL/BE/Expire).
fn verdict_texte(v: smc::v12::trade::Verdict) -> &'static str {
    match v {
        smc::v12::trade::Verdict::Tp3 => "TP3",
        smc::v12::trade::Verdict::Ts => "TS",
        smc::v12::trade::Verdict::Tp2 => "TP2+BE",
        smc::v12::trade::Verdict::Tp1 => "TP1+BE",
        smc::v12::trade::Verdict::Sl => "SL",
        smc::v12::trade::Verdict::Be => "BE",
        smc::v12::trade::Verdict::Expire => "Expire",
    }
}

/// Construit le `SignalBrut` d'un trade (partagé annonce/confirmation).
fn signal_depuis_trade(t: &Trade, asset: Asset, tf: Timeframe, debut_barre: i64) -> SignalBrut {
    let direction = match t.side {
        Side::Buy => Direction::Long,
        Side::Sell => Direction::Short,
    };
    let source = match t.source {
        TradeSource::Ob => "v11-OB",
        TradeSource::BsZones => "BSZones",
    };
    let cle = cle_vers_string(&cle_du_trade(t));
    SignalBrut::avec_cle(
        NOM,
        asset,
        tf,
        direction,
        t.entry,
        t.sl,
        vec![t.tp1, t.tp2, t.tp3],
        t.score,
        format!(
            "{} {} score={} risk={:.4} bar={}",
            source,
            match t.side {
                Side::Buy => "BUY",
                Side::Sell => "SELL",
            },
            t.score,
            t.risk0,
            t.bar_created
        ),
        debut_barre,
        cle,
    )
}

/// Extrait les trades jamais émis d'un carnet, les convertit en signaux et
/// verrouille leurs clés. `annonces` (clés déjà annoncées intrabar) marque
/// `deja_annonce` — le writer saura ne pas re-messager.
pub(crate) fn extraire_nouveaux(
    emis: &mut HashSet<CleTrade>,
    annonces: &HashSet<CleTrade>,
    trades: &[Trade],
    asset: Asset,
    tf: Timeframe,
    debut_barre: i64,
) -> Vec<SignalBrut> {
    let mut signaux = Vec::new();
    // Clés présentes dans le carnet évalué — borne le cache PAR PRÉSENCE :
    // un trade vit potentiellement bien plus de 50 barres, et le carnet ne
    // fait que croître (`bar_created` strictement croissant) — une clé
    // disparue ne peut plus correspondre à un futur trade.
    let mut cles_presentes: HashSet<CleTrade> = HashSet::with_capacity(trades.len());
    for t in trades {
        let cle = cle_du_trade(t);
        cles_presentes.insert(cle);
        if emis.contains(&cle) {
            continue;
        }
        emis.insert(cle);
        let mut s = signal_depuis_trade(t, asset.clone(), tf, debut_barre);
        s.deja_annonce = annonces.contains(&cle);
        signaux.push(s);
    }
    emis.retain(|cle| cles_presentes.contains(cle));
    signaux
}

/// Annonces d'imminence INTRABAR : premier instant où l'évaluation live
/// (clone de la bougie en formation) crée le trade → message Telegram,
/// sans attendre la clôture (décision propriétaire 23/08 — la règle Pine
/// `barstate.isconfirmed` garde la création du trade et la ligne en base).
///
/// Anti-doublon : `annonces` n'est JAMAIS élagué — un trade qui disparaît
/// puis réapparaît dans la même bougie (prix oscillant au bord de zone)
/// n'est annoncé qu'UNE fois. Nouvelle bougie = nouvelle clé (open_ts) =
/// nouvelle annonce possible, comme l'alerte TV « une fois par barre ».
pub(crate) fn extraire_annonces(
    annonces: &mut HashSet<CleTrade>,
    emis: &HashSet<CleTrade>,
    trades_eval: &[Trade],
    asset: Asset,
    tf: Timeframe,
    debut_barre: i64,
) -> Vec<SignalBrut> {
    let mut signaux = Vec::new();
    for t in trades_eval {
        let cle = cle_du_trade(t);
        // Déjà confirmé (émis à la clôture) ou déjà annoncé → silence.
        if emis.contains(&cle) || annonces.contains(&cle) {
            continue;
        }
        annonces.insert(cle);
        let mut s = signal_depuis_trade(t, asset.clone(), tf, debut_barre);
        s.annonce = true;
        signaux.push(s);
    }
    signaux
}

/// Détecte les transitions lifecycle entre l'état vu précédemment et le
/// carnet évalué, les convertit en événements et met à jour l'état vu.
///
/// Un trade disparu du carnet (condition intrabar évanouie — rollback Pine)
/// est retiré silencieusement : son signal d'entrée déjà émis n'est JAMAIS
/// rétracté (R5), et aucun événement d'annulation n'est inventé.
pub(crate) fn diff_lifecycle(
    vus: &mut std::collections::HashMap<CleTrade, EtatVu>,
    trades: &[Trade],
    asset: Asset,
    tf: Timeframe,
    debut_barre: i64,
) -> Vec<EvenementTrade> {
    let mut evenements = Vec::new();
    let mut cles_presentes = HashSet::with_capacity(trades.len());

    for t in trades {
        let cle = cle_du_trade(t);
        cles_presentes.insert(cle);
        let nouvel_etat = EtatVu::depuis(t);
        let precedent = vus.insert(cle, nouvel_etat);

        // Baseline d'un trade fraîchement créé : rien touché, pas rempli.
        let vide = EtatVu {
            rempli: false,
            tp1_touche: false,
            tp2_touche: false,
            tp3_touche: false,
            be_force: false,
            sl_bits: t.sl.to_bits(),
            clture_raison: None,
        };
        let avant = precedent.unwrap_or(vide);

        let mut pousser = |type_ev: TypeEvenementTrade, detail: String, prix: f64| {
            evenements.push(EvenementTrade {
                moteur: NOM.to_string(),
                asset: asset.clone(),
                tf,
                cle_trade: cle_vers_string(&cle),
                evenement: type_ev,
                detail,
                prix,
                debut_barre,
                emis_le: chrono::Utc::now(),
            });
        };

        // Ordre naturel du cycle : remplissage → BE → TP1 → TP2 → TP3 → clôture.
        if !avant.rempli && nouvel_etat.rempli {
            pousser(TypeEvenementTrade::Fill, "retest touché".into(), t.entry);
        }
        if !avant.be_force && nouvel_etat.be_force {
            pousser(TypeEvenementTrade::Be, "BE forcé".into(), t.entry);
        } else if avant.sl_bits != nouvel_etat.sl_bits
            && nouvel_etat.rempli
            && !nouvel_etat.tp3_touche
        {
            // SL déplacé vers l'entrée (BE armé après TP1).
            pousser(TypeEvenementTrade::Be, "SL → entrée".into(), t.entry);
        }
        if !avant.tp1_touche && nouvel_etat.tp1_touche {
            pousser(TypeEvenementTrade::Tp1, "TP1 touché".into(), t.tp1);
        }
        if !avant.tp2_touche && nouvel_etat.tp2_touche {
            pousser(TypeEvenementTrade::Tp2, "TP2 touché".into(), t.tp2);
        }
        if !avant.tp3_touche && nouvel_etat.tp3_touche {
            pousser(TypeEvenementTrade::Tp3, "TP3 touché".into(), t.tp3);
        }
        if avant.clture_raison.is_none() && nouvel_etat.clture_raison.is_some() {
            if let Some(r) = t.close_reason {
                // Détail structuré « verdict|R » — le writer officiel sépare sur
                // '|' pour écrire le verdict et son R en base (courbe de trades).
                pousser(
                    TypeEvenementTrade::Cloture,
                    format!("{}|{:.4}", verdict_texte(t.verdict()), t.realized_r()),
                    match r {
                        CloseReason::Sl => t.sl,
                        // Sortie au stop suivi : prix réel du trailing.
                        CloseReason::Ts => t.ts_px.unwrap_or(t.tp2),
                        // Après TP2 le stop est remonté à TP1 : sortie à TP1.
                        CloseReason::Tp2Sl => t.tp1,
                        CloseReason::Be => t.entry,
                        CloseReason::Tp3 => t.tp3,
                        // Prix de sortie exact non porté par le trade : entrée.
                        CloseReason::Expire | CloseReason::Cancel => t.entry,
                    },
                );
            }
        }
    }

    // Trades disparus du carnet (phantoms intrabar) : retrait silencieux.
    vus.retain(|cle, _| cles_presentes.contains(cle));
    evenements
}

#[cfg(test)]
mod tests {
    use super::*;


    fn trade_sl(ts: i64) -> Trade {
        // Trade bull créé à ts, rempli puis clôturé SL (struct littérale :
        // `new_buy` est pub(crate) smc, inaccessible depuis engine_v12).
        let mut t = Trade {
            id: 1,
            side: Side::Buy,
            source: TradeSource::Ob,
            entry: 100.0,
            sl: 98.0,
            tp1: 102.0,
            tp2: 104.0,
            tp3: 106.0,
            score: 8,
            risk0: 2.0,
            open_ts: ts,
            bar_created: 42,
            ob_key: None,
            filled: true,
            tp1_hit: false,
            tp1_price_touched: false,
            tp2_ts: 0,
            tp3_touched: false,
            be_forced: false,
            mfe_armed: false,
            state: TradeState::Open,
            fill_ts: Some(ts),
            close_reason: Some(CloseReason::Sl),
            close_ts: None,
            close_bar: None,
            close_r: None,
        };
        t.state = TradeState::Closed;
        t
    }

    /// Un seul format de clé, sans parenthèse Debug : l'incident 23/08 venait
    /// d'un signal « (2018, 0, 0, …) » face à un événement « 2018:0:0:… ».
    #[test]
    fn cle_format_unique_sans_debug() {
        let t = trade_sl(1_700_000_000);
        let cle = cle_du_trade(&t);
        let s = cle_vers_string(&cle);
        assert!(!s.contains('('), "pas de format Debug : {}", s);
        assert!(!s.contains(' '), "pas d'espace : {}", s);
        // Le champ clé du signal = exactement la clé de l'événement.
        let mut emis = HashSet::new();
        let annonces = HashSet::new();
        let sig = extraire_nouveaux(
            &mut emis,
            &annonces,
            &[trade_sl(1_700_000_000)],
            Asset::from("XAUUSD"),
            Timeframe::M15,
            0,
        );
        assert_eq!(sig.len(), 1);
        assert_eq!(sig[0].cle, s, "clé signal ≠ clé canonique");
    }

    /// Oscillation intrabar : un setup qui disparaît puis réapparaît dans la
    /// même bougie n'est annoncé qu'UNE fois (l'incident ×9 ne reviendra pas).
    #[test]
    fn annonce_une_seule_fois_malgre_oscillation() {
        let t = trade_sl(1_700_000_000);
        let mut annonces = HashSet::new();
        let emis = HashSet::new();
        let a = Asset::from("XAUUSD");
        let tf = Timeframe::M15;
        // 1re évaluation : le trade existe → annonce.
        let s1 = extraire_annonces(&mut annonces, &emis, &[t.clone()], a.clone(), tf, 0);
        assert_eq!(s1.len(), 1);
        assert!(s1[0].annonce);
        assert!(!s1[0].deja_annonce);
        // 2e évaluation : le trade a disparu (prix éloigné) → rien.
        let s2 = extraire_annonces(&mut annonces, &emis, &[], a.clone(), tf, 30);
        assert!(s2.is_empty());
        // 3e évaluation : le trade réapparaît → PAS de nouvelle annonce.
        let s3 = extraire_annonces(&mut annonces, &emis, &[t.clone()], a, tf, 60);
        assert!(s3.is_empty(), "aucune ré-annonce pour la même clé");
    }

    /// Le signal confirmé porte deja_annonce quand l'imminence est partie.
    #[test]
    fn confirme_marque_deja_annonce() {
        let t = trade_sl(1_700_000_000);
        let mut annonces = HashSet::new();
        let mut emis = HashSet::new();
        let a = Asset::from("XAUUSD");
        let tf = Timeframe::M15;
        let _ = extraire_annonces(&mut annonces, &emis, &[t.clone()], a.clone(), tf, 0);
        let sig = extraire_nouveaux(&mut emis, &annonces, &[t], a, tf, 60);
        assert_eq!(sig.len(), 1);
        assert!(!sig[0].annonce);
        assert!(sig[0].deja_annonce, "l'annonce partie doit être marquée");
    }

    /// La clé repose sur open_ts : deux moteurs qui rejouent des fenêtres
    /// décalées retrouvent la MÊME clé pour le même trade (clôture d'un
    /// trade ouvert avant redémarrage).
    #[test]
    fn cle_stable_malgre_fenetre_decalee() {
        let t = trade_sl(1_700_000_000);
        let k1 = cle_vers_string(&cle_du_trade(&t));
        // Même trade, index de barre différent (fenêtre glissée) → même clé.
        let mut t2 = trade_sl(1_700_000_000);
        t2.bar_created = 9_999;
        let k2 = cle_vers_string(&cle_du_trade(&t2));
        assert_eq!(k1, k2);
        // Une seconde plus tard → clé différente.
        let mut t3 = trade_sl(1_700_000_001);
        t3.bar_created = t.bar_created;
        assert_ne!(k1, cle_vers_string(&cle_du_trade(&t3)));
    }

    /// Clôture : le détail porte « verdict|R » (ex. « SL|-1.0000 »).
    #[test]
    fn cloture_detail_porte_verdict_et_r() {
        let t = trade_sl(1_700_000_000);
        let mut vus = std::collections::HashMap::new();
        // 1re évaluation : trade ouvert rempli (pas encore clôturé).
        let mut ouvert = trade_sl(1_700_000_000);
        ouvert.close_reason = None;
        ouvert.state = TradeState::Open;
        let _ = diff_lifecycle(
            &mut vus,
            &[ouvert],
            Asset::from("XAUUSD"),
            Timeframe::M15,
            0,
        );
        // 2e évaluation : clôturé SL → un événement Cloture « SL|-1.0000 ».
        let evs = diff_lifecycle(
            &mut vus,
            &[t],
            Asset::from("XAUUSD"),
            Timeframe::M15,
            60,
        );
        let clotures: Vec<_> = evs
            .iter()
            .filter(|e| matches!(e.evenement, TypeEvenementTrade::Cloture))
            .collect();
        assert_eq!(clotures.len(), 1);
        assert_eq!(clotures[0].detail, "SL|-1.0000");
        // clé de l'événement = clé canonique du trade (open_ts|side|src|bits).
        assert_eq!(
            clotures[0].cle_trade,
            cle_vers_string(&cle_du_trade(&trade_sl(1_700_000_000)))
        );
    }
}
