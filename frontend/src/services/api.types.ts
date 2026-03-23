export interface Candle {
  timestamp: string
  open: number
  high: number
  low: number
  close: number
  volume: number
}

export interface BacktestResults {
  total_trades: number
  winning_trades: number
  losing_trades: number
  win_rate: number
  capital_initial: number
  capital_final: number
  roi_pct: number
  profit_net: number
  sharpe_ratio: number
  max_drawdown_pct: number
  profit_factor: number
}

export interface PredictionML {
  asset: string
  direction: string
  confiance: number
  est_confiant: boolean
  modele_pret: boolean
}

export interface ReponseEntrainement {
  success: boolean
  accuracy_rf: number
  accuracy_lstm: number
  accuracy_finale: number
  nb_echantillons: number
  duree_ms: number
  derive_detectee: boolean
  message: string
}

export interface HistoriqueEntrainement {
  id: number
  cree_le: number
  asset: string
  timeframe: string
  nb_bougies: number
  accuracy_rf: number
  accuracy_lstm: number
  accuracy_finale: number
  duree_ms: number
  derive_detectee: boolean
}

export interface HistoriqueML {
  historique: HistoriqueEntrainement[]
  derive_detectee: boolean
  seuil_derive: number
  nb_entrainements: number
}

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
  prix_entree: number
  stop_loss: number
  take_profit: number
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

export interface StatutIA {
  ollama_disponible: boolean
  modele: string
  url: string
}

export interface Signal {
  id: string
  asset: string
  timeframe: string
  direction: string
  score: number
  prix_entree: number
  stop_loss: number
  take_profit: number[]   // [tp1, tp2, tp3]
  strategie: string
  statut: string          // 'Actif' | 'Fermé' | 'Annulé'
  verdict: string | null  // null | 'SL' | 'TP1' | 'TP2' | 'TP3' | 'expire'
  prix_verdict: number | null
  ferme_le: number | null
  cree_le: number
}

export interface StatutSignalEngine {
  actif: boolean
  prochain_cycle_dans_secs: number
  signaux_24h: number
  assets_surveilles: number
  timeframes: string[]
  intervalle_secs: number
}

export interface ScoreSmc {
  total: number
  tendance: number
  order_block: number
  imbalance: number
  ifvg: number
  fibonacci: number
  direction: string
  confluence: boolean
  kill_zone_active: boolean
  sweep_detecte: boolean
}

export type { PointSerie, ZoneOb, ZoneImbalance, ZoneFvgBpr, ZoneBpr, ZoneIfvg, NiveauxFibonacci, ResultatTendance, NiveauLiquidite, DeviationAsie, RangeAsie, NiveauForceSignal, DirectionSignal, SignalIndicateur, ReponseIndicators, IndicatorsParams } from './api.types.indicators'


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
  source?: 'binance' | 'ib'
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

export interface EntiteSentiment {
  nom: string
  prix: number
  variation_pct: number
}

export interface SentimentMarche {
  date: string
  usa: EntiteSentiment[]
  europe: EntiteSentiment[]
  matieres_premieres: EntiteSentiment[]
  cryptos: EntiteSentiment[]
  vix: number | null
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

export interface CouvertureDonnees {
  asset: string
  timeframe: string
  count: number
  min_ts: number
  max_ts: number
}

export interface RequeteCollecte {
  assets?: string[]
  timeframes?: string[]
  mois?: number
}

export interface ResultatCollecteItem {
  asset: string
  timeframe: string
  fetched?: number
  inseres?: number
  erreur?: string
}

export interface ResultatCollecte {
  total_inseres: number
  mois: number
  resultats: ResultatCollecteItem[]
}

export interface RocketSignalSave {
  ticker: string; phase: string; score: number
  prix_entree: number; stop_loss: number; target: number
  ratio_volume: number; atr_ratio: number; rsi: number
}

export interface RocketSignalHistorique {
  id: number; ticker: string; phase: string; score: number
  prix_entree: number; stop_loss: number
  target: number; target2: number | null; target3: number | null
  verdict: string | null; prix_verdict: number | null
  prix_peak: number | null; atr14: number | null; rsi: number
  ratio_volume: number; atr_ratio: number
  llm_valide: number | null; llm_conviction: number | null; llm_raison: string | null
  cree_le: string; maj_le: string | null
}

export interface RocketRecommandation {
  type: string
  description: string
  impact_estime: string
  priorite: 'haute' | 'moyenne' | 'faible'
}

export interface RocketAnalyseLlm {
  id: number
  nb_trades: number
  synthese: string
  meilleur_setup: string | null
  pire_setup: string | null
  recommandations: string // JSON brut
  cree_le: string
}

export interface RocketsConfig {
  score_min: number
  phases_actives: string[]
  rsi_max: number
  rsi_min: number
  ratio_volume_min: number
  vol_marche_min: number
}

// ── Straddle ──────────────────────────────────────────────────────────────────

export interface StraddleCreneau {
  id: number
  asset: string
  jour_semaine: number | null    // 0=Lundi...6=Dimanche, null=tous
  heure_debut: string            // "14:00" UTC
  heure_fin: string              // "16:00" UTC
  atr_moyen: number | null
  frequence: number | null       // 0.0–1.0
  llm_raison: string | null
  llm_conviction: number | null  // 0–100
  statut: 'a_tester' | 'valide' | 'invalide'
  cree_le: string
  backtest_winrate: number | null
  backtest_profit_factor: number | null
}

export interface ReponseAnalyseStraddle {
  creneaux: StraddleCreneau[]
  nb_analyses: number
  nb_retenus: number
  message?: string
}
