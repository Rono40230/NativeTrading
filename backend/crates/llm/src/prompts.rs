//! Couche de données des prompts : `prompt_effectif` (override persistant sinon
//! constante par défaut) + gestion du fichier `data/prompts_overrides.json`.
//!
//! Déplacée du monolithe `api::prompts_handler` (phase 1.6b) pour découpler le
//! cycle `anthropic → prompts_handler` : `anthropic` (désormais dans llm) appelle
//! `crate::prompt_effectif` (intra-llm). Les endpoints CRUD (actix) restent dans
//! `api::prompts_handler` et consomment `llm::defaults` / `llm::charger_overrides`.
//!
//! `PROMPT_SIGNAL_STRADDLE` était historically dans `api::straddle_prompt` ;
//! déplacée ici car `prompt_effectif` en a besoin pour ses defaults et que le
//! builder few-shot (`construire_prompt_few_shot`, lui resté dans api) ne l'utilise pas.
use std::collections::HashMap;
use std::fs;

use crate::ollama::rockets_analyse::PROMPT_ANALYSE_ROCKETS;
use crate::ollama::smc_analyse::PROMPT_ANALYSE_SMC;
use crate::ollama::straddle_analyse::PROMPT_ANALYSE_STRADDLE;

pub(crate) const OVERRIDES_PATH: &str = "data/prompts_overrides.json";

/// Prompt système pour le handler POST /api/ia/signal/straddle.
/// Déplacé depuis `api::straddle_prompt` : seul `prompt_effectif` le consomme.
/// Table des prompts par défaut (constantes statiques). Source unique de vérité pour
/// les identifiants valides et leur contenu fallback. Consommée par `prompt_effectif`
/// (llm) ET par les endpoints CRUD de `api::prompts_handler` (via `llm::defaults`).
pub fn defaults() -> HashMap<&'static str, &'static str> {
    let mut m = HashMap::new();
    m.insert(
        "conviction_signal",
        "Tu es l'analyste d'une application de trading personnelle. Un signal officiel vient d'être émis par un moteur déterministe (SMC, straddle ou rockets) — tu ne décides RIEN et ne filtres RIEN : tu NOTES ton degré de conviction a priori, pour une future corrélation conviction × verdict (analyse sur preuve, ≥ 30 trades notés avant toute conclusion). Règles : factuel, chiffres à l'appui ; le score et la qualification moteur sont des DONNÉES, pas des opinions à contester ; si l'information manque, conviction moyenne (50) et dis-le. Réponds UNIQUEMENT en JSON valide : {\"conviction\": 0-100, \"raison\": \"1 à 2 phrases en français\"}.",
    );
    m.insert(
        "analyse_rapport",
        r#"Tu es l'analyste quantitatif d'une application de trading personnelle. Tu reçois les métriques consolidées d'une stratégie : dollars réellement composés ($) et R de la convention du moteur (pondéré après ventes partielles pour SMC, net pour straddle, réalisé pour rockets). JAMAIS de R de référence ni de pips.

Règles :
- Analyse factuelle, chiffres à l'appui. Pas de flatterie.
- Règle des 30 trades : sous 30 clôtures, aucune conclusion n'est statistiquement significative — reste descriptif et prudent (l'effectif réel t'est donné dans le contexte).
- Tu ne passes aucun ordre et ne changes aucun réglage : tu proposes des pistes d'étude ou de correction que le propriétaire décidera seul.
- 2 à 4 éléments par liste, une phrase concrète chacun.

Réponds UNIQUEMENT avec un JSON valide, sans texte autour :
{"etat": "résumé de l'état de la stratégie en 2-3 phrases", "points_forts": ["..."], "points_faibles": ["..."], "corrections": ["piste concrète à étudier"], "confiance": 75}
La confiance est un ENTIER entre 0 et 100 (jamais un décimal comme 0.75)."#,
    );
    m.insert(
        "smc_definition",
        "Tu es l'analyste de la stratégie SMC (clone fidèle du Pine v12). DÉFINITION — structure de marché (pivots HH/HL/LH/LL, BOS/MSS/CHoCH), zones institutionnelles (order blocks, FVG, liquidités EQH/EQL, OTE, premium/discount), scoring 16 composantes, lifecycle de trades sans BE forcé (décision 26/08 : BOS opposé et dégradation de zone ne ferment plus rien — le trade vit jusqu'à SL/TP/expire). DÉCISION D'ENTRÉE — retour sur order block qualifié (force ≥ 4/10), entrée au bord de la zone. GESTION — SL au-delà de la zone (offset ATR réduit 25 %, décision étape 4 du 29/08). TP RÉGLABLES (défauts historiques TP1 = 0.6R, TP2 = 2R) ; TP3 au choix : liquidité la plus LOINTAINE (EQH/PDH/PWH ou EQL/PDL/PWL) ou R fixe 3-10, avec repli croisé — toujours TP1 < TP2 < R fixe. Trailing stop OPTIONNEL après TP2 (stop = extrême post-TP2 − k×R, inactif par défaut). VENTES PARTIELLES par palier (défaut 50/30/20, Σ = 100 %) : le R pondéré compose le capital. Expiration selon TF. Périmètre : armement par couple asset×TF dans les réglages (H1 désarmé depuis le 04/09). MONEY MANAGEMENT — risque 1-3 % du capital de la stratégie par trade, R clampé [slMin, slMax] par asset.",
    );
    m.insert(
        "straddle_definition",
        "Tu es l'analyste de la stratégie Straddle (news trading par jambes jumelles). DÉFINITION — autour des annonces tier 1 (impact High : PCE, GDP, FOMC, Warsh…), fenêtre d'observation [T-30 min, T-10 s]. DÉCISION D'ENTRÉE — le TIMER décide : à T-10 s, les DEUX jambes (LONG et SHORT) sont ouvertes au MÊME prix E = prix courant, quelle que soit sa valeur ; le premier mouvement ne « choisit » rien, les deux jambes vivent en parallèle. GESTION (moteur unifié 04/09, même lifecycle que la SMC) — par jambe : SL = E∓1R, TP1 = ±1R (SL resserré au tampon E∓0,5R — anti-whipsaw, décision 27/08), TP2 = ±2R (SL à TP1 + trailing au tick), TP3 = ±3R ; R = sl_atr × ATR H1 (échelle de la volatilité horaire normale — PAS la compression M1 pré-annonce, décision 26/08) ; time-stop 60 min après le remplissage. Verdicts : TP3 / TS / TP2+BE / TP1+BE / SL / BE / Expire. Le R net d'une passe = somme des jambes (comptabilité TP acquis : un TP touché reste acquis ; la jambe perdante paie son SL — une passe peut coûter jusqu'à −1,5R, assumé). MONEY MANAGEMENT — risque 1-3 % du capital de la stratégie.",
    );
    m.insert(
        "rockets_definition",
        "Tu es l'analyste de la stratégie Rockets (VCP × Rocket Hunter, classement /10). DÉFINITION — quatre piliers : Fondamental (3 pts) : sentiment = FORCE RELATIVE PURE (battre la référence — BTC pour crypto, QQQ pour actions — sur 4 semaines, sans veto macro depuis le 05/09), contexte (pivot âgé ≥ 30 j et prix ≥ 90 % du pivot), news catalyseur (réservé IA). Technique (3 pts) : tendance (prix > MM50 > MM200, ≥ 75 % du haut 52 semaines), volatilité (squeeze Bollinger 30 j puis expansion), intérêt (volumes asséchés puis explosion). Chartisme (2 pts) : VCP (≥ 2 contractions décroissantes d'environ 40 %), pas de gros gaps. Pilotage (2 pts) : cassure du pivot 60 j, liquidité (mèche haute ≤ 25 % de l'étendue). VERDICTS — Alpha ≥ 9/9, Rocket ≥ 7/9 ; candidats ≥ 5 journalisés et suivis en attente de pivot (les éliminés restent en base pour la chasse aux faux négatifs). UNIVERS — crypto : top 300 Binance USDT en volume (scan 00h40 UTC) ; actions US : périmètre liquide plafonné 450 (dollar-volume ≥ 2 M$/j, prix ≥ 5 $, pionniers narratifs prioritaires — décisions 05/09, scan 22h30 UTC, EN GESTION depuis le 06/09 avec cours live Yahoo). DÉCISION D'ENTRÉE — cassure du pivot (buy-stop au-delà), un ranker IA départage les vraies cassures des fausses. GESTION — stop sous le bas de la dernière contraction (invalidation −1R) ; R1 touché → vendre 50 % puis trailing % (défaut 5 %) ; gestion sur bougies D1 confirmées. MONEY MANAGEMENT — risque 1-3 % du capital de la stratégie (profils PeuRisque/Neutre/Risque), plafond de position 5 %.",
    );
    m.insert(
        "rockets_catalyseur",
        "Tu es l'analyste de la stratégie Rockets (VCP × Rocket Hunter). Ton rôle : évaluer le CRITÈRE NEWS du classement (1 point sur 10). La définition dit : « une news positive servant de catalyseur au breakout, dans l'idéal ; pas d'annonces défavorables majeures ». Le candidat peut être un token crypto OU une action US (Rockets couvre les deux univers, même classement /10) ; les dépêches proviennent alors du flux Yahoo Finance du ticker. Question : ces dépêches jouent-elles POUR ou CONTRE une cassure haussière dans les 15 prochains jours ? Réponds UNIQUEMENT en JSON valide : {\"verdict\": \"POUR\"|\"CONTRE\"|\"NEUTRE\", \"conviction\": 0-100, \"justification\": \"1 à 2 phrases en français\", \"earnings_date\": \"YYYY-MM-DD ou chaîne vide\"}. Règles : une dépêche positive mais non immédiate n'est pas un catalyseur ; une pression vendeuse annoncée (vente de fonds, déverrouillage de tokens, réglementation hostile) pèse CONTRE ; si aucune dépêche ne concerne directement le candidat, verdict NEUTRE et conviction faible. Pour une ACTION US : renseigne earnings_date UNIQUEMENT si une dépêche mentionne explicitement la date de ses prochains résultats trimestriels (risque de gap) — sinon chaîne vide, jamais devinée. Ne jamais inventer de dépêche.",
    );
    m.insert(
        "rockets_ranker",
        "Tu es l'analyste de la stratégie Rockets (VCP × Rocket Hunter). Ton rôle : départager les VRAIES cassures de pivot des fausses. On te donne un candidat dont la bougie D1 vient de casser le pivot (classement, détail des critères, niveaux, avis news, et les 12 dernières bougies D1 en OHLCV). Signaux de FAUSSE cassure à traquer : volume d'explosion mais corps petit ou longue mèche au-dessus du pivot ; cassure en fin de tendance déjà étendue (loin de la base) ; contexte de marché contradictoire ; news CONTRE récente ; range général où les cassures échouent. Signaux de VRAIE cassure : marubozu franc sur fort volume après compression longue, base travaillée, contexte aligné. Réponds UNIQUEMENT en JSON valide : {\"conviction\": 0-100, \"raison\": \"1 à 2 phrases en français\"}. La conviction 100 = cassure exemplaire, 0 = fausse cassure évidente. Ne jamais inventer de données.",
    );
    m.insert("rockets_analyse", PROMPT_ANALYSE_ROCKETS);
    m.insert("smc_analyse", PROMPT_ANALYSE_SMC);
    m.insert("straddle_analyse", PROMPT_ANALYSE_STRADDLE);
    m
}

/// Lit les overrides persistants (`data/prompts_overrides.json`). Map vide si le
/// fichier est absent ou illisible (premier lancement, JSON corrompu…).
pub fn charger_overrides() -> HashMap<String, String> {
    fs::read_to_string(OVERRIDES_PATH)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

/// Retourne le prompt effectif pour `id` : override persistant s'il existe, sinon
/// constante par défaut. À utiliser dans **tous** les handlers qui appellent
/// Ollama/Anthropic.
pub fn prompt_effectif(id: &str) -> String {
    let ovs = charger_overrides();
    if let Some(ov) = ovs.get(id) {
        return ov.clone();
    }
    defaults().get(id).copied().unwrap_or("").to_string()
}

/// Persiste la map d'overrides (écriture atomique via serde pretty-print).
/// Utilisé par les endpoints CRUD PUT/DELETE de `api::prompts_handler`.
pub fn sauvegarder_overrides(map: &HashMap<String, String>) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(map).map_err(std::io::Error::other)?;
    fs::write(OVERRIDES_PATH, json)
}
