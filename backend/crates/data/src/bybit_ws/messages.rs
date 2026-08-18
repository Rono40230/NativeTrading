//! Parsing des messages WS Bybit v5 (klines, pong) — module extrait de
//! `bybit_ws` pour la limite de 600 lignes du pré-audit. Fonctions pures,
//! testables sans DB ni réseau.

use std::collections::HashMap;

use common::{Asset, Timeframe};

use super::bybit_interval_vers_tf;

/// Extrait `(interval, symbol)` d'un topic `kline.{interval}.{symbol}`.
pub(super) fn extraire_topic(topic: &str) -> Option<(&str, &str)> {
    let reste = topic.strip_prefix("kline.")?;
    let (interval, symbol) = reste.split_once('.')?;
    if interval.is_empty() || symbol.is_empty() {
        return None;
    }
    Some((interval, symbol))
}

// ─────────────────────────────────────────────────────────────────────────────
// Parsing des messages WS
// ─────────────────────────────────────────────────────────────────────────────

/// Action à exécuter après parsing d'un message texte WS.
#[derive(Debug)]
pub(super) enum ActionWs {
    /// Rien à faire (message ignoré : ack souscription, etc.).
    Ignorer,
    /// Bybit a envoyé un ping applicatif → répondre `{"op":"pong"}`.
    Pong,
    /// Klines parsées — confirmées ET non confirmées. Les non confirmées
    /// alimentent le runtime tick (évaluation intrabar), les confirmées
    /// sont en outre insérées en DB.
    Klines(Vec<KlineWs>),
}

/// Kline parsée d'un message Bybit — confirmée ou non.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct KlineWs {
    pub(super) asset: Asset,
    pub(super) tf: Timeframe,
    /// Début de la bougie (epoch sec — déjà aligné, fourni par Bybit).
    pub(super) debut: i64,
    pub(super) open: f64,
    pub(super) high: f64,
    pub(super) low: f64,
    pub(super) close: f64,
    pub(super) volume: f64,
    pub(super) confirmee: bool,
}

/// Parse un message texte brut Bybit en `ActionWs`. `mapping` (symbol → asset
/// DB) est construit à partir de la DB au début de chaque session — il
/// remplace l'ancien mapping hardcodé `bybit_vers_asset()`. Fonction pure →
/// testable sans DB ni réseau.
pub(super) fn traiter_texte(message: &str, mapping: &HashMap<String, String>) -> ActionWs {
    let valeur: serde_json::Value = match serde_json::from_str(message) {
        Ok(v) => v,
        Err(e) => {
            // Message non-JSON (rare) : on ignore sans crasher.
            tracing::debug!(
                "Bybit WS: message non-JSON ignoré ({}): {:?}",
                e,
                message.get(..80).unwrap_or(message)
            );
            return ActionWs::Ignorer;
        }
    };

    // Ping applicatif Bybit → on devra répondre pong.
    if valeur.get("op").and_then(|o| o.as_str()) == Some("ping") {
        return ActionWs::Pong;
    }

    // Autres messages `op` (pong, ack subscribe). On logge seulement les échecs.
    if valeur.get("op").is_some() {
        if valeur.get("success").and_then(|s| s.as_bool()) == Some(false) {
            let msg = valeur
                .get("ret_msg")
                .and_then(|m| m.as_str())
                .unwrap_or("raison inconnue");
            tracing::warn!("Bybit WS: opération échouée: {}", msg);
        }
        return ActionWs::Ignorer;
    }

    // Message kline.
    let Some(topic) = valeur.get("topic").and_then(|t| t.as_str()) else {
        return ActionWs::Ignorer;
    };
    let Some((interval, symbol)) = extraire_topic(topic) else {
        return ActionWs::Ignorer;
    };
    let Some(asset_str) = mapping.get(symbol).map(|s| s.as_str()) else {
        return ActionWs::Ignorer; // symbole non suivi
    };
    let Some(tf_str) = bybit_interval_vers_tf(interval) else {
        return ActionWs::Ignorer; // interval non suivi
    };
    // La légitimité d'un asset vient de la table `assets` (mapping symbole →
    // id lu en DB au début de session) — plus de liste codée : tout ticker
    // ajouté est accepté tel quel.
    let asset = Asset::from(asset_str);
    let Ok(tf) = Timeframe::try_from(tf_str) else {
        return ActionWs::Ignorer;
    };

    let Some(blocs) = valeur.get("data").and_then(|d| d.as_array()) else {
        return ActionWs::Ignorer;
    };

    let mut klines = Vec::with_capacity(blocs.len());
    for bloc in blocs {
        // Toutes les klines sont parsées : les non confirmées alimentent le
        // runtime tick (bougie en formation), les confirmées ferment la
        // bougie avec les valeurs officielles (et vont en DB).
        let confirmee = bloc.get("confirm").and_then(|c| c.as_bool()) == Some(true);
        let Some(start) = bloc.get("start").and_then(|s| s.as_i64()) else {
            continue;
        };
        // Bybit WS kline v5 : `start` peut être en secondes (10 chiffres) OU
        // millisecondes (13 chiffres) selon la version du topic. On détecte.
        let start_sec = if start > 1_000_000_000_000 { start / 1000 } else { start };
        let champ_f64 = |cle: &str| bloc.get(cle).and_then(|x| x.as_str()).and_then(|s| s.parse::<f64>().ok());
        let Some(open) = champ_f64("open") else {
            continue;
        };
        let Some(high) = champ_f64("high") else {
            continue;
        };
        let Some(low) = champ_f64("low") else {
            continue;
        };
        let Some(close) = champ_f64("close") else {
            continue;
        };
        let volume = champ_f64("volume").unwrap_or(0.0);
        klines.push(KlineWs {
            asset: asset.clone(),
            tf,
            debut: start_sec,
            open,
            high,
            low,
            close,
            volume,
            confirmee,
        });
    }

    if klines.is_empty() {
        ActionWs::Ignorer
    } else {
        ActionWs::Klines(klines)
    }
}
