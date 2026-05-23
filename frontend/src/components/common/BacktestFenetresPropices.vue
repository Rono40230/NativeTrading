<template>
  <div class="glass-card rounded-xl border border-blue-500/20 bg-blue-500/5 p-3">
    <h3 class="text-xs font-semibold text-blue-300 mb-2">
      ⚡ Fenêtres propices Straddle
    </h3>

    <div v-if="!fenetres.length" class="text-[11px] text-gray-500 italic py-2">
      Aucune fenêtre propice détectée (min. 2 trades, win rate ≥ 50%)
    </div>

    <table v-else class="w-full text-xs">
      <thead>
        <tr class="text-gray-500 border-b border-white/10">
          <th class="text-left pb-1.5">Heure UTC</th>
          <th class="text-right pb-1.5">Trades</th>
          <th class="text-right pb-1.5">Win %</th>
          <th class="text-right pb-1.5">P&L R</th>
          <th class="text-left pb-1.5 pl-2">Événement type</th>
        </tr>
      </thead>
      <tbody>
        <tr
          v-for="f in fenetres"
          :key="f.heure"
          class="border-b border-white/5 hover:bg-white/5"
        >
          <td class="py-1.5 font-mono font-bold text-white">
            {{ String(f.heure).padStart(2, '0') }}h–{{ String(f.heure + 1).padStart(2, '0') }}h
          </td>
          <td class="text-right text-gray-400">{{ f.nb_trades }}</td>
          <td
            class="text-right font-semibold"
            :class="f.win_rate >= 0.6 ? 'text-emerald-400' : 'text-yellow-400'"
          >
            {{ (f.win_rate * 100).toFixed(0) }}%
          </td>
          <td
            class="text-right font-semibold"
            :class="f.pnl_r_total >= 0 ? 'text-emerald-400' : 'text-red-400'"
          >
            {{ f.pnl_r_total.toFixed(1) }}R
          </td>
          <td class="pl-2 text-[10px] text-gray-400">
            {{ f.evenement_type ?? '—' }}
          </td>
        </tr>
      </tbody>
    </table>

    <p class="text-[9px] text-gray-600 mt-2">
      Fenêtres triées par P&L total décroissant. Heure UTC. Croiser avec le calendrier pour confirmer.
    </p>
  </div>
</template>

<script setup lang="ts">
import type { FenetrePropice } from '@/services/api.backtest'

defineProps<{
  fenetres: FenetrePropice[]
}>()
</script>
