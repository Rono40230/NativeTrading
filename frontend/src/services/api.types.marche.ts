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
  source?: 'binance' | 'mt5'
  actif?: boolean
  symbol_mt5?: string | null
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

/// Sentiment composite 0-100 par classe — retiré de l'UI le 05/09 (décision
/// propriétaire : jauge fear & greed + mini-jauges crypto/forex/métaux/indices
/// sans intérêt). Le composite reste calculé côté backend : il alimente le
/// filtre de sentiment des signaux SMC (/api/sentiment/composite = inspection).

export interface TraductionReponse {
  texte_fr: string
}
