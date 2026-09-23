// ── Précision M5 ──────────────────────────────────────────────────────────────
//
// (Les fonctions créneaux — lister/inserer/supprimer StraddleCreneau et la
// mise à jour precision — ont été supprimées le 23/09 avec l'endpoint
// /api/straddle/analyser : l'analyse LLM éphémère dupliquait le pipeline
// créneaux IA matinal, persistant celui-là. La table straddle_creneaux
// reste en base, simplement plus alimentée.)

/// Résultat d'une analyse de précision M5 d'un créneau horaire — calculé par
/// `strategies::straddle_precision`, servi par /api/straddle/precision-horaire.
pub struct PrecisionM5 {
    pub timing_optimal: String,
    pub fenetre_entree: String,
    pub whipsaw_minutes: i64,
    pub nb_occurrences: i64,
    pub atr_pic: f64,
}
