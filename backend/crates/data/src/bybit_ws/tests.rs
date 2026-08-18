use std::collections::HashMap;
use std::sync::atomic::Ordering;

use common::{Asset, Timeframe};

use super::messages::{traiter_texte, ActionWs, extraire_topic, KlineWs};
use super::{
    bybit_interval_vers_tf, construire_mapping, construire_topics, marquer_demarre,
    tf_vers_bybit_interval, BYBIT_WS_DEMARRE,
};

/// Mapping de test équivalent au pré-remplissage de la migration 0064.
fn mapping_test() -> HashMap<String, String> {
    construire_mapping(&[
        ("BTCUSDT".to_string(), "BTC".to_string()),
        ("ETHUSDT".to_string(), "ETH".to_string()),
        ("XAUUSDT".to_string(), "XAUUSD".to_string()),
        ("XAGUSDT".to_string(), "XAGUSD".to_string()),
        ("DOGEUSDT".to_string(), "DOGE".to_string()),
    ])
}

#[test]
fn construire_topics_dynamiques() {
    let assets = vec![
        ("BTCUSDT".to_string(), "BTC".to_string()),
        ("XAUUSDT".to_string(), "XAUUSD".to_string()),
    ];
    let tfs = vec![Timeframe::M15, Timeframe::D1];
    let topics = construire_topics(&assets, &tfs);
    // 2 actifs × 2 timeframes, intervals Bybit corrects.
    assert_eq!(topics.len(), 4);
    assert!(topics.contains(&"kline.15.BTCUSDT".to_string()));
    assert!(topics.contains(&"kline.D.XAUUSDT".to_string()));
    // Liste vide → aucun topic.
    assert!(construire_topics(&[], &tfs).is_empty());
    assert!(construire_topics(&assets, &[]).is_empty());
}

#[test]
fn mapping_dynamique_symbole_vers_asset() {
    let mapping = mapping_test();
    assert_eq!(mapping.get("BTCUSDT").map(|s| s.as_str()), Some("BTC"));
    assert_eq!(mapping.get("XAUUSDT").map(|s| s.as_str()), Some("XAUUSD"));
    assert!(!mapping.contains_key("EURUSDT")); // non suivi
}

#[test]
fn mapping_intervals_bybit_vers_db() {
    assert_eq!(bybit_interval_vers_tf("1"), Some("M1"));
    assert_eq!(bybit_interval_vers_tf("5"), Some("M5"));
    assert_eq!(bybit_interval_vers_tf("15"), Some("M15"));
    assert_eq!(bybit_interval_vers_tf("30"), Some("M30"));
    assert_eq!(bybit_interval_vers_tf("60"), Some("H1"));
    assert_eq!(bybit_interval_vers_tf("240"), Some("H4"));
    assert_eq!(bybit_interval_vers_tf("D"), Some("D1"));
    assert_eq!(bybit_interval_vers_tf("W"), Some("W1"));
    assert_eq!(bybit_interval_vers_tf("120"), None); // H2 non couvert
    // Bijection interval ↔ timeframe.
    for tf in [
        Timeframe::M1, Timeframe::M5, Timeframe::M15, Timeframe::M30,
        Timeframe::H1, Timeframe::H4, Timeframe::D1, Timeframe::W1,
    ] {
        let interval = tf_vers_bybit_interval(&tf).expect("interval connu");
        assert_eq!(bybit_interval_vers_tf(interval), Some(tf.as_str()));
    }
}

#[test]
fn extraction_topic() {
    assert_eq!(extraire_topic("kline.15.XAUUSDT"), Some(("15", "XAUUSDT")));
    assert_eq!(extraire_topic("kline.D.BTCUSDT"), Some(("D", "BTCUSDT")));
    assert_eq!(extraire_topic("kline.1.DOGEUSDT"), Some(("1", "DOGEUSDT")));
    // Topics non-kline ou malformés.
    assert_eq!(extraire_topic("tickers.BTCUSDT"), None);
    assert_eq!(extraire_topic("kline."), None);
    assert_eq!(extraire_topic("kline.15."), None);
    assert_eq!(extraire_topic("autrechose"), None);
}

#[test]
fn parsing_message_kline_confirmee() {
    // Exemple tiré de la spec Bybit (champ confirm: true).
    let message = r#"{
        "topic": "kline.15.XAUUSDT",
        "type": "snapshot",
        "data": [{
            "start": 1786521600,
            "end": 1786522500,
            "interval": "15",
            "open": "4409.66",
            "high": "4414.0",
            "low": "4409.45",
            "close": "4412.28",
            "volume": "114.787",
            "turnover": "505540.3",
            "confirm": true,
            "timestamp": 1786521700000
        }]
    }"#;
    match traiter_texte(message, &mapping_test()) {
        ActionWs::Klines(klines) => {
            assert_eq!(klines.len(), 1, "une kline attendue");
            let k = &klines[0];
            assert_eq!(k.asset, Asset::from("XAUUSD"));
            assert_eq!(k.tf, Timeframe::M15);
            assert_eq!(k.debut, 1786521600);
            assert!(k.confirmee, "kline confirmée attendue");
            assert!((k.open - 4409.66).abs() < 1e-6);
            assert!((k.high - 4414.0).abs() < 1e-6);
            assert!((k.low - 4409.45).abs() < 1e-6);
            assert!((k.close - 4412.28).abs() < 1e-6);
            assert!((k.volume - 114.787).abs() < 1e-6);
        }
        autre => panic!("attendu ActionWs::Klines, obtenu {:?}", autre),
    }
}

#[test]
fn parsing_message_kline_non_confirmee_transmise() {
    // Bougie en cours (confirm: false) : ignorée de la DB mais transmise
    // au runtime tick (évaluation intrabar).
    let message = r#"{
        "topic": "kline.1.BTCUSDT",
        "type": "delta",
        "data": [{
            "start": 100,
            "interval": "1",
            "open": "1.0",
            "high": "2.0",
            "low": "0.5",
            "close": "1.5",
            "volume": "10.0",
            "confirm": false
        }]
    }"#;
    match traiter_texte(message, &mapping_test()) {
        ActionWs::Klines(klines) => {
            assert_eq!(klines.len(), 1, "la kline non confirmée doit être transmise");
            let k = &klines[0];
            assert_eq!(k.asset, Asset::from("BTC"));
            assert_eq!(k.tf, Timeframe::M1);
            assert!(!k.confirmee, "kline non confirmée attendue");
            assert_eq!(k.debut, 100);
        }
        autre => panic!("attendu ActionWs::Klines, obtenu {:?}", autre),
    }
}

#[test]
fn parsing_melange_confirmees_et_non_confirmees() {
    let message = r#"{
        "topic": "kline.5.ETHUSDT",
        "data": [
            {"start": 1000, "interval": "5", "open": "10", "high": "11", "low": "9", "close": "10.5", "volume": "5", "confirm": true},
            {"start": 2000, "interval": "5", "open": "20", "high": "21", "low": "19", "close": "20.5", "volume": "6", "confirm": false}
        ]
    }"#;
    match traiter_texte(message, &mapping_test()) {
        ActionWs::Klines(klines) => {
            assert_eq!(klines.len(), 2, "les deux klines sont parsées");
            assert!(klines[0].confirmee);
            assert!(!klines[1].confirmee);
            assert_eq!(klines[0].asset, Asset::from("ETH"));
            assert_eq!(klines[0].tf, Timeframe::M5);
        }
        autre => panic!("attendu ActionWs::Klines, obtenu {:?}", autre),
    }
}

#[test]
fn parsing_ping_applicatif_renvoie_pong() {
    assert!(matches!(
        traiter_texte(r#"{"op":"ping"}"#, &mapping_test()),
        ActionWs::Pong
    ));
}

#[test]
fn parsing_pong_et_ack_subscribe_ignores() {
    assert!(matches!(
        traiter_texte(r#"{"op":"pong"}"#, &mapping_test()),
        ActionWs::Ignorer
    ));
    assert!(matches!(
        traiter_texte(r#"{"op":"subscribe","success":true}"#, &mapping_test()),
        ActionWs::Ignorer
    ));
}

#[test]
fn parsing_message_non_json_ignore() {
    assert!(matches!(
        traiter_texte("not json {{", &mapping_test()),
        ActionWs::Ignorer
    ));
    assert!(matches!(traiter_texte("", &mapping_test()), ActionWs::Ignorer));
}

#[test]
fn parsing_symbole_non_suivi_ignore() {
    // Topic bien formé mais symbole absent du mapping DB.
    let message = r#"{
        "topic": "kline.15.EURUSDT",
        "data": [{"start": 1, "interval": "15", "open": "1", "high": "1", "low": "1", "close": "1", "volume": "1", "confirm": true}]
    }"#;
    assert!(matches!(
        traiter_texte(message, &mapping_test()),
        ActionWs::Ignorer
    ));
}

#[test]
fn parsing_nouvel_asset_accepte() {
    // Asset ajouté à l'exécution (aucune liste codée) : le ticker DB fait
    // foi — la kline est parsée et routée, sans recompilation.
    let mut mapping = HashMap::new();
    mapping.insert("NEWUSDT".to_string(), "NEWCOIN".to_string());
    let message = r#"{
        "topic": "kline.15.NEWUSDT",
        "data": [{"start": 1, "interval": "15", "open": "1", "high": "1", "low": "1", "close": "1", "volume": "1", "confirm": true}]
    }"#;
    match traiter_texte(message, &mapping) {
        ActionWs::Klines(klines) => {
            assert_eq!(klines.len(), 1);
            assert_eq!(klines[0].asset.as_str(), "NEWCOIN");
            assert!(klines[0].confirmee);
        }
        autre => panic!("attendu Klines, obtenu {:?}", autre),
    }
}

#[test]
fn garde_anti_double_start() {
    // On manipule directement la statique pour ce test ; on la remet dans
    // son état initial ensuite afin de ne pas polluer les autres tests.
    let avant = BYBIT_WS_DEMARRE
        .compare_exchange(false, false, Ordering::SeqCst, Ordering::SeqCst)
        .is_ok();
    // Premier « démarrage ».
    let premier = marquer_demarre();
    // Second appel doit être ignoré.
    let second = marquer_demarre();
    // Restauration.
    BYBIT_WS_DEMARRE.store(false, Ordering::SeqCst);
    // `avant` vaut true si la statique était bien à false au départ.
    assert!(avant, "la garde devait être à false au départ du test");
    assert!(premier, "le premier marquage doit renvoyer true");
    assert!(!second, "le second marquage doit renvoyer false");
}
