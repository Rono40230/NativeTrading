<template>
  <!-- KPIs -->
  <div class="grid grid-cols-5 gap-3 flex-shrink-0">
    <div class="kpi-card text-center">
      <div class="text-xl font-bold text-white">{{ stats.total }}</div>
      <div class="text-xs text-white mt-0.5">Total clôturés</div>
    </div>
    <div class="kpi-card text-center">
      <div class="text-xl font-bold text-emerald-400">{{ stats.winPct }}%</div>
      <div class="text-xs text-white mt-0.5">Win rate</div>
    </div>
    <div class="kpi-card text-center">
      <div class="text-xl font-bold" :class="stats.rMoyen >= 0 ? 'text-emerald-400' : 'text-red-400'">
        {{ stats.rMoyen }}R
      </div>
      <div class="text-xs text-white mt-0.5">R moyen</div>
    </div>
    <div class="kpi-card text-center">
      <div class="text-base font-bold">
        <span class="text-emerald-400">{{ stats.gain }}</span>
        <span class="text-white mx-1">/</span>
        <span class="text-red-400">{{ stats.sl }}</span>
      </div>
      <div class="text-xs text-white mt-0.5">Ratio trades</div>
    </div>
    <div class="kpi-card text-center">
      <div class="text-xl font-bold text-red-400">{{ stats.tauxSL }}%</div>
      <div class="text-xs text-white mt-0.5">Loss rate réel</div>
    </div>
  </div>

  <!-- Tranches + répartition -->
  <div class="grid grid-cols-[1fr_1.8fr] gap-4 flex-1 min-h-0 mt-3">
    <!-- Gauche -->
    <div class="flex flex-col gap-4 min-h-0 overflow-auto pr-1">
      <slot name="gauche" />

      <!-- Interprétation probabiliste -->
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
            <span class="font-bold" :class="analyseProba.kCritique50 <= 3 ? 'text-red-400' : 'text-yellow-400'">
              {{ analyseProba.kCritique50 }} SL consécutifs
            </span>
            dépasse <span class="font-bold">50%</span> —
            soit un scénario
            <span :class="analyseProba.kCritique50 <= 3 ? 'text-red-400 font-bold' : 'text-yellow-300 font-bold'">
              {{ analyseProba.kCritique50 <= 2 ? 'très probable' : analyseProba.kCritique50 <= 4 ? 'probable' : 'possible' }}
            </span>.
          </p>
          <p class="text-xs leading-relaxed">
            <span class="font-bold text-white">⚠️ Zone danger</span> :
            série de
            <span class="font-bold text-red-400">{{ analyseProba.kDanger }}+ SL</span>
            avec une probabilité
            <span class="font-bold text-red-400">{{ analyseProba.probAuKDanger }}%</span>.
          </p>
          <p class="text-xs leading-relaxed">
            <span class="font-bold text-emerald-400">✅ Zone sûreté</span> :
            série de
            <span class="font-bold text-emerald-400">{{ analyseProba.kSurete }}+ SL consécutifs</span>
            reste statistiquement très rare (&lt;5%).
          </p>
          <div class="mt-2 pt-2 border-t border-white/10 text-xs flex gap-4">
            <div>
              <span class="text-white">Espérance math.</span><br>
              <span class="font-bold" :class="analyseProba.esperance >= 0 ? 'text-emerald-400' : 'text-red-400'">
                {{ analyseProba.esperance }}R / trade
              </span>
            </div>
            <div>
              <span class="text-white">Drawdown max estimé</span><br>
              <span class="font-bold text-orange-400">-{{ analyseProba.kDanger }}R (à risque constant)</span>
            </div>
          </div>
        </template>
      </div>
    </div>

    <!-- Droite : heatmap -->
    <div class="flex flex-col min-h-0">
      <div class="flex items-baseline gap-3 mb-1 flex-shrink-0">
        <h3 class="section-title">Probabilité de séries de SL consécutifs</h3>
        <span class="text-xs text-white">sur {{ sampleSize }} trades clôturés</span>
      </div>
      <p class="text-xs text-white mb-2 flex-shrink-0">
        Ligne <span class="text-blue-400 font-bold">surlignée</span> = votre loss rate réel ({{ lossRateReel }}%).
        Colonnes = nombre de SL consécutifs.
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
</template>

<script setup lang="ts">
import { couleurProba } from '@/composables/useProbaHeatmap'

defineProps<{
  stats: {
    total: number; winPct: number; rMoyen: number
    gain: number; sl: number; tauxSL: number
  }
  tranches: unknown[]
  lossRateReel: number
  sampleSize: number
  kValues: number[]
  tableauPertes: { lossRate: number; isActual: boolean; probs: number[] }[]
  analyseProba: {
    kCritique50: number; kDanger: number; probAuKDanger: number
    kSurete: number; esperance: number
  }
}>()
</script>

<style scoped>
.kpi-card     { @apply bg-white/5 rounded-lg p-3 border border-white/10; }
.section-title { @apply text-xs font-semibold text-white mb-2 uppercase tracking-wide; }
</style>
