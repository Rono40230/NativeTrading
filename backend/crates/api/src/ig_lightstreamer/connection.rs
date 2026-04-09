//! Gestion de la connexion Lightstreamer : session, bind et parsing TLCP.

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use common::{Asset, Candle, Timeframe};
use db::Database;
use futures_util::StreamExt;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};

use crate::ig_session::IgSession;
use super::{LsCandle, SubKey};
use super::protocol;

// ─── Session Lightstreamer active ─────────────────────────────────────────────

pub(super) struct LsSession {
    pub(super) session_id: String,
    pub(super) endpoint:   String,
}

// ─── Connexion + abonnements + bind ──────────────────────────────────────────

pub(super) async fn connect_and_bind(
    ig_session: &Arc<Mutex<IgSession>>,
    db: &Arc<Database>,
    subs: &Arc<Mutex<HashMap<SubKey, usize>>>,
    ls_session: &Arc<Mutex<Option<LsSession>>>,
    tx: &broadcast::Sender<LsCandle>,
) -> Result<()> {
    // 1. S'assurer que la session REST IG est active
    let (ls_endpoint, cst, account_id) = {
        let mut sess = ig_session.lock().await;
        if !sess.est_connecte() {
            sess.login(db).await?;
        }
        let endpoint = sess
            .lightstreamer_endpoint
            .clone()
            .ok_or_else(|| anyhow!("lightstreamerEndpoint absent de la réponse IG"))?;
        let cst = sess
            .cst()
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow!("CST absent"))?;
        let account_id = sess.account_id.clone().unwrap_or_default();
        (endpoint, cst, account_id)
    };

    // 2. Ouvrir une session Lightstreamer
    let session_id = protocol::create_session(&ls_endpoint, &account_id, &cst).await?;
    tracing::info!(
        "IG Lightstreamer: session créée ({})",
        &session_id[..12.min(session_id.len())]
    );

    {
        let mut ls = ls_session.lock().await;
        *ls = Some(LsSession {
            session_id: session_id.clone(),
            endpoint:   ls_endpoint.clone(),
        });
    }

    // 3. Réabonner toutes les souscriptions connues
    let subs_snap: Vec<(SubKey, usize)> = {
        subs.lock().await.iter().map(|(k, v)| (k.clone(), *v)).collect()
    };
    for (key, sub_id) in &subs_snap {
        protocol::send_subscribe(&ls_endpoint, &session_id, &key.epic, &key.resolution, *sub_id)
            .await
            .unwrap_or_else(|e| tracing::warn!("Réabonnement échoué: {}", e));
    }

    // 4. Boucle bind infinie
    bind_loop(&ls_endpoint, &session_id, subs, tx, db).await
}

// ─── Boucle bind_session (stream HTTP infini) ─────────────────────────────────

async fn bind_loop(
    endpoint: &str,
    session_id: &str,
    subs: &Arc<Mutex<HashMap<SubKey, usize>>>,
    tx: &broadcast::Sender<LsCandle>,
    db: &Arc<Database>,
) -> Result<()> {
    let url  = format!("{}/lightstreamer/bind_session.txt", endpoint);
    let body = format!("LS_session={}", session_id);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(0)) // stream infini
        .build()?;

    let resp = client
        .post(&url)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await?;

    if !resp.status().is_success() {
        return Err(anyhow!("LS bind_session HTTP {}", resp.status()));
    }

    let mut field_state: HashMap<usize, Vec<Option<f64>>> = HashMap::new();
    let mut stream = resp.bytes_stream();
    let mut buffer = String::new();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        buffer.push_str(&String::from_utf8_lossy(&chunk));

        while let Some(pos) = buffer.find('\n') {
            let line = buffer[..pos].trim_end_matches('\r').to_string();
            buffer.drain(..=pos);
            handle_line(&line, &mut field_state, subs, tx, db).await;
        }
    }

    Ok(())
}

// ─── Parsing d'une ligne TLCP ─────────────────────────────────────────────────

async fn handle_line(
    line: &str,
    field_state: &mut HashMap<usize, Vec<Option<f64>>>,
    subs: &Arc<Mutex<HashMap<SubKey, usize>>>,
    tx: &broadcast::Sender<LsCandle>,
    db: &Arc<Database>,
) {
    if line.is_empty() || line.starts_with("PROBE") || line.starts_with("LOOP") {
        return;
    }

    // Format mise à jour : "<sub_id>,<item_pos>,<f1>|<f2>|...|<fN>"
    let parts: Vec<&str> = line.splitn(3, ',').collect();
    if parts.len() < 3 {
        return;
    }

    let sub_id: usize = match parts[0].parse() {
        Ok(v) => v,
        Err(_) => return,
    };

    // Mettre à jour l'état MERGE pour ce sub_id (valeurs partielles possibles)
    let state = field_state.entry(sub_id).or_insert_with(|| vec![None; 10]);
    for (i, val) in parts[2].split('|').enumerate() {
        if i >= state.len() { break; }
        if !val.is_empty() {
            state[i] = val.parse::<f64>().ok();
        }
    }

    // Calculer les mid prices bid/ask
    let open  = protocol::mid(state.get(protocol::IDX_BID_OPEN),  state.get(protocol::IDX_OFR_OPEN));
    let high  = protocol::mid(state.get(protocol::IDX_BID_HIGH),  state.get(protocol::IDX_OFR_HIGH));
    let low   = protocol::mid(state.get(protocol::IDX_BID_LOW),   state.get(protocol::IDX_OFR_LOW));
    let close = protocol::mid(state.get(protocol::IDX_BID_CLOSE), state.get(protocol::IDX_OFR_CLOSE));
    let utm   = state.get(protocol::IDX_UTM).and_then(|v| *v);
    let closed = state
        .get(protocol::IDX_CONS_END)
        .and_then(|v| *v)
        .map(|v| v == 1.0)
        .unwrap_or(false);

    // Attendre toutes les valeurs essentielles avant d'émettre
    let (open, high, low, close, utm) = match (open, high, low, close, utm) {
        (Some(o), Some(h), Some(l), Some(c), Some(u)) => (o, h, l, c, u),
        _ => return,
    };

    let timestamp = DateTime::<Utc>::from_timestamp_millis(utm as i64)
        .unwrap_or_else(Utc::now);

    // Retrouver asset + timeframe depuis le sub_id
    let (asset, timeframe) = {
        let subs = subs.lock().await;
        match subs.iter().find(|(_, &v)| v == sub_id) {
            Some((k, _)) => (k.asset.clone(), k.timeframe),
            None => return,
        }
    };

    let candle    = Candle { timestamp, open, high, low, close, volume: 0.0 };
    let ls_candle = LsCandle { asset: asset.clone(), timeframe, candle: candle.clone(), closed };

    let _ = tx.send(ls_candle);

    // Persister en DB uniquement à la clôture de la bougie
    if closed {
        let db = db.clone();
        let tf = timeframe;
        tokio::spawn(async move {
            if let Err(e) = db.inserer_bougies(&asset, &tf, &[candle]).await {
                tracing::warn!("LS: erreur insert DB: {}", e);
            }
        });
    }
}
