/**
 * Méthodes API dédiées au pilotage du data pipeline (workers d'ingestion,
 * statut runtime, routing des assets, import CSV générique).
 * Importées et spreadées dans apiService (api.service.ts).
 */
import { http } from './http.client'

// ── Types ─────────────────────────────────────────────────────────────────────

/** Configuration des workers d'ingestion (table configuration, clés worker_*). */
export interface WorkerConfig {
  timeframes: string[]
  historique_mois: number
  actif_bybit: boolean
}

/** Mise à jour partielle — seules les clés présentes sont écrites côté serveur. */
export interface WorkerConfigUpdate {
  timeframes?: string[]
  historique_mois?: number
  actif_bybit?: boolean
}

/** Statut runtime d'un worker (instantané des compteurs côté serveur). */
export interface WorkerStatutItem {
  /** Interrupteur configuré (worker_actif_*). */
  actif: boolean
  /** Connexion vivante (session WS ouverte / dernier cycle OK). */
  connecte: boolean
  /** Nombre d'actifs routés vers ce worker (config DB). */
  nb_assets: number
  /** Nombre d'actifs de la session/cour en cours (runtime). */
  nb_assets_session: number
  /** Timestamp Unix (s) de la dernière connexion — null si jamais connecté. */
  derniere_connexion: number | null
  /** Timestamp Unix (s) de la dernière bougie insérée — null si aucune. */
  derniere_bougie: number | null
  /** Bougies insérées depuis le démarrage du serveur. */
  bougies_inserees: number
}

export interface WorkerStatus {
  bybit: WorkerStatutItem
}

/** Asset avec colonnes de routing worker. */
export interface WorkerAsset {
  id: string
  source: string
  symbol_bybit: string | null
  actif: boolean
}

/** Résultat d'un backfill Dukascopy d'un mois (POST /api/data/dukascopy-backfill). */
export interface ResultatBackfillDukascopy {
  asset: string
  instrument: string
  diviseur: number
  timeframe: string
  annee: number
  mois: number
  /** Jours réellement téléchargés avec données. */
  jours_traites: number
  /** Jours 404/fériés/week-ends sans requête. */
  jours_sans_donnees: number
  /** Bougies M1 téléchargées (avant agrégation). */
  bougies: number
  /** Bougies au timeframe demandé (après agrégation). */
  bougies_cible: number
  /** Bougies réellement insérées (INSERT OR IGNORE — doublons exclus). */
  inserees: number
  /** Signal non bloquant (ex: instrument 404 systématique). */
  avertissement: string | null
  /** Erreurs par jour (rate limit, réseau…). */
  erreurs: string[]
}

// ── Méthodes ──────────────────────────────────────────────────────────────────

export const workerApi = {
  /** GET /api/worker/config — configuration courante des workers. */
  async getWorkerConfig(): Promise<WorkerConfig> {
    const res = await http.get('/api/worker/config')
    return res.data
  },

  /** PUT /api/worker/config — mise à jour partielle, retourne la config effective. */
  async putWorkerConfig(body: WorkerConfigUpdate): Promise<WorkerConfig> {
    const res = await http.put('/api/worker/config', body)
    return res.data
  },

  /** GET /api/worker/status — statut runtime (poll 30 s dans la vue Données). */
  async getWorkerStatus(): Promise<WorkerStatus> {
    const res = await http.get('/api/worker/status')
    return res.data
  },

  /** GET /api/worker/assets — routing complet des assets. */
  async getWorkerAssets(): Promise<WorkerAsset[]> {
    const res = await http.get('/api/worker/assets')
    return res.data.assets
  },

  /**
   * POST /api/data/dukascopy-backfill — télécharge UN mois de candles M1
   * depuis le datafeed public Dukascopy (rate-limité : ~4 s par fichier
   * quotidien → un mois ouvré ≈ 1,5-3 min). Timeout long volontaire.
   * L'instrument est résolu côté serveur (colonne assets.datafeed_dukascopy).
   */
  async backfillDukascopyMois(params: {
    asset: string
    timeframe: string
    annee: number
    mois: number
  }): Promise<ResultatBackfillDukascopy> {
    const res = await http.post('/api/data/dukascopy-backfill', params, { timeout: 600_000 })
    return res.data
  },
}
