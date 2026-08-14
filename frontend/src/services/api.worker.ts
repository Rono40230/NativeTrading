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

/** Résultat d'un import CSV. */
export interface ResultatImportCsv {
  total_lues: number
  total_inserees: number
  doublons: number
  lignes_ignorees: number
  asset: string
  timeframe: string
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

  /** POST /api/data/import-csv — contenu CSV brut + cible asset/timeframe. */
  async importerCsv(csv: string, asset: string, timeframe: string): Promise<ResultatImportCsv> {
    const res = await http.post('/api/data/import-csv', { csv, asset, timeframe }, { timeout: 120000 })
    return res.data
  },
}
