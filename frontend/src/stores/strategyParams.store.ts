import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { apiService } from '@/services/api.service'
import type { StraddleParams } from '@/components/common/StraddleParamsPanel.vue'
import type { SmcParams } from '@/components/common/SmcParamsPanel.vue'
import type { RocketsConfig } from '@/services/api.types'

export const useStrategyParamsStore = defineStore('strategyParams', () => {
  // Données brutes (noms de champs DB)
  const straddleRaw = ref<Record<string, any>>({})
  const smcRaw      = ref<Record<string, any>>({})
  const rocketsRaw  = ref<Record<string, any>>({})

  const isLoaded = ref(false)
  const loading  = ref(false)

  // Forme typée pour StraddleParamsPanel (seuil_atr ≠ atr_seuil en DB)
  const straddleParams = computed<StraddleParams>(() => ({
    atr_periode:     (straddleRaw.value.atr_periode     ?? 14),
    seuil_atr:       (straddleRaw.value.atr_seuil       ?? 1.5),
    tp_mult_1:       (straddleRaw.value.tp_mult_1       ?? 2.0),
    tp_mult_2:       (straddleRaw.value.tp_mult_2       ?? 3.5),
    tp_mult_3:       (straddleRaw.value.tp_mult_3       ?? 5.0),
    sl_mult:         (straddleRaw.value.sl_mult         ?? 0.5),
    trailing_atr:    (straddleRaw.value.trailing_atr    ?? 1.5),
    vente_partielle: straddleRaw.value.vente_partielle  ? 1 : 0,
  }))

  // Forme typée pour SmcParamsPanel
  const smcParams = computed<SmcParams>(() => ({
    atr_periode: (smcRaw.value.atr_periode ?? 14),
    score_min:   (smcRaw.value.score_min   ?? 70),
    atr_tp1:     (smcRaw.value.atr_tp1     ?? 1.5),
    atr_tp2:     (smcRaw.value.atr_tp2     ?? 3.0),
    atr_tp3:     (smcRaw.value.atr_tp3     ?? 5.0),
    atr_sl:      (smcRaw.value.atr_sl      ?? 1.0),
  }))

  // Forme typée pour RocketsReglages
  const rocketsConfig = computed<RocketsConfig | null>(() =>
    isLoaded.value ? (rocketsRaw.value as unknown as RocketsConfig) : null
  )

  // Chargement unique — ne fait rien si déjà chargé
  async function charger() {
    if (isLoaded.value || loading.value) return
    loading.value = true
    try {
      const [straddle, smc, rockets] = await Promise.all([
        apiService.getStraddleParams(),
        apiService.getSmcParams(),
        apiService.getRocketsConfig(),
      ])
      straddleRaw.value = straddle as Record<string, any>
      smcRaw.value      = smc      as Record<string, any>
      rocketsRaw.value  = rockets  as Record<string, any>
      isLoaded.value    = true
    } finally {
      loading.value = false
    }
  }

  // Sauvegarde + mise à jour du cache local → propagation réactive à tous les consumers
  async function saveStraddle(params: Record<string, any>) {
    await apiService.putStraddleParams(params)
    straddleRaw.value = { ...straddleRaw.value, ...params }
  }

  async function saveSmc(params: Record<string, any>) {
    await apiService.putSmcParams(params)
    smcRaw.value = { ...smcRaw.value, ...params }
  }

  async function saveRockets(params: RocketsConfig | Record<string, any>) {
    await apiService.putRocketsConfig(params as RocketsConfig)
    rocketsRaw.value = { ...rocketsRaw.value, ...(params as Record<string, any>) }
  }

  return {
    straddleRaw, smcRaw, rocketsRaw,
    straddleParams, smcParams, rocketsConfig,
    isLoaded, loading,
    charger, saveStraddle, saveSmc, saveRockets,
  }
})
