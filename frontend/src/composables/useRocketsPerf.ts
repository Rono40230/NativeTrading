import { ref, onMounted, onUnmounted } from 'vue'
import { apiService } from '@/services/api.service'

export type EquityPoint = {
  ticker: string
  verdict: string
  pnl_r: number
  equity_cumulee: number
  ferme_le: number
  duree_min?: number
}

export type EquityData = {
  capital_initial: number
  risk_pct: number
  nb_trades_saisis: number
  points: EquityPoint[]
}

export function useRocketsPerf(capital = 10000, risk_pct = 0.015) {
  const data = ref<EquityData | null>(null)
  const chargement = ref(false)

  async function charger(silencieux = false) {
    if (!silencieux) chargement.value = true
    try {
      const result = await apiService.getRocketsEquity(capital, risk_pct)
      data.value = result
    } catch {
      data.value = null
    } finally {
      if (!silencieux) chargement.value = false
    }
  }

  let _poll: ReturnType<typeof setInterval> | null = null

  onMounted(() => {
    charger()
    _poll = setInterval(() => charger(true), 30_000)
  })

  onUnmounted(() => {
    if (_poll !== null) { clearInterval(_poll); _poll = null }
  })

  return { data, chargement, charger }
}
