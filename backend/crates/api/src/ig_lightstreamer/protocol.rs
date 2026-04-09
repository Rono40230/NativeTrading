//! Fonctions TLCP Lightstreamer IG — protocole HTTP standalone.
//! Toutes les fonctions ici sont pures (pas de self) et testables isolément.

use anyhow::{anyhow, Result};
use common::Timeframe;

// ─── Résolutions Lightstreamer IG ─────────────────────────────────────────────

pub(super) fn resolution_ls(tf: &Timeframe) -> &'static str {
    match tf {
        Timeframe::M1  => "1MINUTE",
        Timeframe::M5  => "5MINUTE",
        Timeframe::M15 => "15MINUTE",
        Timeframe::M30 => "30MINUTE",
        Timeframe::H1  => "HOUR",
        Timeframe::H4  => "4HOUR",
        Timeframe::D1  => "DAY",
        Timeframe::W1  => "WEEK",
    }
}

// ─── Schéma des champs LS ─────────────────────────────────────────────────────
// Index dans l'ordre déclaré lors du subscribe :
// 0=OFR_OPEN 1=OFR_HIGH 2=OFR_LOW 3=OFR_CLOSE
// 4=BID_OPEN 5=BID_HIGH 6=BID_LOW 7=BID_CLOSE
// 8=CONS_END 9=UTM

pub(super) const LS_SCHEMA: &str =
    "OFR_OPEN OFR_HIGH OFR_LOW OFR_CLOSE BID_OPEN BID_HIGH BID_LOW BID_CLOSE CONS_END UTM";

pub(super) const IDX_OFR_OPEN:  usize = 0;
pub(super) const IDX_OFR_HIGH:  usize = 1;
pub(super) const IDX_OFR_LOW:   usize = 2;
pub(super) const IDX_OFR_CLOSE: usize = 3;
pub(super) const IDX_BID_OPEN:  usize = 4;
pub(super) const IDX_BID_HIGH:  usize = 5;
pub(super) const IDX_BID_LOW:   usize = 6;
pub(super) const IDX_BID_CLOSE: usize = 7;
pub(super) const IDX_CONS_END:  usize = 8;
pub(super) const IDX_UTM:       usize = 9;

// ─── create_session TLCP ──────────────────────────────────────────────────────

pub(super) async fn create_session(endpoint: &str, account_id: &str, cst: &str) -> Result<String> {
    let url = format!("{}/lightstreamer/create_session.txt", endpoint);

    let body = format!(
        "LS_op2=create&LS_cid=mgA3YGZlZG9kbA%3D%3D&LS_adapter_set=DEFAULT\
         &LS_user={account}&LS_password=CST-{cst}\
         &LS_polling=false&LS_polling_millis=0&LS_idle_millis=0\
         &LS_report_info=false",
        account = urlencoding_simple(account_id),
        cst = cst,
    );

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()?;

    let resp = client
        .post(&url)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await?
        .text()
        .await?;

    // Réponse : "CONOK,<session_id>,<keep_alive>,<max_bandwidth>,*\r\n"
    for line in resp.lines() {
        if line.starts_with("CONOK") {
            let parts: Vec<&str> = line.splitn(5, ',').collect();
            if parts.len() >= 2 {
                return Ok(parts[1].to_string());
            }
        }
        if line.starts_with("CONERR") || line.starts_with("END") {
            return Err(anyhow!("LS create_session refusé: {}", line));
        }
    }

    Err(anyhow!("LS create_session: réponse inattendue: {}", &resp[..resp.len().min(200)]))
}

// ─── send_subscribe ───────────────────────────────────────────────────────────

pub(super) async fn send_subscribe(
    endpoint: &str,
    session_id: &str,
    epic: &str,
    resolution: &str,
    sub_id: usize,
) -> Result<()> {
    let url = format!("{}/lightstreamer/control.txt", endpoint);
    let item = format!("CHART:{}:{}:1", epic, resolution);

    let body = format!(
        "LS_session={session}&LS_op=add&LS_subId={sub_id}\
         &LS_mode=MERGE&LS_group={item}&LS_schema={schema}\
         &LS_data_adapter=CHART&LS_snapshot=true",
        session = session_id,
        sub_id  = sub_id,
        item    = urlencoding_simple(&item),
        schema  = urlencoding_simple(LS_SCHEMA),
    );

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    client
        .post(&url)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await?;

    Ok(())
}

// ─── send_unsubscribe ─────────────────────────────────────────────────────────

pub(super) async fn send_unsubscribe(endpoint: &str, session_id: &str, sub_id: usize) -> Result<()> {
    let url = format!("{}/lightstreamer/control.txt", endpoint);
    let body = format!(
        "LS_session={session}&LS_op=delete&LS_subId={sub_id}",
        session = session_id,
        sub_id  = sub_id,
    );
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;
    client
        .post(&url)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await?;
    Ok(())
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Calcule le mid price entre bid et ask.
pub(super) fn mid(bid: Option<&Option<f64>>, ask: Option<&Option<f64>>) -> Option<f64> {
    match (bid.and_then(|v| *v), ask.and_then(|v| *v)) {
        (Some(b), Some(a)) => Some((b + a) / 2.0),
        (Some(b), None)    => Some(b),
        (None, Some(a))    => Some(a),
        _                  => None,
    }
}

/// Encodage URL minimal pour les paramètres TLCP.
pub(super) fn urlencoding_simple(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            ' '  => out.push_str("%20"),
            ':'  => out.push_str("%3A"),
            '|'  => out.push_str("%7C"),
            '&'  => out.push_str("%26"),
            '='  => out.push_str("%3D"),
            '+'  => out.push_str("%2B"),
            _    => out.push(c),
        }
    }
    out
}
