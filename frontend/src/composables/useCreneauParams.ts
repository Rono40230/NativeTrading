import { ref } from 'vue'
import { useRoute } from 'vue-router'

export interface ParamsCreneau {
  asset: string
  timing_optimal: string
  jour_semaine: number | null
  id: number | null
}

/**
 * Lit les query params injectés par StraddleView ("🧪 Tester").
 * Nécessite `?asset=...&timing=HH:MM&jour=...`.
 * Retourne `modeCreneau` (null si navigation directe) et `creneauApi()`
 * qui formate les params pour l'appel `runBacktest`.
 */
export function useCreneauParams() {
  const route = useRoute()
  const modeCreneau = ref<ParamsCreneau | null>(null)

  const { asset, timing, jour, id } = route.query
  if (asset && timing && String(timing) !== '') {
    modeCreneau.value = {
      asset: String(asset),
      timing_optimal: String(timing),
      jour_semaine: jour && String(jour) !== '' ? Number(jour) : null,
      id: id && String(id) !== '' ? Number(id) : null,
    }
  }

  function creneauApi(): { timing_optimal: string; jour_semaine: number | null } | undefined {
    if (!modeCreneau.value) return undefined
    return {
      timing_optimal: modeCreneau.value.timing_optimal,
      jour_semaine: modeCreneau.value.jour_semaine,
    }
  }

  return { modeCreneau, creneauApi }
}
