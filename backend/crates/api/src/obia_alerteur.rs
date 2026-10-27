//! OBIA P2 étape 1 — alerteur espion maison (GO owner 26/10).
//!
//! Spec P1 (règles mesurées 20-23/10, gravées au cockpit) : déclencheur =
//! CLÔTURE d'une bougie H4 ou Daily dont le cours a touché une zone active,
//! même légèrement (l'effleurement d'une bougie NON clôturée reste muet) ;
//! prix annoncé = le BORD de la zone ; zone touchée puis TRAVERSÉE =
//! retirée sans sonner (US100 22/10) ; H1 exclu (bornes portail instables,
//! périmètre espion) ; les côtés achat ET vente alertent.
//!
//! Les zones sont celles du panneau OrderBlock IA v6, lues par l'espion
//! EA_Espion_OBIA dans son journal (UTF-16) : 4 objets par zone
//! (`OBIA10_Z_{S|B}{ts}_B` cadre, `_F` FVG, `_OB` Order Block, `_T`
//! étiquette au médian). L'alerteur consomme le DERNIER snapshot du
//! journal (zones vivantes à l'instant) et l'évalue contre les bougies
//! H4/D1 CLÔTURÉES en base (Axi). Idempotent par (zone, TF, bougie) en
//! table obia_alertes — un redémarrage ne re-sonne jamais.

use std::sync::Arc;

use common::{Asset, Candle, Timeframe};

/// Journal de l'espion OBIA (même machine que l'API).
const CHEMIN_JOURNAL: &str =
    "/home/rono/.mt5/drive_c/Program Files/MetaTrader 5/MQL5/Files/espion_obia_journal.jsonl";

/// Symbole de l'étape 1 (le périmètre mesuré de la spec — le portail ne
/// couvre que l'or ; l'extension à d'autres charts viendra avec l'étape 2).
const SYMBOLE_CIBLE: &str = "XAUUSD";

/// Famille de canaux (matrice alertes_canaux).
const FAMILLE: &str = "obia_contact";

/// Cadence du cycle (les bougies H4/D1 clôturent rarement — 30 s suffit).
const CADENCE_SEC: u64 = 30;

// ── Zone ─────────────────────────────────────────────────────────────────────

/// Une zone active du panneau — bornes du FVG (boîte `_F`).
#[derive(Debug, Clone, PartialEq)]
pub struct ZoneObia {
    pub id: String,
    /// true = zone de demande (achat, ▲) ; false = offre (vente, ▼).
    pub cote_achat: bool,
    pub bas: f64,
    pub haut: f64,
    /// Début (epoch s) de la boîte — naissance de la zone.
    pub debut: i64,
}

/// Décision d'évaluation d'une bougie clôturée contre une zone.
#[derive(Debug, Clone, PartialEq)]
pub enum Decision {
    /// Bougie antérieure à la zone, ou aucun contact.
    Rien,
    /// Contact clôturé : prix annoncé = le bord touché.
    Alerte { bord: f64 },
    /// Clôture au-delà du bord opposé : la zone est consommée, muette.
    Retiree,
}

/// Règle P1 pure : contact de la bougie (mèche comprise — « même
/// légèrement ») SONNÉ à la clôture ; traversée complète = muette.
pub fn evaluer(zone: &ZoneObia, bougie: &Candle) -> Decision {
    if bougie.timestamp.timestamp() < zone.debut {
        return Decision::Rien; // la zone n'existait pas à l'ouverture de la bougie
    }
    let touche = bougie.high >= zone.bas && bougie.low <= zone.haut;
    if !touche {
        return Decision::Rien;
    }
    let traversee = if zone.cote_achat {
        bougie.close < zone.bas
    } else {
        bougie.close > zone.haut
    };
    if traversee {
        return Decision::Retiree;
    }
    // Bord touché : le prix approche par le bord proche de la zone
    // (demande touchée par le haut, offre par le bas).
    Decision::Alerte {
        bord: if zone.cote_achat { zone.haut } else { zone.bas },
    }
}

// ── Lecture du journal de l'espion ───────────────────────────────────────────

/// Décode le journal UTF-16 (BOM LE) de l'espion en texte.
fn decoder_utf16(bytes: &[u8]) -> Option<String> {
    if bytes.len() < 2 || bytes[0] != 0xFF || bytes[1] != 0xFE {
        return None;
    }
    let mots: Vec<u16> = bytes[2..]
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    Some(String::from_utf16_lossy(&mots))
}

/// Zones vivantes du dernier snapshot du journal pour `symbole` — `None`
/// quand l'espion est détaché (marqueur `fin`) : session close, on ne
/// suit plus rien. `Some(vide)` = espion actif, aucune zone dessinée.
pub fn lire_zones(texte: &str, symbole_cible: &str) -> Option<Vec<ZoneObia>> {
    let mut snapshot: Option<serde_json::Value> = None;
    for ligne in texte.lines().rev() {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(ligne) else {
            continue;
        };
        if v.get("fin").and_then(|f| f.as_bool()) == Some(true) {
            return None; // session close après ce point : espion détaché
        }
        if !v.get("objets").and_then(|o| o.as_array()).map_or(false, |a| !a.is_empty()) {
            continue;
        }
        snapshot = Some(v);
        break; // dernier snapshot porteur
    }
    let Some(snap) = snapshot else {
        return Some(Vec::new());
    };
    // chart id → symbole (l'espion balaye plusieurs graphiques).
    let mut symboles: std::collections::HashMap<i64, String> = std::collections::HashMap::new();
    if let Some(graphs) = snap.get("graphiques").and_then(|g| g.as_array()) {
        for g in graphs {
            let id = g.get("id").and_then(|i| i.as_i64());
            let s = g.get("symbole").and_then(|s| s.as_str());
            if let (Some(id), Some(s)) = (id, s) {
                symboles.insert(id, s.to_string());
            }
        }
    }
    let mut zones = Vec::new();
    if let Some(objets) = snap.get("objets").and_then(|o| o.as_array()) {
        for o in objets {
            let nom = o.get("nom").and_then(|n| n.as_str()).unwrap_or_default();
            let Some(reste) = nom.strip_prefix("OBIA10_Z_") else {
                continue;
            };
            let Some(id) = reste.strip_suffix("_F") else {
                continue; // seule la boîte FVG définit les bornes de la zone
            };
            let cote_achat = id.starts_with('B');
            if !cote_achat && !id.starts_with('S') {
                continue;
            }
            let chart = o.get("chart").and_then(|c| c.as_i64());
            let sur_cible = chart
                .and_then(|c| symboles.get(&c))
                .map_or(false, |s| s == symbole_cible);
            if !sur_cible {
                continue;
            }
            let Some(ancres) = o.get("ancres").and_then(|a| a.as_array()) else {
                continue;
            };
            let mut bas = f64::MAX;
            let mut haut = f64::MIN;
            let mut debut = i64::MAX;
            let mut valide = false;
            for a in ancres {
                if let (Some(p), Some(t)) = (
                    a.get("prix").and_then(|p| p.as_f64()),
                    a.get("ts").and_then(|t| t.as_i64()),
                ) {
                    bas = bas.min(p);
                    haut = haut.max(p);
                    debut = debut.min(t);
                    valide = true;
                }
            }
            if valide && haut > bas {
                zones.push(ZoneObia {
                    id: id.to_string(),
                    cote_achat,
                    bas,
                    haut,
                    debut,
                });
            }
        }
    }
    Some(zones)
}

// ── Cycle ────────────────────────────────────────────────────────────────────

/// Bougie du TF en formation à `maintenant`, sur la grille ancrée par la
/// dernière clôturée (broker-safe, même astuce que le compositeur).
fn ts_formante(derniere_ts: i64, duree: i64, maintenant: i64) -> i64 {
    let n = ((maintenant - derniere_ts).max(0) / duree).max(1);
    derniere_ts + n * duree
}

/// Mémoire de l'alerteur entre les cycles. L'INDICATEUR retire une zone à
/// l'effleurement INTRABAR (mitigation FVG) : au cycle suivant elle a
/// disparu du journal et, sans mémoire, la clôture qui suit ne pourrait
/// plus l'évaluer — cas du 26/10 18h (mèche 3988 sous la zone 4002-4007,
/// clôture 4020 : contact clôturé à sonner, zone déjà retirée du chart).
/// Chaque disparue attend la clôture de la bougie qui était FORMANT à
/// l'instant de sa disparition, puis est évaluée par la même règle.
#[derive(Default)]
struct EtatAlerteur {
    vues: std::collections::HashMap<String, ZoneObia>,
    /// (zone, tf, ts de la bougie à attendre).
    pendantes: Vec<(ZoneObia, &'static str, i64)>,
}

/// Boucle périodique : zones du journal × dernières bougies H4/D1 clôturées,
/// idempotent par (zone, TF, bougie). Une zone « Retiree » est consommée à
/// vie (filtre en amont du cycle).
pub async fn boucle(db: Arc<db::Database>) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(CADENCE_SEC));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let asset = Asset::from(SYMBOLE_CIBLE);
    let mut etat = EtatAlerteur::default();
    loop {
        interval.tick().await;
        if let Err(e) = un_cycle(&db, &asset, &mut etat).await {
            tracing::warn!("OBIA alerteur : {e}");
        }
    }
}

async fn un_cycle(db: &Arc<db::Database>, asset: &Asset, etat: &mut EtatAlerteur) -> anyhow::Result<()> {
    let bytes = match tokio::fs::read(CHEMIN_JOURNAL).await {
        Ok(b) => b,
        Err(_) => return Ok(()), // espion jamais lancé : alerteur dormant
    };
    let Some(texte) = decoder_utf16(&bytes) else {
        return Ok(()); // pas un journal UTF-16 : rien à faire
    };
    let maintenant = chrono::Utc::now().timestamp();
    let Some(zones) = lire_zones(&texte, SYMBOLE_CIBLE) else {
        // Espion détaché : session close — on ne suit plus rien.
        etat.vues.clear();
        etat.pendantes.clear();
        return Ok(());
    };

    // Zones déjà consommées (traversées) à vie.
    let mortes: std::collections::HashSet<String> =
        sqlx::query_scalar("SELECT DISTINCT zone_id FROM obia_alertes WHERE action = 'retiree'")
            .fetch_all(db.pool())
            .await
            .unwrap_or_default()
            .into_iter()
            .collect();

    // Dernière clôturée par TF (ancre de grille pour les formantes).
    let mut ancres: Vec<(&'static str, Timeframe, i64, Vec<common::Candle>)> = Vec::new();
    for (tf_nom, tf) in [("H4", Timeframe::H4), ("D1", Timeframe::D1)] {
        let bougies = db.obtenir_bougies(asset, &tf, 3).await.unwrap_or_default();
        if let Some(derniere) = bougies.last() {
            ancres.push((tf_nom, tf, derniere.timestamp.timestamp(), bougies));
        }
    }

    // ① Disparitions : zones vues au cycle précédent absentes du snapshot —
    // on planifie l'évaluation de la bougie formante de CHAQUE TF.
    let vivantes: std::collections::HashSet<&str> = zones.iter().map(|z| z.id.as_str()).collect();
    let disparues: Vec<ZoneObia> = etat
        .vues
        .values()
        .filter(|z| !vivantes.contains(z.id.as_str()) && !mortes.contains(&z.id))
        .cloned()
        .collect();
    for zone in &disparues {
        for (tf_nom, _, derniere_ts, _) in &ancres {
            let duree = tf_nom_duree(tf_nom);
            let attendue = ts_formante(*derniere_ts, duree, maintenant);
            let deja_planifiee = etat
                .pendantes
                .iter()
                .any(|(z, t, ts)| z.id == zone.id && t == tf_nom && *ts == attendue);
            if !deja_planifiee {
                tracing::info!(
                    "🎯 OBIA : zone {} disparue du chart — évaluation à la clôture {} (bougie {})",
                    zone.id,
                    tf_nom,
                    attendue
                );
                etat.pendantes.push((zone.clone(), tf_nom, attendue));
            }
        }
    }
    etat.vues = zones.iter().map(|z| (z.id.clone(), z.clone())).collect();

    // ② Zones vivantes × bougies clôturées (règle directe), idempotent.
    for zone in &zones {
        if mortes.contains(&zone.id) {
            continue;
        }
        for (tf_nom, _, _, bougies) in &ancres {
            let duree = tf_nom_duree(tf_nom);
            for bougie in bougies
                .iter()
                .filter(|b| b.timestamp.timestamp() + duree <= maintenant)
            {
                evaluer_une(db, zone, tf_nom, bougie, maintenant).await;
            }
        }
    }

    // ③ Pendantes : chaque bougie attendue clôturée → même règle d'évaluation.
    let mut restantes: Vec<(ZoneObia, &'static str, i64)> = Vec::new();
    for (zone, tf_nom, attendue) in etat.pendantes.drain(..) {
        let duree = tf_nom_duree(tf_nom);
        if maintenant < attendue + duree {
            restantes.push((zone, tf_nom, attendue));
            continue;
        }
        if let Some((_, _, _, bougies)) = ancres.iter().find(|a| a.0 == tf_nom) {
            if let Some(bougie) = bougies.iter().find(|b| b.timestamp.timestamp() == attendue) {
                evaluer_une(db, &zone, tf_nom, bougie, maintenant).await;
            }
            // introuvable en fenêtre : abandon silencieux (disparition sans
            // contact — rechargement d'indicateur, changement de graphique…)
        }
    }
    etat.pendantes = restantes;
    Ok(())
}

fn tf_nom_duree(tf_nom: &str) -> i64 {
    if tf_nom == "D1" {
        86_400
    } else {
        14_400
    }
}

/// Évalue une bougie clôturée contre une zone (dedupe DB, insertion,
/// émission) — chemin commun des zones vivantes et des pendantes.
async fn evaluer_une(
    db: &Arc<db::Database>,
    zone: &ZoneObia,
    tf_nom: &str,
    bougie: &Candle,
    maintenant: i64,
) {
    let ts = bougie.timestamp.timestamp();
    let deja: Option<i64> = sqlx::query_scalar(
        "SELECT 1 FROM obia_alertes WHERE zone_id = ? AND tf = ? AND candle_ts = ?",
    )
    .bind(&zone.id)
    .bind(tf_nom)
    .bind(ts)
    .fetch_optional(db.pool())
    .await
    .ok()
    .flatten();
    if deja.is_some() {
        return;
    }
    let decision = evaluer(zone, bougie);
    let action = match decision {
        Decision::Rien => return,
        Decision::Retiree => "retiree",
        Decision::Alerte { .. } => "alerte",
    };
    let _ = sqlx::query(
        "INSERT OR IGNORE INTO obia_alertes (zone_id, tf, candle_ts, action, emis_le)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&zone.id)
    .bind(tf_nom)
    .bind(ts)
    .bind(action)
    .bind(maintenant)
    .execute(db.pool())
    .await;
    if let Decision::Alerte { bord } = decision {
        emettre_alerte(db, zone, tf_nom, bord, ts).await;
    } else {
        tracing::info!(
            "🎯 OBIA : zone {} {} traversée — retirée sans sonner",
            zone.id,
            tf_nom
        );
    }
}

async fn emettre_alerte(db: &Arc<db::Database>, zone: &ZoneObia, tf: &str, bord: f64, ts: i64) {
    let cote = if zone.cote_achat { "▲ achat" } else { "▼ vente" };
    let titre = format!("🎯 OBIA {SYMBOLE_CIBLE} {tf} — zone {cote} touchée (bord {bord:.2})");
    let _ = crate::mobile_relay::emettre(
        db,
        FAMILLE,
        &titre,
        Some(SYMBOLE_CIBLE),
        Some(tf),
        serde_json::json!({
            "zone": zone.id, "cote": cote, "bord": bord,
            "bas": zone.bas, "haut": zone.haut, "bougie_ts": ts,
        }),
        false,
    )
    .await;
    crate::alertes_canaux::telegram_si_actif(
        db.pool(),
        FAMILLE,
        &format!("{titre}\nzone {} — {} / {}", zone.id, zone.bas, zone.haut),
    )
    .await;
    tracing::info!("🎯 OBIA : {titre}");
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    fn bougie(ts: i64, o: f64, h: f64, l: f64, c: f64) -> Candle {
        Candle {
            timestamp: Utc.timestamp_opt(ts, 0).single().unwrap_or_default(),
            open: o, high: h, low: l, close: c, volume: 0.0,
        }
    }

    fn zone_vente() -> ZoneObia {
        // Cas réel du journal (XAU, offre ▼) : FVG 4009.08–4016.15.
        ZoneObia {
            id: "S1792773900".into(),
            cote_achat: false,
            bas: 4009.08,
            haut: 4016.15,
            debut: 1_792_773_900,
        }
    }

    /// Contact clôturé sonné (XAU 22h00 23/10) : mèche dans la zone, close
    /// en deçà du bord proche → ALERTE au bord BAS de la zone d'offre.
    #[test]
    fn contact_cloture_sonne_au_bord() {
        let z = zone_vente();
        let b = bougie(1_792_810_000, 4002.0, 4012.0, 4001.0, 4005.0);
        assert_eq!(
            evaluer(&z, &b),
            Decision::Alerte { bord: 4009.08 }
        );
    }

    /// « Même légèrement » : l'effleurement par la mèche suffit (spéc P1).
    #[test]
    fn effleurement_de_meme_suffit() {
        let z = zone_vente();
        let b = bougie(1_792_810_000, 4000.0, 4009.50, 3999.0, 4001.0);
        assert!(matches!(evaluer(&z, &b), Decision::Alerte { .. }));
    }

    /// Traversée complète = retirée SANS sonner (US100 22/10).
    #[test]
    fn traversee_muet_retraite_la_zone() {
        let z = zone_vente();
        let b = bougie(1_792_810_000, 4010.0, 4025.0, 4005.0, 4020.0);
        assert_eq!(evaluer(&z, &b), Decision::Retiree);
    }

    /// Bougie antérieure à la naissance de la zone : jamais sonnée.
    #[test]
    fn bougie_anterieure_ignoree() {
        let z = zone_vente();
        let b = bougie(1_792_700_000, 4012.0, 4015.0, 4010.0, 4014.0);
        assert_eq!(evaluer(&z, &b), Decision::Rien);
    }

    /// Zone de demande (achat ▲) : le bord touché est le HAUT.
    #[test]
    fn zone_demande_alerte_au_bord_haut() {
        let z = ZoneObia {
            id: "B1".into(),
            cote_achat: true,
            bas: 3980.0,
            haut: 3985.0,
            debut: 1_000,
        };
        let b = bougie(2_000, 3990.0, 3992.0, 3982.0, 3988.0);
        assert_eq!(evaluer(&z, &b), Decision::Alerte { bord: 3985.0 });
        // traversée baissière : close SOUS le bas → muette.
        let b2 = bougie(2_000, 3981.0, 3988.0, 3975.0, 3977.0);
        assert_eq!(evaluer(&z, &b2), Decision::Retiree);
    }

    /// Parseur : snapshot réel du journal → la zone XAU extraite, la zone
    /// d'un autre graphique exclue ; marqueur `fin` → session close (None).
    #[test]
    fn parse_journal_snapshot_reel() {
        let ligne = r#"{"ts":1792936029,"heure":"2026.10.25 13:47:09","symbole":"XAUUSD","tf":"H1","seq":1,"plein":true,"graphiques":[{"id":-100,"symbole":"XAUUSD","tf":"H1"},{"id":-200,"symbole":"NAS100.fs","tf":"H1"}],"objets":[{"chart":-100,"nom":"OBIA10_Z_S1792773900_F","type":20,"ancres":[{"ts":1792773900,"prix":4016.15},{"ts":1792817100,"prix":4009.08}],"texte":""},{"chart":-200,"nom":"OBIA10_Z_S1792641600_F","type":20,"ancres":[{"ts":1792641600,"prix":29727.5},{"ts":1792868400,"prix":29690.38}],"texte":""},{"chart":-100,"nom":"KasperPanelBg","type":110,"ancres":[],"texte":""}]}"#;
        let zones = lire_zones(ligne, "XAUUSD").unwrap_or_default();
        assert_eq!(zones.len(), 1, "seule la zone XAUUSD est retenue");
        let z = &zones[0];
        assert_eq!(z.id, "S1792773900");
        assert!(!z.cote_achat);
        assert!((z.bas - 4009.08).abs() < 1e-9);
        assert!((z.haut - 4016.15).abs() < 1e-9);
        assert_eq!(z.debut, 1_792_773_900);

        // Espion détaché : le marqueur fin FERME la session (None ≠ vide).
        let avec_fin = format!("{ligne}\n{{\"ts\":1,\"fin\":true,\"seq\":2}}");
        assert!(lire_zones(&avec_fin, "XAUUSD").is_none());

        // Espion actif sans snapshot porteur : session ouverte, zéro zone.
        assert!(lire_zones("{\"ts\":1,\"seq\":3,\"plein\":false}", "XAUUSD")
            .map_or(false, |v| v.is_empty()));
    }

    /// Grille des bougies formantes, ancrée sur la dernière clôturée —
    /// broker-safe (jamais d'ancre UTC brute). Une H4 ts L clôture à L+D :
    /// 30 min après sa clôture, la formante est L+D ; à l'instant exact de
    /// clôture, la nouvelle s'ouvre (L+D) ; à la clôture suivante, L+2D.
    #[test]
    fn ts_formante_ancree_sur_la_derniere_cloturee() {
        let h4 = 14_400i64;
        let derniere = 1_793_014_800i64; // dernière H4 CLÔTURÉE (fermée à +D)
        assert_eq!(ts_formante(derniere, h4, derniere + h4 + 1_800), derniere + h4);
        assert_eq!(ts_formante(derniere, h4, derniere + h4), derniere + h4);
        assert_eq!(ts_formante(derniere, h4, derniere + 2 * h4 + 60), derniere + 2 * h4);
    }

    /// Décodeur UTF-16 : BOM LE + « é » (données réelles du journal).
    #[test]
    fn decode_utf16_bom_le() {
        let texte = "{\"heure\":\"13:47:09\"}";
        let mut bytes = vec![0xFF, 0xFE];
        for u in texte.encode_utf16() {
            bytes.extend_from_slice(&u.to_le_bytes());
        }
        assert_eq!(decoder_utf16(&bytes).as_deref(), Some(texte));
        assert!(decoder_utf16(b"pas du utf16").is_none());
    }
}
