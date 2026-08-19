/// Types liés aux données de marché, IA, calendrier macro, news et tendances.
/// Importés et re-exportés depuis api.types.ts — ne pas importer ce fichier directement.

export interface PatternHoraire {
  heure: number        // 0-23 UTC
  jour_semaine: number // 0=dim, 1=lun, ..., 6=sam
  atr_moyen: number
  nb_points: number
  cluster: number      // 0=calme, 1=modéré, 2=élevé, 3=extrême
}

export interface ReponsePatternsVolatilite {
  patterns: PatternHoraire[]
  seuil_straddle_calibre: number
  nb_points_total: number
  asset: string
  timeframe: string
}

export interface RequeteAnalyseIA {
  asset: string
  timeframe: string
  direction: string
  score_smc: number
  score_min?: number
  prix_entree: number
  stop_loss: number
  take_profit_1: number
  take_profit_2?: number
  take_profit_3?: number
  tendance: number
  order_block: number
  imbalance: number
  ifvg: number
  fibonacci: number
  confiance_ml: number
}

export interface ReponseAnalyseIA {
  analyse: string
  modele: string
}

export interface ReponseChatIA {
  reponse: string
  modele: string
}

export interface ReponseChartIA {
  analyse: string
  modele: string
}

export interface ImageAvecTF {
  base64: string
  timeframe: string
}

export interface LigneTendanceKasper {
  tf: string
  tendance: 'haussier' | 'baissier' | null
  valeur_ema_rapide: number | null
  valeur_ema_lente: number | null
}

export type ModeCalculTendance = 'bougie_cloturee' | 'bougie_en_cours'

export interface ReponseTendanceMultiTf {
  asset: string
  ema_rapide: number
  ema_lente: number
  mode_calcul: ModeCalculTendance
  lignes: LigneTendanceKasper[]
}

export interface AssetInfo {
  id: string
  nom: string
  type: 'crypto' | 'metal' | 'forex' | 'indice'
  source?: 'binance'
  actif?: boolean
}

export interface AnnonceCalendrier {
  id: string
  date_heure: string
  devise: string
  titre: string
  impact: 'High' | 'Medium'
  precedent: string | null
  prevision: string | null
  est_passe?: boolean
}

export interface FearGreedData {
  valeur: number
  label: string
  categorie: 'extreme_fear' | 'fear' | 'neutral' | 'greed' | 'extreme_greed'
}

export interface EntiteSentiment {
  nom: string
  prix: number
  /** Variation de la séance en cours (colonne « Jour », live). */
  variation_pct: number
  /** Variation de la veille clôturée (colonne « Veille », figée). */
  variation_veille?: number
}

export interface SentimentMarche {
  date: string
  /** Date de la référence figée (colonne « Veille »). */
  date_veille?: string
  usa: EntiteSentiment[]
  europe: EntiteSentiment[]
  matieres_premieres: EntiteSentiment[]
  cryptos: EntiteSentiment[]
  vix: number | null
}

/// Sentiment composite 0-100 par classe d'actifs (GET /api/sentiment/composite).
export interface SentimentComposite {
  global: number | null
  crypto: number | null
  forex: number | null
  metaux: number | null
  indices: number | null
  rsi_btc: number | null
  rsi_eth: number | null
  rsi_xau: number | null
  breadth_pct: number | null
  fear_greed: number | null
  vix_score: number | null
  vix_brut: number | null
  /// CNN Fear & Greed (référence actions US) — jauge globale.
  cnn_fg: number | null
}

export type NiveauAlerte = 'critique' | 'important' | 'modere' | 'veille'

export interface ArticleNews {
  id: string
  titre: string
  titre_fr?: string
  source: string
  url: string
  date: string
  score: number
  niveau: NiveauAlerte
  theme: 'macro' | 'crypto' | 'metaux' | 'autre'
  sentiment?: 'haussier' | 'neutre' | 'baissier'
  /** Résumé RSS (revue de presse) — affiché immédiatement, avant le scrape. */
  resume_source?: string
}

export interface AlertesNews {
  articles: ArticleNews[]
  score_max: number
  mis_a_jour: string
}

export interface ContenuArticle {
  texte: string
}

export interface TraductionReponse {
  texte_fr: string
}
