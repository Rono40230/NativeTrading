<template>
  <div class="overflow-x-auto rounded-xl border border-white/10 mb-5">
    <table class="w-full text-[11px]">
      <thead>
        <tr class="border-b border-white/10 bg-white/[0.03] text-[9px] text-gray-500 uppercase tracking-widest font-semibold">
          <th class="text-left px-4 py-3 text-sm">Phase</th>
          <th class="text-left px-4 py-3 text-sm">Ticker</th>
          <th class="text-left px-4 py-3 text-sm w-44">Tendance</th>
          <th class="text-right px-4 py-3 text-sm text-amber-400">E. Limite</th>
          <th class="text-right px-4 py-3 text-sm text-sky-400">E. Stop</th>
          <th class="text-right px-4 py-3 text-sm text-purple-400">E. Idéale</th>
          <th class="text-right px-4 py-3 text-sm text-red-400">Invalidation</th>
          <th class="text-right px-4 py-3 text-sm text-red-500">SL</th>
          <th class="text-right px-4 py-3 text-sm text-emerald-500">TP1 (1.5R)</th>
          <th class="text-right px-4 py-3 text-sm text-emerald-500">TP2 (2.5R)</th>
          <th class="text-right px-4 py-3 text-sm text-emerald-400">Trail (3.5R)</th>
          <th class="text-right px-4 py-3 text-sm text-blue-400">Coef trail.</th>
          <th class="text-right px-4 py-3 text-sm">Score</th>
        </tr>
      </thead>
      <tbody>
        <tr
          v-for="s in signaux"
          :key="s.symbol"
          class="border-b border-white/5 hover:bg-white/[0.03] transition-colors"
        >
          <td class="px-4 py-3 text-xs">
            {{ icone(s.phase) }} <span class="text-gray-400">{{ labelPhase(s.phase) }}</span>
          </td>
          <td class="px-4 py-3 text-sm font-bold text-white">{{ s.ticker }}</td>
          <td class="px-4 py-3">
            <svg viewBox="0 0 160 52" style="width:160px;height:52px">
              <polyline
                v-if="s.closes.length >= 2"
                :points="sparklinePath(s.closes)"
                fill="none"
                :stroke="s.change1h >= 0 ? '#10b981' : '#ef4444'"
                stroke-width="1.5"
                stroke-linejoin="round"
                stroke-linecap="round"
              />
              <text v-else x="80" y="28" text-anchor="middle" fill="#374151" font-size="9">…</text>
            </svg>
          </td>
          <!-- Entrée limite -->
          <td class="px-4 py-3 text-right">
            <span
              class="font-mono text-sm"
              :class="s.typeEntreeRec === 'limite' ? 'text-amber-300 font-bold' : 'text-amber-600'"
            >{{ formatPrix(s.entreeLimite) }}</span>
            <span v-if="s.typeEntreeRec === 'limite'" class="ml-1 text-[8px] text-amber-400">★</span>
          </td>
          <!-- Entrée stop -->
          <td class="px-4 py-3 text-right">
            <span
              class="font-mono text-sm"
              :class="s.typeEntreeRec === 'stop' ? 'text-sky-300 font-bold' : 'text-sky-600'"
            >{{ formatPrix(s.entreeStop) }}</span>
            <span v-if="s.typeEntreeRec === 'stop'" class="ml-1 text-[8px] text-sky-400">★</span>
          </td>
          <!-- Entrée idéale (recommandée algo) -->
          <td class="px-4 py-3 text-right">
            <span
              class="text-[9px] font-semibold px-2 py-0.5 rounded-full"
              :class="s.typeEntreeRec === 'stop'
                ? 'bg-sky-500/20 text-sky-300'
                : 'bg-amber-500/20 text-amber-300'"
            >{{ s.typeEntreeRec === 'stop' ? '⚡ Stop' : '⏳ Limite' }}</span>
          </td>
          <!-- Niveau invalidation -->
          <td class="px-4 py-3 text-right font-mono text-sm text-red-300/70">
            {{ formatPrix(s.niveauInvalidation) }}
          </td>
          <td class="px-4 py-3 text-right font-mono text-sm text-red-400">{{ formatPrix(s.sl) }}</td>
          <td class="px-4 py-3 text-right font-mono text-sm text-emerald-400">{{ formatPrix(s.tp1) }}</td>
          <td class="px-4 py-3 text-right font-mono text-sm text-emerald-400">{{ formatPrix(s.tp2) }}</td>
          <td class="px-4 py-3 text-right font-mono text-sm text-emerald-300">{{ formatPrix(s.tp3Trigger) }}</td>
          <td class="px-4 py-3 text-right">
            <span class="text-sm font-bold font-mono" :class="classeCoefTrailing(s.trailingCoeff)">
              {{ s.trailingCoeff?.toFixed(1) ?? '—' }}×
            </span>
          </td>
          <td class="px-4 py-3 text-right">
            <span class="text-sm font-bold" :class="classeScore(s.score)">{{ s.score }}</span>
            <span class="text-xs text-gray-600">/100</span>
          </td>
        </tr>
      </tbody>
    </table>
  </div>
</template>

<script setup lang="ts">
import type { SignalRocket } from '@/composables/useVeilleRockets'
import { useRocketsHelpers } from '@/composables/useRocketsHelpers'

defineProps<{ signaux: SignalRocket[] }>()

const {
  icone, labelPhase, classeScore, classeCoefTrailing,
  formatPrix, sparklinePath,
} = useRocketsHelpers()
</script>
