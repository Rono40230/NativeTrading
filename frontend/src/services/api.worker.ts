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
}

// ── Méthodes ──────────────────────────────────────────────────────────────────

export const workerApi = {
  /** GET /api/worker/config — configuration courante des workers. */
  async getWorkerConfig(): Promise<WorkerConfig> {
    const res = await http.get('/api/worker/config')
    return res.data
  },
}
