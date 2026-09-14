<template>
  <AnalysePageShell
    titre="Analyse KDJ/Halftrend"
    retour-label="Stratégie KDJ/Halftrend"
    retour-route="/kdj"
    :synthese="[
      { label: 'Trades clôturés', valeur: kpi.nb },
      { label: 'Gagnants', valeur: kpi.wr, classe: kpi.wr !== '—' && parseFloat(kpi.wr) >= 50 ? 'text-emerald-400' : 'text-white' },
      { label: 'Σ R réalisé', valeur: kpi.sommeR, classe: kpi.classeR },
      { label: 'R moyen', valeur: kpi.rMoyen, classe: kpi.classeR },
    ]"
  >
    <div class="space-y-6">
      <!-- Verdicts -->
      <section>
        <h3 class="text-sm font-bold text-white mb-3">Verdicts</h3>
        <div class="grid grid-cols-2 md:grid-cols-4 gap-3">
          <div v-for="v in verdicts" :key="v.label" class="border border-white/10 bg-white/5 rounded-xl p-4 text-center">
            <div class="text-2xl font-bold" :class="v.classe">{{ v.nb }}</div>
            <div class="text-[11px] text-white mt-1">{{ v.label }}</div>
            <div class="text-[10px] font-mono mt-1" :class="v.classe">{{ v.r }}</div>
          </div>
        </div>
      </section>

      <!-- Par actif -->
      <section>
        <h3 class="text-sm font-bold text-white mb-3">Par actif</h3>
        <div class="overflow-x-auto border border-white/10 bg-white/5 rounded-xl">
          <table class="w-full text-xs">
            <thead>
              <tr class="text-white border-b border-white/10">
                <th class="text-left p-3">Actif</th>
                <th class="p-3">Trades</th>
                <th class="p-3">Gagnants</th>
                <th class="p-3">Σ R</th>
                <th class="p-3">R moyen</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="a in parAsset" :key="a.asset" class="border-b border-white/5">
                <td class="p-3 font-mono font-semibold text-white">{{ a.asset }}</td>
                <td class="p-3 text-center text-white">{{ a.nb }}</td>
                <td class="p-3 text-center text-white">{{ a.wr }}</td>
                <td class="p-3 text-center font-mono" :class="a.classeR">{{ a.sommeR }}</td>
                <td class="p-3 text-center font-mono" :class="a.classeR">{{ a.rMoyen }}</td>
              </tr>
              <tr v-if="!parAsset.length">
                <td colspan="5" class="p-6 text-center text-white text-xs">Aucun trade clôturé — la stratégie démarre (moteur H1 armé, état régi par le registre).</td>
              </tr>
            </tbody>
          </table>
        </div>
      </section>

      <!-- Référence -->
      <section class="border border-white/10 bg-white/5 rounded-xl p-4">
        <h3 class="text-sm font-bold text-white mb-2">Références du rejeu 24 mois (frais 0,05 %/ordre inclus)</h3>
        <p class="text-xs text-white leading-relaxed">
          XAUUSD H1 : +0,12→0,38 %/trade (PF 1,1-1,6) · XAGUSD : +0,19→0,66 (PF 1,1-1,5) ·
          DAX −0,3 (PF 0,6) · BTC ±0 · SP500 −0,1. Screening TV recoupé sur 5 actifs.
          Rappel métrologie : le R des entrées collées à l'EMA200 explose — le juge
          historique est le % du prix ; ici R = P&L / distance(entrée→EMA200).
        </p>
      </section>
    </div>
  </AnalysePageShell>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import AnalysePageShell from '@/components/common/AnalysePageShell.vue'
import { apiService } from '@/services/api.service'
import type { Signal } from '@/services/api.types'

const signaux = ref<Signal[]>([])

onMounted(async () => {
  try {
    const tous = await apiService.getSignaux(500)
    signaux.value = tous.filter((s) => s.strategie.toLowerCase() === 'kdj_halftrend')
  } catch { /* vide */ }
})

const clotures = computed(() => signaux.value.filter((s) => s.statut === 'Fermé' && s.r_realise !== null && s.r_realise !== undefined))

const fmt = (r: number) => `${r >= 0 ? '+' : ''}${r.toFixed(2)}R`
const classe = (r: number) => (r >= 0 ? 'text-emerald-400' : 'text-red-400')

const kpi = computed(() => {
  const rs = clotures.value.map((s) => s.r_realise ?? 0)
  const nb = rs.length
  if (!nb) return { nb: 0, wr: '—', sommeR: '—', rMoyen: '—', classeR: 'text-white' }
  const somme = rs.reduce((a, b) => a + b, 0)
  return {
    nb,
    wr: `${Math.round((rs.filter((r) => r > 0).length / nb) * 100)} %`,
    sommeR: fmt(somme),
    rMoyen: fmt(somme / nb),
    classeR: classe(somme),
  }
})

const verdicts = computed(() => {
  const familles = ['TP', 'SL', 'Retournement']
  return familles.map((f) => {
    const trades = clotures.value.filter((s) => (s.verdict ?? '').startsWith(f))
    const somme = trades.reduce((a, s) => a + (s.r_realise ?? 0), 0)
    return {
      label: f,
      nb: trades.length,
      r: trades.length ? fmt(somme) : '—',
      classe: f === 'SL' ? 'text-red-400' : f === 'TP' ? 'text-emerald-400' : 'text-amber-400',
    }
  })
})

const parAsset = computed(() => {
  const groupes = new Map<string, number[]>()
  for (const s of clotures.value) {
    const liste = groupes.get(s.asset) ?? []
    liste.push(s.r_realise ?? 0)
    groupes.set(s.asset, liste)
  }
  return [...groupes.entries()]
    .map(([asset, rs]) => {
      const somme = rs.reduce((a, b) => a + b, 0)
      return {
        asset,
        nb: rs.length,
        wr: `${Math.round((rs.filter((r) => r > 0).length / rs.length) * 100)} %`,
        sommeR: fmt(somme),
        rMoyen: fmt(somme / rs.length),
        classeR: classe(somme),
      }
    })
    .sort((a, b) => b.nb - a.nb)
})
</script>
