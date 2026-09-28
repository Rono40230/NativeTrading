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

/** Bandeau sentiment (15/09) — les vraies jauges au-dessus du tableau. */
export interface Fng {
  valeur: number
  classe: string
  /** Variation vs la veille (points d'indice). */
  delta_veille: number
}

export interface Positioning {
  asset: string
  ratio_long: number
  ratio_short: number
  /** ratio_long / ratio_short (1,0 = équilibre). */
  ls: number
  /** Funding courant en % (négatif = shorts paient). */
  funding_pct: number
}

export interface Breadth {
  univers: string
  au_dessus: number
  total: number
}

export interface PresseBias {
  haussier: number
  neutre: number
  baissier: number
}

export interface BandeauSentiment {
  fng?: Fng
  positioning: Positioning[]
  breadth: Breadth[]
  presse?: PresseBias
  maj_le: number
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
  bandeau?: BandeauSentiment
}

/// Sentiment composite 0-100 par classe — retiré de l'UI le 05/09 (décision
/// propriétaire : jauge fear & greed + mini-jauges crypto/forex/métaux/indices
/// sans intérêt). Le composite reste calculé côté backend : il alimente le
/// filtre de sentiment des signaux SMC (/api/sentiment/composite = inspection).

export interface TraductionReponse {
  texte_fr: string
}

// ── Événements prévisibles × réactivité (28/09) ─────────────────────────────

/** Ligne événement × asset : ATR des 3 premières minutes de l'événement
 *  rapporté à l'habitude M1 de l'asset sur la période (×2 = deux fois le
 *  range d'une minute normale). */
export interface EvenementReactif {
  asset: string
  ratio: number
  atr: number
  habitude: number
  minutes: number
}

/** Un événement récurrent défini dans son fuseau d'origine (New York,
 *  Londres…), converti en heure de Paris avec les bascules été/hiver. */
export interface EvenementPrevisible {
  ident: string
  nom: string
  detail: string
  fuseau: string
  heure_locale: string
  jours: number[]
  prochaine_ts: number | null
  prochaine_heure_paris: string | null
  max_ratio: number
  reactivite: EvenementReactif[]
}

export interface ReponseEvenements {
  calcule_le: number
  periode_jours: number
  fenetre_minutes: number
  evenements: EvenementPrevisible[]
}
