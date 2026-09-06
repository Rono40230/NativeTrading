<template>
  <div v-if="open" class="fixed inset-0 z-50 flex items-center justify-center bg-black/70" @click.self="$emit('close')">
    <div class="rounded-xl border border-white/10 p-5 w-[96vw] h-[92vh] flex flex-col gap-3" style="background: #0d1117;">

      <!-- Header -->
      <div class="flex items-center justify-between flex-shrink-0">
        <h2 class="text-lg font-bold">📊 Analyse Rockets</h2>
        <button class="text-white hover:text-white text-xl leading-none" @click="$emit('close')">×</button>
      </div>

      <!-- Onglets -->
      <div class="flex gap-1 flex-shrink-0">
        <button
          v-for="tab in TABS" :key="tab.id"
          class="px-4 py-1.5 rounded-lg text-xs font-semibold transition-all border"
          :class="onglet === tab.id ? 'bg-white/10 border-white/20 text-white' : 'border-transparent text-white hover:text-white'"
          @click="onglet = tab.id as 'perf' | 'ia' | 'reglages'"
        >{{ tab.label }}</button>
      </div>

      <!-- Onglet Recommandations IA -->
      <div v-if="onglet === 'ia'" class="flex-1 overflow-auto">
        <RocketsAnalyseLlm />
      </div>

      <!-- Onglet Réglages -->
      <div v-if="onglet === 'reglages'" class="flex-1 overflow-auto">
        <RocketsReglages />
      </div>

      <!-- KPIs (onglet performance) -->
      <div v-if="onglet === 'perf'" class="grid grid-cols-5 gap-3 flex-shrink-0">
        <div class="kpi-card text-center">
          <div class="text-xl font-bold text-white">{{ stats.total }}</div>
          <div class="text-xs text-white mt-0.5">Total clôturés</div>
        </div>
        <div class="kpi-card text-center">
          <div class="text-xl font-bold text-emerald-400">{{ stats.tauxGagnants }}%</div>
          <div class="text-xs text-white mt-0.5">Win rate</div>
        </div>
        <div class="kpi-card text-center">
          <div class="text-xl font-bold" :class="stats.rMoyen >= 0 ? 'text-emerald-400' : 'text-red-400'">{{ stats.rMoyen }}R</div>
          <div class="text-xs text-white mt-0.5">R moyen</div>
        </div>
        <div class="kpi-card text-center">
          <div class="text-base font-bold">
            <span class="text-emerald-400">{{ stats.gagnants }}</span>
            <span class="text-white mx-1">/</span>
            <span class="text-red-400">{{ stats.perdants }}</span>
          </div>
          <div class="text-xs text-white mt-0.5">Gagnants / perdants</div>
        </div>
        <div class="kpi-card text-center">
          <div class="text-xl font-bold text-red-400">{{ stats.tauxSL }}%</div>
          <div class="text-xs text-white mt-0.5">Loss rate réel</div>
        </div>
      </div>

      <!-- Contenu : 2 colonnes (onglet performance) -->
      <div v-if="onglet === 'perf'" class="grid grid-cols-[1fr_1.8fr] gap-4 flex-1 min-h-0">

        <!-- Gauche : par univers + verdicts -->
        <div class="flex flex-col gap-4 min-h-0 overflow-auto pr-1">
          <div>
            <h3 class="section-title">Par univers</h3>
            <div class="grid grid-cols-2 gap-2">
              <div v-for="u in parUnivers" :key="u.label" class="kpi-card">
                <div class="flex justify-between mb-1">
                  <span class="text-xs font-bold px-1.5 py-0.5 rounded-full"
                    :class="u.label === 'crypto' ? 'bg-amber-900/60 text-amber-300' : 'bg-blue-900/60 text-blue-300'"
                  >{{ u.label === 'crypto' ? 'Crypto' : 'Actions US' }}</span>
                  <span class="text-white text-xs">{{ u.total }}</span>
                </div>
                <div class="text-xs">Win : <span class="font-bold" :class="u.winPct >= 50 ? 'text-emerald-400' : 'text-red-400'">{{ u.winPct }}%</span></div>
                <div class="text-xs">ΣR : <span class="font-bold" :class="u.rSomme >= 0 ? 'text-emerald-400' : 'text-red-400'">{{ u.rSomme }}</span></div>
              </div>
            </div>
          </div>

          <div>
            <h3 class="section-title">Verdicts</h3>
            <div class="flex flex-wrap gap-2">
              <span v-for="(n, v) in stats.verdicts" :key="v"
                class="text-xs font-semibold px-2 py-1 rounded-full border"
                :class="v === 'SL' ? 'bg-red-500/10 text-red-400 border-red-500/30' : 'bg-emerald-500/10 text-emerald-400 border-emerald-500/30'"
              >{{ v }} × {{ n }}</span>
              <span v-if="!stats.total" class="text-xs text-white">Aucun trade clôturé — la verticale est jeune.</span>
            </div>
          </div>

          <!-- Analyse du tableau de probabilités -->
          <div class="kpi-card space-y-2">
            <h3 class="section-title">🔍 Interprétation</h3>
            <div v-if="lossRateReel === 0" class="text-xs text-white italic">
              Aucune perte enregistrée — impossible de calculer un loss rate réel.
            </div>
            <template v-else>
              <p class="text-xs leading-relaxed">
                Avec un loss rate réel de
                <span class="font-bold text-blue-400">{{ lossRateReel }}%</span>
                sur <span class="font-bold text-white">{{ sampleSize }}</span> trades,
                la probabilité de subir au moins
                <span class="font-bold" :class="analyseProba.kCritique50 <= 3 ? 'text-red-400' : 'text-yellow-400'">{{ analyseProba.kCritique50 }} SL consécutifs</span>
                dépasse <span class="font-bold">50%</span>
                — soit un scénario <span :class="analyseProba.kCritique50 <= 2 ? 'text-red-400 font-bold' : 'text-yellow-300 font-bold'">
                  {{ analyseProba.kCritique50 <= 2 ? 'très probable' : analyseProba.kCritique50 <= 4 ? 'probable' : 'possible' }}
                </span>.
              </p>
              <p class="text-xs leading-relaxed">
                <span class="font-bold text-white">⚠️ Zone danger</span> :
                série de
                <span class="font-bold text-red-400">{{ analyseProba.kDanger }}+ SL</span>
                avec une probabilité
                <span class="font-bold text-red-400">{{ analyseProba.probAuKDanger }}%</span>.
                Prévoir suffisamment de capital pour absorber cette série sans modifier la stratégie.
              </p>
              <p class="text-xs leading-relaxed">
                <span class="font-bold text-emerald-400">✅ Zone sûreté</span> :
                une série de
                <span class="font-bold text-emerald-400">{{ analyseProba.kSurete }}+ SL consécutifs</span>
                reste statistiquement très rare (&lt;5%) — ce n'est qu'alors qu'un remise en question de la stratégie est justifiée.
              </p>
              <div class="mt-2 pt-2 border-t border-white/10 text-xs flex gap-4">
                <div>
                  <span class="text-white">Espérance math.</span><br>
                  <span class="font-bold" :class="analyseProba.esperance >= 0 ? 'text-emerald-400' : 'text-red-400'">{{ analyseProba.esperance }}R / trade</span>
                </div>
                <div>
                  <span class="text-white">Drawdown max estimé</span><br>
                  <span class="font-bold text-orange-400">-{{ analyseProba.kDanger }}R (à risque constant)</span>
                </div>
              </div>
            </template>
          </div>
        </div>

        <!-- Droite : tableau probabilités -->
        <div class="flex flex-col min-h-0">
          <div class="flex items-baseline gap-3 mb-1 flex-shrink-0">
            <h3 class="section-title">Probabilité de séries de SL consécutifs</h3>
            <span class="text-xs text-white">sur {{ sampleSize }} trades clôturés</span>
          </div>
          <p class="text-xs text-white mb-2 flex-shrink-0">
            Ligne <span class="text-blue-400 font-bold">surlignée</span> = votre loss rate réel ({{ lossRateReel }}%).
            Colonnes = nombre de SL consécutifs. Vert = très probable, rouge = rare.
          </p>
          <div class="overflow-auto flex-1 rounded-lg">
            <table class="text-xs border-collapse w-full">
              <thead class="sticky top-0" style="background: #0d1117">
                <tr>
                  <th class="px-3 py-1.5 text-left text-white font-medium">Loss %</th>
                  <th v-for="k in kValues" :key="k" class="px-3 py-1.5 text-center text-white font-medium">{{ k }}</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="row in tableauPertes" :key="row.lossRate">
                  <td class="px-3 py-1 font-mono font-bold sticky left-0"
                      :style="row.isActual ? 'background:#1e3a5f; color:#93c5fd' : 'background:#0d1117; color:#ffffff'">
                    {{ row.lossRate }}%
                  </td>
                  <td v-for="(pct, ki) in row.probs" :key="ki"
                      class="px-3 py-1 text-center font-bold"
                      :style="{ background: couleurProba(pct), color: pct > 15 ? '#fff' : '#6b7280' }">
                    {{ pct }}%
                  </td>
                </tr>
              </tbody>
            </table>
          </div>
        </div>
      </div>

    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { apiService } from '@/services/api.service'
import type { Signal } from '@/services/api.service'
import RocketsAnalyseLlm from '@/components/common/RocketsAnalyseLlm.vue'
import RocketsReglages from '@/components/common/RocketsReglages.vue'
import { useRocketsStats } from '@/composables/useRocketsStats'

const TABS = [
  { id: 'perf',     label: '📊 Performance' },
  { id: 'ia',       label: '🤖 Recommandations IA' },
  { id: 'reglages', label: '⚙️ Réglages scan' },
]
const onglet = ref<'perf' | 'ia' | 'reglages'>('perf')

const props = defineProps<{ open: boolean }>()
defineEmits(['close'])

// V2 (05/09) : trades officiels depuis la table partagée — plus de
// props.rockets v1 (table vide) ni d'historiqueRockets.
const cloutes = ref<Signal[]>([])
watch(() => props.open, async (ouvert) => {
  if (!ouvert) return
  try {
    const data = await apiService.getSignaux(500)
    cloutes.value = data.filter(s =>
      s.strategie.toLowerCase() === 'rockets'
      && s.statut === 'Fermé' && s.verdict !== null,
    )
  } catch {
    cloutes.value = []
  }
})

const {
  stats, parUnivers,
  kValues, sampleSize, lossRateReel,
  tableauPertes, analyseProba, couleurProba,
} = useRocketsStats(computed(() => cloutes.value))
</script>

<style scoped>
.kpi-card    { @apply bg-white/5 rounded-lg p-3 border border-white/10; }
.section-title { @apply text-xs font-semibold text-white mb-2 uppercase tracking-wide; }
</style>
