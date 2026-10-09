//! Alertes d'approche des zones SMC — spec docs/spec_alertes_zones_smc.md.
//!
//! Le runtime voit chaque prix live ; les moteurs SMC ARMÉS exposent leurs
//! order blocks jamais touchés (Vierge) avec leur ATR. Ce watcher mesure,
//! à chaque prix, la distance au bord proche de la zone fraîche la plus
//! proche de chaque sens : ≤ seuil × ATR → une alerte (Telegram + face app),
//! une seule par zone et par épisode (ré-armement par hystérésis).
//!
//! Périmètre = couples ARMÉS SMC (précision propriétaire 09/10) : un couple
//! désarmé n'a pas de moteur dans le runtime → aucune zone, par construction.
//! Nuit 23h–7h (heure locale = Paris) : le watcher ne fait RIEN — aucun état,
//! aucune notification ; au réveil, seules les zones encore en bande alertent.
//! Purement mémoire (décision ⑤) : aucun état ne survit aux redémarrages.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use actix_web::{web, HttpResponse, Responder};
use chrono::Timelike;
use common::{Asset, Timeframe};
use engine::{Runtime, ZoneApproche};

use crate::state::AppState;

/// Plafond anti-flood : alertes notifiées par heure glissante.
const PLAFOND_HEURE: usize = 20;
/// Rétention de la face app « alertes récentes ».
const RETENTION_RECENTES_SEC: i64 = 2 * 3600;

// ── Configuration (clés `configuration`, rechargée au tick 60 s) ─────────────

#[derive(Debug, Clone)]
pub struct ConfigZones {
    pub actif: bool,
    pub seuil_atr: f64,
    pub hysteresis_atr: f64,
    pub nuit_debut: u32,
    pub nuit_fin: u32,
    pub telegram: bool,
}

impl Default for ConfigZones {
    fn default() -> Self {
        Self {
            actif: true,
            seuil_atr: 0.25,
            hysteresis_atr: 1.0,
            nuit_debut: 23,
            nuit_fin: 7,
            telegram: true,
        }
    }
}

/// Lit la config avec repli sur les défauts (clé absente ou illisible).
pub async fn lire_config(db: &db::Database) -> ConfigZones {
    let mut c = ConfigZones::default();
    if let Ok(Some(v)) = db.lire_config("smc_alertes_zones_actif").await {
        c.actif = v != "0";
    }
    if let Ok(Some(v)) = db.lire_config("smc_alertes_zones_seuil_atr").await {
        if let Ok(f) = v.parse::<f64>() {
            if f > 0.0 {
                c.seuil_atr = f;
            }
        }
    }
    if let Ok(Some(v)) = db.lire_config("smc_alertes_zones_hysteresis_atr").await {
        if let Ok(f) = v.parse::<f64>() {
            if f >= 0.0 {
                c.hysteresis_atr = f;
            }
        }
    }
    if let Ok(Some(v)) = db.lire_config("smc_alertes_zones_nuit_debut").await {
        if let Ok(h) = v.parse::<u32>() {
            if h < 24 {
                c.nuit_debut = h;
            }
        }
    }
    if let Ok(Some(v)) = db.lire_config("smc_alertes_zones_nuit_fin").await {
        if let Ok(h) = v.parse::<u32>() {
            if h < 24 {
                c.nuit_fin = h;
            }
        }
    }
    if let Ok(Some(v)) = db.lire_config("smc_alertes_zones_telegram").await {
        c.telegram = v != "0";
    }
    c
}

// ── État mémoire ─────────────────────────────────────────────────────────────

/// Identifiant stable d'une zone pour l'épisode d'approche.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct CleZone {
    asset: String,
    tf: String,
    achat: bool,
    ts_zone: i64,
}

/// Une zone vue par la face app (ligne de la carte Scanner).
#[derive(Debug, Clone, serde::Serialize)]
pub struct ZoneWatch {
    pub asset: String,
    pub tf: String,
    pub achat: bool,
    /// Bornes affichées [haut ; bas] de la zone.
    pub zone_haut: f64,
    pub zone_bas: f64,
    /// Distance au bord proche en multiples d'ATR (> 0 : pas encore touchée).
    pub distance_atr: f64,
    /// Distance au bord proche en % du prix.
    pub distance_pct: f64,
    pub prix: f64,
    pub ts_zone: i64,
    /// Présente si un épisode d'approche est déjà notifié pour la zone.
    pub alerte_le: Option<i64>,
}

#[derive(Default)]
pub struct EtatZones {
    /// Zones déjà notifiées pour l'épisode d'approche EN COURS.
    alertees: HashMap<CleZone, i64>,
    /// Zones actuellement en bande (rafraîchies à chaque prix du couple).
    en_bande: Vec<ZoneWatch>,
    /// Alertes notifiées (rétention 2 h).
    recentes: Vec<ZoneWatch>,
    /// Horodatages des notifications (fenêtre glissante 1 h) — anti-flood.
    fenetre: Vec<i64>,
}

fn etat() -> &'static Mutex<EtatZones> {
    static ETAT: OnceLock<Mutex<EtatZones>> = OnceLock::new();
    ETAT.get_or_init(|| Mutex::new(EtatZones::default()))
}

// ── Cœur pur (testé isolément) ───────────────────────────────────────────────

/// Silence nocturne ? Bornes heure locale : début INCLUS, fin EXCLUE
/// (23 → 7 : silence de 23h00 à 6h59).
pub(crate) fn etat_nuit(heure: u32, debut: u32, fin: u32) -> bool {
    if debut == fin {
        return false;
    }
    if debut < fin {
        heure >= debut && heure < fin
    } else {
        heure >= debut || heure < fin
    }
}

/// Distance signée au bord proche : > 0 = approche possible, ≤ 0 = touché.
fn distance(prix: f64, z: &ZoneApproche) -> f64 {
    if z.achat {
        prix - z.bord_proche
    } else {
        z.bord_proche - prix
    }
}

/// La zone fraîche ÉLIGIBLE la plus proche du prix, par sens — None si
/// aucune zone du sens n'est devant le prix.
fn plus_proche<'a>(zones: &'a [ZoneApproche], prix: f64, achat: bool) -> Option<&'a ZoneApproche> {
    zones
        .iter()
        .filter(|z| z.achat == achat && distance(prix, z) > 0.0)
        .min_by(|a, b| distance(prix, a).total_cmp(&distance(prix, b)))
}

/// Met à jour les épisodes d'approche du couple et retourne (zones en
/// bande, zones à NOTIFIER maintenant). Règles : une zone éligible par
/// couple × sens (la plus proche) ; une notification par zone et par
/// épisode ; ré-armement quand la distance repasse seuil + hystérésis ;
/// purge des clés dont la zone a disparu (touchée, invalidée, évincée).
pub(crate) fn maj_episode(
    zones: &[ZoneApproche],
    prix: f64,
    seuil_atr: f64,
    hysteresis_atr: f64,
    couple: (String, String),
    alertees: &mut HashMap<CleZone, i64>,
    maintenant: i64,
) -> (Vec<ZoneWatch>, Vec<ZoneWatch>) {
    // 1. Purge : clés du couple dont la zone n'existe plus OU redevenue
    //    lointaine au-delà de l'hystérésis (ré-armement).
    alertees.retain(|cle, _| {
        if cle.asset != couple.0 || cle.tf != couple.1 {
            return true;
        }
        match zones.iter().find(|z| z.achat == cle.achat && z.ts_zone == cle.ts_zone) {
            None => false, // zone morte (touchée/invalidée/FIFO) → purge
            Some(z) => distance(prix, z) <= (seuil_atr + hysteresis_atr) * z.atr,
        }
    });

    // 2. Bande d'approche : la plus proche de chaque sens, si ≤ seuil × ATR.
    let mut en_bande = Vec::new();
    let mut a_alerter = Vec::new();
    for achat in [true, false] {
        let Some(z) = plus_proche(zones, prix, achat) else { continue };
        let d = distance(prix, z);
        let seuil = seuil_atr * z.atr;
        if d > seuil {
            continue;
        }
        let cle = CleZone {
            asset: couple.0.clone(),
            tf: couple.1.clone(),
            achat: z.achat,
            ts_zone: z.ts_zone,
        };
        let w = ZoneWatch {
            asset: couple.0.clone(),
            tf: couple.1.clone(),
            achat: z.achat,
            zone_haut: z.bord_proche.max(z.bord_lointain),
            zone_bas: z.bord_proche.min(z.bord_lointain),
            distance_atr: if z.atr > 0.0 { d / z.atr } else { f64::INFINITY },
            distance_pct: if prix > 0.0 { d / prix * 100.0 } else { 0.0 },
            prix,
            ts_zone: z.ts_zone,
            alerte_le: None,
        };
        if alertees.contains_key(&cle) {
            en_bande.push(w); // déjà notifiée pour cet épisode
        } else {
            alertees.insert(cle, maintenant);
            let mut w_notif = w.clone();
            w_notif.alerte_le = Some(maintenant);
            a_alerter.push(w_notif.clone());
            en_bande.push(w_notif);
        }
    }
    (en_bande, a_alerter)
}

// ── Watcher (appelé à chaque prix live du couple) ────────────────────────────

/// Vérifie l'approche des zones ARMÉES du couple. Ne fait RIEN la nuit
/// (aucun état, aucune notification) ni si le watch est désactivé.
pub async fn verifier(
    db: &db::Database,
    runtime: &Runtime,
    cfg: &ConfigZones,
    asset: &Asset,
    tf: Timeframe,
    prix: f64,
) {
    if !cfg.actif {
        return;
    }
    if etat_nuit(chrono::Local::now().hour(), cfg.nuit_debut, cfg.nuit_fin) {
        return;
    }
    let zones = runtime.zones_approche(asset, tf);
    let couple = (asset.as_str().to_string(), tf.as_str().to_string());
    let maintenant = chrono::Utc::now().timestamp();
    let a_alerter = {
        let mut e = match etat().lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        let (en_bande, a_alerter) = maj_episode(
            &zones,
            prix,
            cfg.seuil_atr,
            cfg.hysteresis_atr,
            couple.clone(),
            &mut e.alertees,
            maintenant,
        );
        // Face app : remplacer les lignes du couple par l'état courant.
        e.en_bande.retain(|w| w.asset != couple.0 || w.tf != couple.1);
        e.en_bande.extend(en_bande);
        // Anti-flood : élaguer la fenêtre puis compter.
        e.fenetre.retain(|t| maintenant - *t < 3600);
        a_alerter
            .into_iter()
            .filter(|w| {
                let ok = e.fenetre.len() < PLAFOND_HEURE;
                if ok {
                    e.fenetre.push(maintenant);
                    e.recentes.push(w.clone());
                } else {
                    tracing::warn!(
                        asset = %w.asset,
                        "approche zone non notifiée (plafond {PLAFOND_HEURE}/h atteint)"
                    );
                }
                ok
            })
            .collect::<Vec<ZoneWatch>>()
    };

    for w in a_alerter {
        let sens: &str = if w.achat { "ACHAT" } else { "VENTE" };
        let msg = format!(
            "📍 Approche zone {sens}\n{} {} · [{:.2} ; {:.2}] · à {:.2} × ATR",
            w.asset, w.tf, w.zone_haut, w.zone_bas, w.distance_atr
        );
        tracing::info!(
            "📍 Approche zone {} {} [{:.2};{:.2}] prix {:.2} ({:.2} × ATR)",
            w.asset, w.tf, w.zone_haut, w.zone_bas, w.prix, w.distance_atr
        );
        if cfg.telegram {
            let (token, chat) = notifications::telegram::lire_tokens_pool(db.pool()).await;
            if !token.is_empty() && !chat.is_empty() {
                if let Err(e) = notifications::telegram::post_message(&token, &chat, &msg).await {
                    tracing::warn!("Approche zone (Telegram) : {e}");
                }
            }
        }
    }
}

// ── Face app ─────────────────────────────────────────────────────────────────

/// GET /api/smc/zones-approche — zones en bande (live) + alertes récentes.
pub async fn get_zones_approche(_state: web::Data<AppState>) -> impl Responder {
    let maintenant = chrono::Utc::now().timestamp();
    let mut e = match etat().lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    e.recentes.retain(|w| maintenant - w.alerte_le.unwrap_or(maintenant) < RETENTION_RECENTES_SEC);
    HttpResponse::Ok().json(serde_json::json!({
        "en_bande": e.en_bande,
        "recentes": e.recentes,
        "plafond_heure": PLAFOND_HEURE,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn zone(achat: bool, proche: f64, lointain: f64, ts: i64, atr: f64) -> ZoneApproche {
        ZoneApproche { achat, bord_proche: proche, bord_lointain: lointain, ts_zone: ts, atr }
    }

    #[test]
    fn nuit_bornes_23_7() {
        assert!(!etat_nuit(22, 23, 7), "22h59 : encore jour");
        assert!(etat_nuit(23, 23, 7), "23h00 inclus");
        assert!(etat_nuit(3, 23, 7));
        assert!(etat_nuit(6, 23, 7));
        assert!(!etat_nuit(7, 23, 7), "7h00 exclu");
        assert!(!etat_nuit(12, 23, 7));
        // Fenêtre sans chevauchement de minuit.
        assert!(etat_nuit(10, 9, 12));
        assert!(!etat_nuit(13, 9, 12));
    }

    #[test]
    fn approche_une_seule_alerte_par_episode_puis_rearmee() {
        let zones = vec![zone(true, 100.0, 98.0, 1000, 2.0)]; // achat, top=100
        let mut alertees = HashMap::new();
        let couple = ("XAUUSD".into(), "M5".into());

        // Prix à 100,3 → distance 0,3 = 0,15 × ATR (≤ 0,25) : ALERTE.
        let (bande, alerter) = maj_episode(&zones, 100.3, 0.25, 1.0, couple.clone(), &mut alertees, 1);
        assert_eq!(alerter.len(), 1);
        assert_eq!(bande.len(), 1);
        assert!(alerter[0].achat);

        // Même position : déjà alertée → rien.
        let (_, alerter) = maj_episode(&zones, 100.31, 0.25, 1.0, couple.clone(), &mut alertees, 2);
        assert!(alerter.is_empty());

        // Retrait à 100,4 (0,2 × ATR ≤ seuil) : toujours en bande, pas de nouvelle alerte.
        let (bande, alerter) = maj_episode(&zones, 100.4, 0.25, 1.0, couple.clone(), &mut alertees, 3);
        assert!(alerter.is_empty());
        assert_eq!(bande.len(), 1);

        // Retrait au-delà du seuil + hystérésis (100 + 0,25×2 + 1×2 = 102,5) : ré-armé.
        maj_episode(&zones, 102.6, 0.25, 1.0, couple.clone(), &mut alertees, 4);
        assert!(alertees.is_empty(), "clé ré-armée au-delà de l'hystérésis");

        // Retour dans la bande : NOUVELLE alerte (nouvel épisode).
        let (_, alerter) = maj_episode(&zones, 100.2, 0.25, 1.0, couple.clone(), &mut alertees, 5);
        assert_eq!(alerter.len(), 1);
    }

    #[test]
    fn zone_morte_purge_et_dans_la_zone_pas_dalerte() {
        let zones = vec![zone(true, 100.0, 98.0, 1000, 2.0)];
        let mut alertees = HashMap::new();
        let couple = ("BTC".into(), "M15".into());
        let (_, alerter) = maj_episode(&zones, 100.2, 0.25, 1.0, couple.clone(), &mut alertees, 1);
        assert_eq!(alerter.len(), 1);

        // Prix DANS la zone (99,5 < top) : pas d'alerte d'approche (trop tard)…
        let (bande, alerter) = maj_episode(&zones, 99.5, 0.25, 1.0, couple.clone(), &mut alertees, 2);
        assert!(alerter.is_empty());
        assert!(bande.is_empty(), "la zone traversée n'est pas en bande d'approche");

        // …et la clé survit tant que le prix reste dans la zone (ré-armable
        // s'il s'éloigne de l'autre côté sans que la zone meure).
        assert_eq!(alertees.len(), 1);

        // Zone touchée à la clôture → absente de la liste fraîche → purge.
        maj_episode(&[], 100.0, 0.25, 1.0, couple, &mut alertees, 3);
        assert!(alertees.is_empty());
    }

    #[test]
    fn seule_la_zone_la_plus_proche_par_sens_est_eligible() {
        let zones = vec![
            zone(true, 100.0, 98.0, 1000, 2.0), // proche
            zone(true, 90.0, 88.0, 2000, 2.0),  // lointaine
        ];
        let mut alertees = HashMap::new();
        let (_, alerter) = maj_episode(&zones, 100.2, 0.25, 1.0, ("DAX".into(), "M30".into()), &mut alertees, 1);
        assert_eq!(alerter.len(), 1, "une seule zone d'achat alertée");
        assert_eq!(alerter[0].ts_zone, 1000, "la plus proche");
    }

    #[test]
    fn couple_desarme_sans_moteur_ne_declenche_rien() {
        // Précision propriétaire : périmètre = couples ARMÉS. Un couple sans
        // moteur n'expose aucune zone (Runtime::zones_approche → vide) :
        // maj_episode sur une liste vide ne peut rien alerter et purge les
        // clés restantes d'un armement antérieur.
        let mut alertees = HashMap::new();
        alertees.insert(
            CleZone { asset: "EURUSD".into(), tf: "M5".into(), achat: true, ts_zone: 42 },
            1,
        );
        maj_episode(&[], 1.10, 0.25, 1.0, ("EURUSD".into(), "M5".into()), &mut alertees, 2);
        assert!(alertees.is_empty());
    }

    #[test]
    fn zone_vente_approche_par_le_bas() {
        let zones = vec![zone(false, 110.0, 112.0, 3000, 2.0)]; // vente, bot=110
        let mut alertees = HashMap::new();
        let (bande, alerter) = maj_episode(&zones, 109.7, 0.25, 1.0, ("NAS100".into(), "M5".into()), &mut alertees, 1);
        assert_eq!(alerter.len(), 1);
        assert!(!alerter[0].achat);
        assert_eq!(bande[0].zone_haut, 112.0);
        assert_eq!(bande[0].zone_bas, 110.0);
    }
}
