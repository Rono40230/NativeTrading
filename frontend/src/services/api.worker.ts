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

// ── Méthodes ──────────────────────────────────────────────────────────────────

export const workerApi = {
  /** GET /api/worker/config — configuration courante des workers. */
  async getWorkerConfig(): Promise<WorkerConfig> {
    const res = await http.get('/api/worker/config')
    return res.data
  },

  /** PUT /api/worker/config — mise à jour partielle, retourne la config effective. */

  /** GET /api/worker/status — statut runtime (poll 30 s dans la vue Données). */
  async getWorkerStatus(): Promise<WorkerStatus> {
    const res = await http.get('/api/worker/status')
    return res.data
  },
}
