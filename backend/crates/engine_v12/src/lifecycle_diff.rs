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
    }
}

pub(crate) fn raison_texte(r: CloseReason) -> &'static str {
    match r {
        CloseReason::Sl => "Sl",
        CloseReason::Be => "Be",
        CloseReason::Tp2Sl => "Tp2Sl",
        CloseReason::Tp3 => "Tp3",
        CloseReason::Expire => "Expire",
        CloseReason::Cancel => "Cancel",
    }
}

pub(crate) fn cle_du_trade(t: &Trade) -> CleTrade {
    (
        t.bar_created,
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

/// Extrait les trades jamais émis d'un carnet, les convertit en signaux et
/// verrouille leurs clés. Fonction libre : `emis` (mutable) et `trades`
/// (immutable) appartiennent à des champs distincts de l'appelant.
pub(crate) fn extraire_nouveaux(
    emis: &mut HashSet<CleTrade>,
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
        let direction = match t.side {
            Side::Buy => Direction::Long,
            Side::Sell => Direction::Short,
        };
        let source = match t.source {
            TradeSource::Ob => "v11-OB",
            TradeSource::BsZones => "BSZones",
        };
        signaux.push(SignalBrut::nouveau(
            NOM,
            asset.clone(),
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
        ));
    }
    emis.retain(|cle| cles_presentes.contains(cle));
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
                cle_trade: format!("{}:{}:{}:{}", cle.0, cle.1, cle.2, cle.3),
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
                pousser(
                    TypeEvenementTrade::Cloture,
                    raison_texte(r).to_string(),
                    match r {
                        CloseReason::Sl => t.sl,
                        CloseReason::Be | CloseReason::Tp2Sl => t.entry,
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
