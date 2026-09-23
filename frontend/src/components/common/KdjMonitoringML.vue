<template>
  <div class="space-y-4">
    <!-- Métriques globales (échantillons ml_training_samples — cf. note
         conventions dans MetriquesMLView) -->
    <template v-if="monitoring">
      <div v-if="props.compact">
        <p class="text-[9px] text-white mb-2">{{ monitoring.nb_signals_total }} signaux · {{
          monitoring.nb_feedbacks_clotures }} clôturés</p>
        <div class="grid grid-cols-2 gap-1.5 mb-3">
          <div class="rounded-md border border-emerald-500/20 bg-emerald-900/10 px-2 py-1.5 flex flex-col gap-0.5">
            <span class="text-[9px] text-emerald-600 uppercase tracking-wider">✅ Gagnants</span>
            <span class="text-base font-bold text-emerald-400">{{ monitoring.nb_gagnants }}</span>
          </div>
          <div class="rounded-md border border-red-500/20 bg-red-900/10 px-2 py-1.5 flex flex-col gap-0.5">
            <span class="text-[9px] text-red-600 uppercase tracking-wider">❌ Perdants</span>
            <span class="text-base font-bold text-red-400">{{ monitoring.nb_perdants }}</span>
          </div>
          <div class="rounded-md border border-white/10 bg-white/5 px-2 py-1.5 flex flex-col gap-0.5">
            <span class="text-[9px] text-white uppercase tracking-wider">Win Rate</span>
            <span class="text-base font-bold"
              :class="monitoring.win_rate_global >= 0.55 ? 'text-emerald-400' : monitoring.win_rate_global >= 0.45 ? 'text-yellow-400' : 'text-red-400'">{{
                pct(monitoring.win_rate_global) }}</span>
          </div>
          <div class="rounded-md border border-white/10 bg-white/5 px-2 py-1.5 flex flex-col gap-0.5">
            <span class="text-[9px] text-white uppercase tracking-wider">R moyen/signal</span>
            <span class="text-base font-bold"
              :class="(monitoring.pnl_moyen_r ?? 0) >= 0 ? 'text-emerald-400' : 'text-red-400'">{{
                monitoring.pnl_moyen_r != null ? monitoring.pnl_moyen_r.toFixed(2) + 'R' : '—' }}</span>
          </div>
        </div>
      </div>
    </template>
    <div v-else-if="chargement" class="text-center text-xs text-white py-4 animate-pulse">Chargement stats…</div>
    <div v-else class="text-center text-xs text-white py-4 leading-relaxed">
      KDJ armée le 22/09 — aucun trade clôturé pour l'instant.<br>
      Les stats ML apparaîtront après les premières clôtures.
    </div>

    <!-- Performance par verdict (tp1/tp2/sl/be…) -->
    <div v-if="props.compact && monitoring?.par_categorie?.length">
      <p class="text-[10px] font-semibold uppercase tracking-wider text-white mb-2">Par verdict</p>
      <div class="overflow-x-auto">
        <table class="w-full text-[10px]">
          <thead>
            <tr class="text-white border-b border-white/10">
              <th class="pb-1.5 text-left pr-3">Verdict</th>
              <th class="pb-1.5 text-right pr-3">Trades</th>
              <th class="pb-1.5 text-right pr-3">Win Rate</th>
              <th class="pb-1.5 text-right">R moy</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="(c, i) in monitoring.par_categorie" :key="i" class="border-b border-white/5 hover:bg-white/5">
              <td class="py-1.5 pr-3">
                <span class="text-[10px] px-1.5 py-0.5 rounded font-semibold bg-gray-800 text-white">{{ labelVerdict(c.categorie) }}</span>
              </td>
              <td class="py-1.5 pr-3 text-right text-white">{{ c.nb_trades }}</td>
              <td class="py-1.5 pr-3 text-right font-semibold"
                :class="c.win_rate >= 0.55 ? 'text-emerald-400' : c.win_rate >= 0.45 ? 'text-yellow-400' : 'text-red-400'">
                {{ pct(c.win_rate) }}
              </td>
              <td class="py-1.5 text-right" :class="(c.pnl_r_moyen ?? 0) >= 0 ? 'text-emerald-400' : 'text-red-400'">
                {{ c.pnl_r_moyen != null ? c.pnl_r_moyen.toFixed(2) + 'R' : '—' }}
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>

    <div v-if="derniereMaj" class="pt-1 flex justify-between text-[10px] text-white">
      <button class="hover:text-white transition" @click="charger">↻ Actualiser</button>
      <span>MAJ {{ formatHeure(derniereMaj) }}</span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { http } from '@/services/http.client'
import { formatParis } from '@/utils/date'
import type { StraddleMonitoringData } from '@/services/api.types'

const props = withDefaults(defineProps<{ compact?: boolean }>(), { compact: false })
const monitoring = ref<StraddleMonitoringData | null>(null)
const chargement = ref(false)
const derniereMaj = ref<number | null>(null)

function pct(v: number): string {
  return `${(v * 100).toFixed(1)}%`
}

function formatHeure(ts: number): string {
  return formatParis(new Date(ts), { hour: '2-digit', minute: '2-digit' })
}

function labelVerdict(v: string): string {
  const map: Record<string, string> = {
    tp1: '🎯 TP1', tp2: '🎯 TP2', tp3: '🎯 TP3',
    sl: '🛑 SL', be: '⚖️ BE',
    manuel: '✍️ Manuel', ts: '⏱ Time-stop',
  }
  return map[v.toLowerCase()] ?? v
}

async function charger() {
  chargement.value = true
  try {
    const res = await http.get<StraddleMonitoringData>('/api/kdj/monitoring-ml')
    monitoring.value = res.data
    derniereMaj.value = Date.now()
  } catch { /* repli silencieux — chargement */ }
  finally { chargement.value = false }
}

onMounted(charger)
</script>
