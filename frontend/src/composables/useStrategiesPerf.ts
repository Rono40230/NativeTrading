import { ref, onMounted, onUnmounted } from 'vue'
import { apiService } from '@/services/api.service'

export type StratEquityPoint = {
  asset: string
  verdict: string
  pnl_r: number
  equity_cumulee: number
  ferme_le: number
}

export type StratEquityData = {
  capital_initial: number
  risk_pct: number
  nb_trades_saisis: number
  points: StratEquityPoint[]
}

export function useSmcPerf(capital = 10000, risk_pct = 0.015) {
  const data = ref<StratEquityData | null>(null)
  const chargement = ref(false)

  async function charger(silencieux = false) {
    if (!silencieux || data.value === null) chargement.value = true
    try {
      data.value = await apiService.getSmcEquity(capital, risk_pct)
    } catch {
      data.value = null
    } finally {
      chargement.value = false
    }
  }

  let _poll: ReturnType<typeof setInterval> | null = null

  onMounted(() => { charger(); _poll = setInterval(() => charger(true), 30_000) })
  onUnmounted(() => { if (_poll !== null) { clearInterval(_poll); _poll = null } })

  return { data, chargement, charger }
}

export function useStraddlePerf(capital = 10000, risk_pct = 0.015) {
  const data = ref<StratEquityData | null>(null)
  const chargement = ref(false)

  async function charger(silencieux = false) {
    if (!silencieux || data.value === null) chargement.value = true
    try {
      data.value = await apiService.getStraddleEquity(capital, risk_pct)
    } catch {
      data.value = null
    } finally {
      chargement.value = false
    }
  }

  let _poll: ReturnType<typeof setInterval> | null = null

  onMounted(() => { charger(); _poll = setInterval(() => charger(true), 30_000) })
  onUnmounted(() => { if (_poll !== null) { clearInterval(_poll); _poll = null } })

  return { data, chargement, charger }
}
