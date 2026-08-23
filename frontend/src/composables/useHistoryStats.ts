import { computed } from 'vue'
import type { Ref } from 'vue'
import type { Signal } from '@/services/api.service'

export function useHistoryStats(signaux: Ref<Signal[]>) {
  const smcStats = computed(() => {
    const smc = signaux.value.filter(
      s => ['SMC', 'SmcDirectional', 'SMC+IA'].includes(s.strategie)
    )
    const fermes = smc.filter(s => s.statut === 'Fermé')
    const gagnes = fermes.filter(s => s.verdict && ['TP1', 'TP2', 'TP3'].includes(s.verdict))
    const winrate = fermes.length > 0 ? Math.round((gagnes.length / fermes.length) * 100) : 0
    const avecConviction = smc.filter(s => s.llm_conviction != null)
    const convictionMoyenne =
      avecConviction.length > 0
        ? Math.round(
            avecConviction.reduce((acc, s) => acc + (s.llm_conviction ?? 0), 0) /
              avecConviction.length
          )
        : 0
    const total = smc.length
    const avecLlm = smc.filter(s => s.llm_valide != null).length
    const tauxFiltrage = total > 0 ? Math.round((avecLlm / total) * 100) : 0
    const longs = smc.filter(s => s.direction === 'Long').length
    const shorts = smc.filter(s => s.direction === 'Short').length
    const derniersLlm = smc.filter(s => s.llm_valide != null).slice(0, 5)
    return { total, winrate, convictionMoyenne, tauxFiltrage, longs, shorts, derniersLlm }
  })

  const straddleStats = computed(() => {
    const st = signaux.value.filter(s => s.strategie === 'Straddle')
    const fermes = st.filter(s => s.statut === 'Fermé')
    const gagnes = fermes.filter(s => s.verdict && ['TP1', 'TP2', 'TP3'].includes(s.verdict))
    const winrate = fermes.length > 0 ? Math.round((gagnes.length / fermes.length) * 100) : 0
    const actifs = st.filter(s => s.statut === 'Actif').length
    return { total: st.length, winrate, actifs }
  })

  return { smcStats, straddleStats }
}
