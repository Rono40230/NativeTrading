import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { apiService } from '@/services/api.service'
import type { StraddleParams } from '@/components/common/StraddleParamsPanel.vue'
// Forme API/DB (noms de champs = source de vérité backend) — alias pour éviter
// la collision avec le type UI StraddleParams (champs renommés pour l'IHM).
import type { StraddleParams as ParamsStraddle } from '@/generated/ParamsStraddle'

// Défauts mirroirs des impl Default Rust (db::strategies_params) — utilisés tant que
// l'API n'a pas encore répondu, pour que l'IHM reste non-null et réactive.
// (Les params fantômes SMC/Rockets ont été purgés le 09/09 : les réglages
// réels vivent dans la kv `configuration` et `rockets_params`.)
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

export const useStrategyParamsStore = defineStore('strategyParams', () => {
  // Données brutes (noms de champs DB/API)
  const straddleRaw = ref<ParamsStraddle>({ ...STRADDLE_PARAMS_DEFAUT })

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

  // Chargement unique — ne fait rien si déjà chargé
  async function charger() {
    if (isLoaded.value || loading.value) return
    loading.value = true
    try {
      straddleRaw.value = await apiService.getStraddleParams()
      isLoaded.value = true
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

  return {
    straddleRaw,
    straddleParams,
    isLoaded, loading,
    charger, saveStraddle,
  }
})
