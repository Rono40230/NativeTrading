<template>
  <table class="w-full text-sm">
    <thead>
      <tr class="text-gray-400 text-xs uppercase border-b border-white/10">
        <th class="px-4 py-3 text-left">#</th>
        <th class="px-4 py-3 text-left cursor-pointer hover:text-white select-none" @click="$emit('trierPar', 'ticker')">Ticker <span>{{ icone('ticker') }}</span></th>
        <th class="px-4 py-3 text-left cursor-pointer hover:text-white select-none" @click="$emit('trierPar', 'phase')">Phase <span>{{ icone('phase') }}</span></th>
        <th class="px-4 py-3 text-right cursor-pointer hover:text-white select-none" @click="$emit('trierPar', 'score')">Score <span>{{ icone('score') }}</span></th>
        <th class="px-4 py-3 text-right cursor-pointer hover:text-white select-none" @click="$emit('trierPar', 'prix_entree')">Entrée <span>{{ icone('prix_entree') }}</span></th>
        <th class="px-4 py-3 text-right cursor-pointer hover:text-white select-none" @click="$emit('trierPar', 'stop_loss')">SL <span>{{ icone('stop_loss') }}</span></th>
        <th class="px-4 py-3 text-right cursor-pointer hover:text-white select-none" @click="$emit('trierPar', 'target')">TP1 <span>{{ icone('target') }}</span></th>
        <th class="px-4 py-3 text-right cursor-pointer hover:text-white select-none" @click="$emit('trierPar', 'target2')">TP2 <span>{{ icone('target2') }}</span></th>
        <th class="px-4 py-3 text-right cursor-pointer hover:text-white select-none" @click="$emit('trierPar', 'target3')">TP3 <span>{{ icone('target3') }}</span></th>
        <th v-if="showPrixActuel" class="px-4 py-3 text-right">Prix actuel</th>
        <th v-if="showSortie" class="px-4 py-3 text-right cursor-pointer hover:text-white select-none" @click="$emit('trierPar', 'prix_verdict')">Sortie <span>{{ icone('prix_verdict') }}</span></th>
        <th class="px-4 py-3 text-left cursor-pointer hover:text-white select-none" @click="$emit('trierPar', 'verdict')">Verdict <span>{{ icone('verdict') }}</span></th>
        <th class="px-4 py-3 text-center">IA</th>
        <th class="px-4 py-3 text-left cursor-pointer hover:text-white select-none" @click="$emit('trierPar', 'cree_le')">Date <span>{{ icone('cree_le') }}</span></th>
      </tr>
    </thead>
    <tbody>
      <tr v-for="(r, i) in rockets" :key="r.id" class="border-b border-white/5 hover:bg-white/5 transition-colors">
        <td class="px-4 py-3 text-gray-500">{{ i + 1 }}</td>
        <td class="px-4 py-3 font-semibold text-white">{{ r.ticker }}</td>
        <td class="px-4 py-3">
          <span class="badge" :class="classePhase(r.phase)">{{ r.phase }}</span>
        </td>
        <td class="px-4 py-3 text-right font-mono">{{ r.score }}</td>
        <td class="px-4 py-3 text-right font-mono">{{ fmt(r.prix_entree) }}</td>
        <td class="px-4 py-3 text-right font-mono text-red-400">{{ fmt(r.stop_loss) }}</td>
        <td class="px-4 py-3 text-right font-mono text-emerald-400">{{ fmt(r.target) }}</td>
        <td class="px-4 py-3 text-right font-mono text-emerald-300">{{ r.target2 ? fmt(r.target2) : '—' }}</td>
        <td class="px-4 py-3 text-right font-mono text-emerald-200">{{ r.target3 ? fmt(r.target3) : '—' }}</td>
        <td v-if="showPrixActuel" class="px-4 py-3 text-right font-mono">
          <span v-if="prixActuels[r.ticker]" :class="classePrixActuel(r)">{{ fmt(prixActuels[r.ticker]) }}</span>
          <span v-else class="text-gray-600">—</span>
        </td>
        <td v-if="showSortie" class="px-4 py-3 text-right font-mono text-white">{{ r.prix_verdict ? fmt(r.prix_verdict) : '—' }}</td>
        <td class="px-4 py-3">
          <span class="badge" :class="classeVerdict(r)">{{ labelVerdict(r) }}</span>
        </td>
        <td class="px-4 py-3 text-center">
          <span
            v-if="r.llm_conviction !== null && r.llm_conviction !== undefined"
            class="inline-flex items-center justify-center w-8 h-8 rounded-full text-xs font-bold cursor-help"
            :class="classeConvictionLlm(r.llm_conviction)"
            :title="r.llm_raison ?? ''"
          >{{ r.llm_conviction }}</span>
          <span v-else class="text-gray-700 text-xs">—</span>
        </td>
        <td class="px-4 py-3 text-gray-500 text-xs">{{ r.cree_le.slice(0, 16).replace('T', ' ') }}</td>
      </tr>
    </tbody>
  </table>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import type { RocketSignalHistorique } from '@/services/api.types'

const props = defineProps<{
  rockets: RocketSignalHistorique[]
  prixActuels: Record<string, number>
  triColonne: string
  triDir: 'asc' | 'desc'
  showPrixActuel?: boolean
  showSortie?: boolean
}>()

const showPrixActuel = computed(() => props.showPrixActuel !== false)
const showSortie = computed(() => props.showSortie !== false)

defineEmits<{ trierPar: [col: string] }>()

function icone(col: string): string {
  if (props.triColonne !== col) return '\u21c5'
  return props.triDir === 'asc' ? '\u2191' : '\u2193'
}

function fmt(v: number | undefined): string {
  if (v === undefined || v === null) return '—'
  if (v >= 1000) return new Intl.NumberFormat('en-US', { maximumFractionDigits: 0 }).format(v)
  if (v >= 1) return v.toFixed(4)
  return v.toFixed(6)
}

function classePhase(phase: string): string {
  if (phase.toLowerCase().includes('break')) return 'badge-green'
  if (phase.toLowerCase().includes('bull')) return 'badge-blue'
  if (phase.toLowerCase().includes('bear')) return 'badge-red'
  return 'badge-yellow'
}

function classePrixActuel(r: RocketSignalHistorique): string {
  const prix = props.prixActuels[r.ticker]
  if (!prix) return 'text-gray-400'
  if (prix <= r.stop_loss) return 'text-red-400'
  if (r.target3 && prix >= r.target3) return 'text-emerald-200'
  if (r.target2 && prix >= r.target2) return 'text-emerald-300'
  if (prix >= r.target) return 'text-emerald-400'
  return 'text-blue-300'
}

function classeConvictionLlm(conviction: number | null): string {
  if (conviction === null || conviction === undefined) return 'bg-gray-700 text-gray-400'
  if (conviction >= 70) return 'bg-emerald-900 text-emerald-300 border border-emerald-600'
  if (conviction >= 50) return 'bg-yellow-900 text-yellow-300 border border-yellow-600'
  return 'bg-red-900 text-red-300 border border-red-600'
}

function classeVerdict(r: RocketSignalHistorique): string {
  const v = r.verdict
  if (v === 'TP1' || v === 'TP2' || v === 'TP3' || v === 'confirme') return 'badge-green'
  if (v === 'invalide') return 'badge-red'
  if (v === 'expire') return 'badge-gray'
  const prix = props.prixActuels[r.ticker]
  if (prix) {
    if (r.target3 && prix >= r.target3) return 'badge-green'
    if (r.target2 && prix >= r.target2) return 'badge-blue'
    if (prix >= r.target) return 'badge-blue'
    if (prix <= r.stop_loss) return 'badge-red'
  }
  return 'badge-yellow'
}

function labelVerdict(r: RocketSignalHistorique): string {
  const v = r.verdict
  if (v === 'invalide') return '❌ −1R'
  if (v === 'TP1' || v === 'confirme') return '✅ +1R'
  if (v === 'TP2') return '✅ +2R'
  if (v === 'TP3') {
    const risk = r.prix_entree - r.stop_loss
    if (risk > 0 && r.prix_verdict) {
      return `✅ +${((r.prix_verdict - r.prix_entree) / risk).toFixed(1)}R`
    }
    return '✅ +TP3'
  }
  if (v === 'expire') return '⏰ Délai 6h dépassé'
  const prix = props.prixActuels[r.ticker]
  if (!prix) return '⏳ En cours'
  if (r.target3 && prix >= r.target3) return '🟢 TP3 ✓ · SL@TP2'
  if (r.target2 && prix >= r.target2) return '🔵 TP2 ✓ · SL@TP1'
  if (prix >= r.target) return '🔵 TP1 ✓ · SL@BE'
  if (prix <= r.stop_loss) return '🔴 SL touché'
  return '⏳ En cours'
}
</script>
