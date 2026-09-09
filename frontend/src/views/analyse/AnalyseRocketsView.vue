<template>
  <AnalysePageShell
    titre="🚀 Analyse Rockets"
    retour-label="Rockets"
    retour-route="/rockets"
    :synthese="bandeau"
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
          <span v-for="(n, v) in stats.verdicts" :key="v"
                class="text-xs font-semibold px-2 py-1 rounded-full border"
                :class="v === 'SL' ? 'bg-red-500/10 text-red-400 border-red-500/30' : v === 'Manuel' ? 'bg-blue-500/10 text-blue-300 border-blue-500/30' : 'bg-emerald-500/10 text-emerald-400 border-emerald-500/30'"
          >{{ v === 'TS' ? '🏁' : v === 'Manuel' ? '👤' : '❌' }} {{ v }} × {{ n }}</span>
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

    <!-- ═══ RÉGLAGES (pleine largeur — heatmap séries SL retirée le 09/09 :
         ~5 trades clôturés = loss rate non significatif, même verdict que
         le straddle ; elle reviendra via le composant repliable le jour où
         l'effectif le justifiera) ═══ -->
    <section class="rounded-xl border border-white/10 bg-white/[0.03] p-4 flex flex-col gap-3">
      <div class="flex items-center gap-3">
        <span class="text-sm font-bold text-white uppercase tracking-wider">⚙️ Réglages scan</span>
        <span class="ml-auto text-xs text-white">paramètres du scanner</span>
      </div>
      <RocketsReglages />
    </section>
  </AnalysePageShell>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { apiService } from '@/services/api.service'
import type { Signal } from '@/services/api.service'
import { useRocketsStats } from '@/composables/useRocketsStats'
import AnalysePageShell from '@/components/common/AnalysePageShell.vue'
import RocketsAnalyseLlm from '@/components/common/RocketsAnalyseLlm.vue'
import RocketsReglages from '@/components/common/RocketsReglages.vue'

const clotes = ref<Signal[]>([])
onMounted(async () => {
  try {
    const data = await apiService.getSignaux(500)
    clotes.value = data.filter(s =>
      s.strategie?.toLowerCase() === 'rockets'
      && s.statut === 'Fermé' && s.verdict !== null,
    )
  } catch { clotes.value = [] }
})

const { stats, parUnivers } = useRocketsStats(computed(() => clotes.value))

const bandeau = computed(() => [
  { label: 'clôturés', valeur: stats.value.total },
  { label: 'win rate', valeur: `${stats.value.tauxGagnants}%`, classe: stats.value.tauxGagnants >= 50 ? 'text-emerald-400' : 'text-red-400' },
  { label: 'R moyen', valeur: `${stats.value.rMoyen}R`, classe: stats.value.rMoyen >= 0 ? 'text-emerald-400' : 'text-red-400' },
  { label: 'loss rate', valeur: `${stats.value.tauxSL}%`, classe: 'text-red-400' },
])
</script>

<style scoped>
.kpi-card    { @apply bg-white/5 rounded-lg p-3 border border-white/10; }
</style>
