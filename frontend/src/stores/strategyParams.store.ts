import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { apiService } from '@/services/api.service'
import type { StraddleParams } from '@/components/common/StraddleParamsPanel.vue'
import type { SmcParams } from '@/components/common/SmcParamsPanel.vue'
import type { RocketsConfig } from '@/services/api.types'
// Formes API/DB (noms de champs = source de vérité backend) — alias pour éviter
// la collision avec les types UI SmcParams/StraddleParams (champs renommés pour l'IHM).
import type { StraddleParams as ParamsStraddle } from '@/generated/ParamsStraddle'
import type { SmcParams as ParamsSmc } from '@/generated/ParamsSmc'

// Défauts mirroirs des impl Default Rust (db::strategies_params) — utilisés tant que
// l'API n'a pas encore répondu, pour que l'IHM reste non-null et réactive.
const SMC_PARAMS_DEFAUT: ParamsSmc = {
  atr_periode: 14,
  score_min: 70,
  atr_tp1: 2.0,
  atr_tp2: 3.0,
  atr_tp3: 5.0,
  atr_sl: 1.0,
  horizon_bougies: 24,
  vente_partielle: true,
  kill_zone_filtre: true,
  pct_cloture_tp1: 0.33,
  pct_cloture_tp2: 0.33,
}

const STRADDLE_PARAMS_DEFAUT: ParamsStraddle = {
  atr_periode: 14,
  atr_seuil: 1.5,
  tp_mult_1: 1.5,
  tp_mult_2: 2.5,
  tp_mult_3: 5.0,
  sl_mult: 0.5,
  horizon_bougies: 48,
  trailing_atr: 1.5,
  vente_partielle: true,
  pct_cloture_tp1: 0.33,
  pct_cloture_tp2: 0.33,
  placement_sec: 10,
  trailing_r: 1.0,
}

// Défauts mirroirs des migrations backend (rockets_config) — placeholder pré-chargement.
const ROCKETS_CONFIG_DEFAUT: RocketsConfig = {
  score_min: 40,
  phases_actives: ['breakout', 'prelancement'],
  rsi_max: 85.0,
  rsi_min: 0.0,
  ratio_volume_min: 1.5,
  vol_marche_min: 500000.0,
  sl_mult: 1.0,
  trailing_coeff_min: 1.5,
  trailing_coeff_max: 5.0,
  seuil_score_faible: 65,
  seuil_score_fort: 80,
  vente_partielle: true,
  pct_cloture_tp1: 0.33,
  pct_cloture_tp2: 0.33,
}

export const useStrategyParamsStore = defineStore('strategyParams', () => {
  // Données brutes (noms de champs DB/API)
  const straddleRaw = ref<ParamsStraddle>({ ...STRADDLE_PARAMS_DEFAUT })
  const smcRaw      = ref<ParamsSmc>({ ...SMC_PARAMS_DEFAUT })
  const rocketsRaw  = ref<RocketsConfig>({ ...ROCKETS_CONFIG_DEFAUT })

  const isLoaded = ref(false)
  const loading  = ref(false)

  // Forme typée pour StraddleParamsPanel (seuil_atr ≠ atr_seuil en DB ;
  // vente_partielle : bool API ↔ number 0/1 IHM)
  const straddleParams = computed<StraddleParams>(() => {
    const r = straddleRaw.value
    return {
      atr_periode:     r.atr_periode,
      seuil_atr:       r.atr_seuil,
      tp_mult_1:       r.tp_mult_1,
      tp_mult_2:       r.tp_mult_2,
      tp_mult_3:       r.tp_mult_3,
      sl_mult:         r.sl_mult,
      trailing_atr:    r.trailing_atr,
      vente_partielle: r.vente_partielle ? 1 : 0,
      pct_cloture_tp1: r.pct_cloture_tp1,
      pct_cloture_tp2: r.pct_cloture_tp2,
    }
  })

  // Forme typée pour SmcParamsPanel (vente_partielle : bool API ↔ number 0/1 IHM)
  const smcParams = computed<SmcParams>(() => {
    const r = smcRaw.value
    return {
      atr_periode: r.atr_periode,
      score_min:   r.score_min,
      atr_tp1:     r.atr_tp1,
      atr_tp2:     r.atr_tp2,
      atr_tp3:     r.atr_tp3,
      atr_sl:      r.atr_sl,
      vente_partielle: r.vente_partielle ? 1 : 0,
      pct_cloture_tp1: r.pct_cloture_tp1,
      pct_cloture_tp2: r.pct_cloture_tp2,
    }
  })

  // Forme typée pour RocketsReglages
  const rocketsConfig = computed<RocketsConfig | null>(() =>
    isLoaded.value ? rocketsRaw.value : null
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
      straddleRaw.value = straddle
      smcRaw.value      = smc
      rocketsRaw.value  = rockets
      isLoaded.value    = true
    } finally {
      loading.value = false
    }
  }

  // Sauvegarde + mise à jour du cache local → propagation réactive à tous les consumers
  async function saveStraddle(params: Partial<ParamsStraddle>) {
    const merged = { ...straddleRaw.value, ...params }
    await apiService.putStraddleParams(merged)
    straddleRaw.value = merged
  }

  async function saveSmc(params: Partial<ParamsSmc>) {
    const merged = { ...smcRaw.value, ...params }
    await apiService.putSmcParams(merged)
    smcRaw.value = merged
  }

  async function saveRockets(params: Partial<RocketsConfig>) {
    const merged = { ...rocketsRaw.value, ...params }
    await apiService.putRocketsConfig(merged)
    rocketsRaw.value = merged
  }

  return {
    straddleRaw, smcRaw, rocketsRaw,
    straddleParams, smcParams, rocketsConfig,
    isLoaded, loading,
    charger, saveStraddle, saveSmc, saveRockets,
  }
})
