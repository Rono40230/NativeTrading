//! Scoring, classification et déduplication des articles de presse.
//! Séparé de news_handlers.rs pour respecter la limite de 300 lignes.
use chrono::Utc;
use serde::Serialize;

// ── Types de sortie ──────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct ArticleNews {
    pub id: String,
    pub titre: String,
    pub titre_fr: Option<String>,
    pub source: String,
    pub url: String,
    pub date: String,
    pub score: u8,
    pub niveau: &'static str,
    pub theme: &'static str,       // "macro" | "crypto" | "metaux" | "autre"
    pub sentiment: Option<String>, // "haussier" | "neutre" | "baissier" | None
}

#[derive(Serialize)]
pub struct AlertesNews {
    pub articles: Vec<ArticleNews>,
    pub score_max: u8,
    pub mis_a_jour: String,
}

// ── Mots-clés avec leur poids ────────────────────────────────────────────────

const MOTS_MACRO: &[(&str, u8)] = &[
    ("federal reserve", 15),
    ("rate hike", 15),
    ("rate cut", 15),
    ("nfp", 14),
    ("cpi", 14),
    ("inflation", 12),
    ("payroll", 12),
    ("recession", 13),
    ("debt ceiling", 13),
    ("crisis", 12),
    ("crash", 12),
    ("circuit breaker", 14),
    ("gdp", 10),
    ("ecb", 10),
    ("fed ", 10),
    ("bce", 8),
];

const MOTS_MARCHES: &[(&str, u8)] = &[
    ("bitcoin", 10),
    ("btc", 10),
    ("selloff", 10),
    ("sell-off", 10),
    ("liquidation", 10),
    ("crypto", 7),
    ("sec ", 8),
    ("halving", 9),
    ("stablecoin", 7),
    ("etf", 6),
    ("earnings", 6),
    ("volatility", 6),
    // Métaux précieux (XAUUSD, XAGUSD)
    ("xauusd", 10),
    ("gold", 8),
    ("silver", 7),
    ("spot gold", 9),
    ("ounce", 7),
    ("bullion", 7),
    ("precious metal", 8),
];

// ── Scoring et classification ────────────────────────────────────────────────

/// Cap par catégorie — évite les scores artificiels cumulatifs.
const CAP_MACRO: u8 = 25;
const CAP_MARCHES: u8 = 20;

/// Bonus temporel selon l'âge de l'article.
pub fn bonus_temporel(date_iso: &str) -> i16 {
    let age_min = chrono::DateTime::parse_from_rfc3339(date_iso)
        .map(|d| {
            Utc::now()
                .signed_duration_since(d.with_timezone(&Utc))
                .num_minutes()
        })
        .unwrap_or(9999);
    if age_min < 60 {
        10
    } else if age_min < 240 {
        5
    } else if age_min < 1440 {
        0
    } else {
        -10
    }
}

pub fn scorer(titre_lower: &str, base: u8, date_iso: &str) -> u8 {
    let mut bonus_macro: u8 = 0;
    for (mot, poids) in MOTS_MACRO {
        if titre_lower.contains(mot) {
            bonus_macro = bonus_macro.saturating_add(*poids);
        }
    }
    let mut bonus_marches: u8 = 0;
    for (mot, poids) in MOTS_MARCHES {
        if titre_lower.contains(mot) {
            bonus_marches = bonus_marches.saturating_add(*poids);
        }
    }
    let bonus = bonus_macro
        .min(CAP_MACRO)
        .saturating_add(bonus_marches.min(CAP_MARCHES));
    let raw = base as i16 + bonus as i16 + bonus_temporel(date_iso);
    raw.clamp(0, 100) as u8
}

/// Détermine le thème dominant de l'article.
pub fn classer_theme(titre_lower: &str, source: &str) -> &'static str {
    let source_lower = source.to_lowercase();

    const MOTS_CRYPTO: &[&str] = &[
        "bitcoin",
        "btc",
        "ethereum",
        "eth",
        "crypto",
        "defi",
        "nft",
        "blockchain",
        "altcoin",
        "stablecoin",
        "halving",
        "coinbase",
        "binance",
        "liquidation",
        "solana",
        "sol",
        "ripple",
        "xrp",
        "cardano",
        "ada",
        "dogecoin",
        "doge",
        "digital asset",
        "token",
        "web3",
        "on-chain",
        "memecoin",
        "satoshi",
        "hashrate",
        "mining",
        "miner",
        "exchange",
        "wallet",
        "decentral",
        "$btc",
        "$eth",
    ];
    const MOTS_METAUX: &[&str] = &[
        "gold",
        "silver",
        "xauusd",
        "xagusd",
        "ounce",
        "bullion",
        "precious metal",
        "spot gold",
        "kitco",
        "palladium",
        "platinum",
        "safe haven",
        "troy",
        "precious",
        "metal",
    ];
    const MOTS_MACRO_THEME: &[&str] = &[
        "federal reserve",
        "rate hike",
        "rate cut",
        "cpi",
        "nfp",
        "inflation",
        "payroll",
        "recession",
        "gdp",
        "ecb",
        "fed",
        "fomc",
        "treasury",
        "interest rate",
        "monetary policy",
        "central bank",
        "dollar",
        "dxy",
        "yield",
        "bond",
        "tariff",
        "trade war",
        "jobs report",
        "labor",
        "pce",
        "unemployment",
        "stimulus",
        "debt ceiling",
        "fiscal",
        "hawkish",
        "dovish",
        "senator",
        "congress",
        "white house",
        "government",
        "sec ",
        "cftc",
        "market",
        "stock",
        "equity",
        "s&p",
        "nasdaq",
        "wall street",
    ];

    if MOTS_CRYPTO.iter().any(|m| titre_lower.contains(m)) {
        return "crypto";
    }
    if MOTS_METAUX.iter().any(|m| titre_lower.contains(m)) {
        return "metaux";
    }
    if MOTS_MACRO_THEME.iter().any(|m| titre_lower.contains(m)) {
        return "macro";
    }

    // Fallback par source connue
    if source_lower.contains("cointelegraph")
        || source_lower.contains("cryptonews")
        || source_lower.contains("decrypt")
    {
        return "crypto";
    }
    if source_lower.contains("fxstreet") || source_lower.contains("kitco") {
        return "metaux";
    }
    if source_lower.contains("reuters")
        || source_lower.contains("cnbc")
        || source_lower.contains("marketwatch")
        || source_lower.contains("yahoo")
    {
        return "macro";
    }

    "autre"
}

/// Jaccard sur bigrammes de mots — [0.0, 1.0].
pub fn jaccard_bigrammes(a: &str, b: &str) -> f32 {
    use std::collections::HashSet;
    let bigrammes = |s: &str| -> HashSet<(String, String)> {
        let mots: Vec<&str> = s.split_whitespace().collect();
        mots.windows(2)
            .map(|w| (w[0].to_string(), w[1].to_string()))
            .collect()
    };
    let sa = bigrammes(&a.to_lowercase());
    let sb = bigrammes(&b.to_lowercase());
    let inter = sa.intersection(&sb).count() as f32;
    let union = sa.union(&sb).count() as f32;
    if union == 0.0 {
        1.0
    } else {
        inter / union
    }
}

/// Retire les doublons quasi-identiques (Jaccard > 0.6).
pub fn dedupliquer(articles: Vec<ArticleNews>) -> Vec<ArticleNews> {
    let mut gardes: Vec<ArticleNews> = Vec::with_capacity(articles.len());
    for article in articles {
        let est_doublon = gardes
            .iter()
            .any(|g| jaccard_bigrammes(&g.titre, &article.titre) > 0.6);
        if !est_doublon {
            gardes.push(article);
        }
    }
    gardes
}

pub fn niveau(score: u8) -> &'static str {
    match score {
        80..=100 => "critique",
        60..=79 => "important",
        40..=59 => "modere",
        _ => "veille",
    }
}
