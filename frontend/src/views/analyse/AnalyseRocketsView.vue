<template>
  <AnalysePageShell
    titre="🚀 Analyse Rockets"
    retour-label="Rockets"
    retour-route="/rockets"
  >
    <!-- ═══ RANGÉE 1 : dossier de décision | IA ═══ -->
    <div class="grid grid-cols-2 gap-4">
      <!-- 🎯 Dossier -->
      <section class="rounded-xl border border-blue-500/30 bg-blue-950/20 p-4 flex flex-col gap-4">
        <h2 class="text-sm font-bold text-blue-300 uppercase tracking-wider">🎯 Le dossier de décision</h2>

        <div class="grid grid-cols-4 gap-3">
          <div class="text-center">
            <div class="text-3xl font-bold text-white">{{ stats.total }}</div>
            <div class="text-[10px] uppercase tracking-wide text-white mt-1">clôturés</div>
          </div>
          <div class="text-center">
            <div class="text-3xl font-bold" :class="stats.tauxGagnants >= 50 ? 'text-emerald-400' : 'text-red-400'">{{ stats.tauxGagnants }}%</div>
            <div class="text-[10px] uppercase tracking-wide text-white mt-1">win rate</div>
          </div>
          <div class="text-center">
            <div class="text-3xl font-bold" :class="stats.rMoyen >= 0 ? 'text-emerald-400' : 'text-red-400'">{{ stats.rMoyen >= 0 ? '+' : '' }}{{ stats.rMoyen }}R</div>
            <div class="text-[10px] uppercase tracking-wide text-white mt-1">R moyen</div>
          </div>
          <div class="text-center">
            <div class="text-3xl font-bold text-red-400">{{ stats.tauxSL }}%</div>
            <div class="text-[10px] uppercase tracking-wide text-white mt-1">loss rate</div>
          </div>
        </div>

        <!-- LA dimension rockets : par univers -->
        <table v-if="parUnivers.length" class="w-full text-sm border-collapse">
          <thead>
            <tr class="text-white border-b border-white/15">
              <th class="py-2 text-left text-xs uppercase tracking-wide">Univers</th>
              <th class="py-2 text-right text-xs uppercase tracking-wide">Trades</th>
              <th class="py-2 text-right text-xs uppercase tracking-wide">ΣR</th>
              <th class="py-2 text-right text-xs uppercase tracking-wide">Win rate</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="u in parUnivers" :key="u.label" class="border-b border-white/5">
              <td class="py-2 text-white">
                <span class="font-semibold px-1.5 py-0.5 rounded-full"
                      :class="u.label === 'crypto' ? 'bg-amber-900/60 text-amber-300' : 'bg-blue-900/60 text-blue-300'"
                >{{ u.label === 'crypto' ? 'Crypto' : 'Actions US' }}</span>
              </td>
              <td class="py-2 text-right text-white">{{ u.total }}</td>
              <td class="py-2 text-right text-lg font-bold" :class="u.rSomme >= 0 ? 'text-emerald-400' : 'text-red-400'">{{ u.rSomme >= 0 ? '+' : '' }}{{ u.rSomme }}R</td>
              <td class="py-2 text-right" :class="u.winPct >= 50 ? 'text-emerald-400' : 'text-red-400'">{{ u.winPct }}%</td>
            </tr>
          </tbody>
        </table>

        <!-- Verdicts -->
        <div class="flex flex-wrap gap-2">
          <span v-for="v in verdictsChips" :key="v.label"
                class="text-xs font-semibold px-2 py-1 rounded-full border"
                :class="v.label === 'SL' ? 'bg-red-500/10 text-red-400 border-red-500/30' : v.label === 'Manuel' ? 'bg-blue-500/10 text-blue-300 border-blue-500/30' : 'bg-emerald-500/10 text-emerald-400 border-emerald-500/30'"
          >{{ v.label === 'TS' ? '🏁' : v.label === 'Manuel' ? '👤' : v.label === 'Expire' ? '⌛' : '❌' }} {{ v.label }} × {{ v.n }}</span>
          <span v-if="!stats.total" class="text-xs text-white">Aucun trade clôturé — la verticale est jeune.</span>
        </div>
      </section>

      <!-- 🤖 Recommandations IA -->
      <section class="rounded-xl border border-purple-500/25 bg-purple-950/10 p-4 flex flex-col">
        <h2 class="text-sm font-bold text-purple-300 uppercase tracking-wider mb-3">🤖 Recommandations IA</h2>
        <div class="flex-1 min-h-0 overflow-y-auto">
          <RocketsAnalyseLlm />
        </div>
      </section>
    </div>

    <!-- (Les réglages fantômes « scan » ont été purgés le 09/09 : les réglages
         réels vivent dans la modale « Paramètres moteur » de la carte Rockets
         du dashboard — table rockets_params.) -->
  </AnalysePageShell>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { chargerAnalyse, type AnalyseStrategie } from '@/composables/useAnalyses'
import AnalysePageShell from '@/components/common/AnalysePageShell.vue'
import RocketsAnalyseLlm from '@/components/common/RocketsAnalyseLlm.vue'

// HARMONISATION 15/09 : mêmes chiffres que la carte et le rapport d'activité
// (/api/analyses/rockets — base vécue, R-distance, WR $).
const analyse = ref<AnalyseStrategie | null>(null)
onMounted(async () => {
  analyse.value = await chargerAnalyse('rockets')
})

const stats = computed(() => ({
  total: analyse.value?.nb_trades ?? 0,
  tauxGagnants: Math.round((analyse.value?.taux_reussite ?? 0) * 100),
  rMoyen: Math.round((analyse.value?.r_moyen ?? 0) * 100) / 100,
  tauxSL: Math.round((analyse.value?.taux_perte ?? 0) * 100),
}))

/// Par univers (crypto USDT vs actions US) — regroupement des assets de
/// l'analyse (n, ΣR distance, WR $ pondéré).
const parUnivers = computed(() => {
  const groupes = new Map<string, { n: number; r: number; gagnants: number }>()
  for (const c of analyse.value?.assets ?? []) {
    const u = c.label.endsWith('USDT') ? 'crypto' : 'action'
    const e = groupes.get(u) ?? { n: 0, r: 0, gagnants: 0 }
    e.n += c.n
    e.r += c.r
    e.gagnants += c.wr * c.n
    groupes.set(u, e)
  }
  return [...groupes.entries()]
    .map(([label, e]) => ({
      label,
      total: e.n,
      rSomme: parseFloat(e.r.toFixed(2)),
      winPct: e.n > 0 ? Math.round((e.gagnants / e.n) * 100) : 0,
    }))
    .sort((a, b) => b.rSomme - a.rSomme)
})

const verdictsChips = computed(() =>
  (analyse.value?.verdicts ?? []).map(v => ({ label: v.label, n: v.n })),
)
</script>

<style scoped>
.kpi-card    { @apply bg-white/5 rounded-lg p-3 border border-white/10; }
</style>
