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
        <th class="px-4 py-3 text-right cursor-pointer hover:text-white select-none" @click="$emit('trierPar', 'target3')">Trailing (R+3) <span>{{ icone('target3') }}</span></th>
        <th class="px-4 py-3 text-left">Phase</th>
        <th v-if="showPrixActuel" class="px-4 py-3 text-right">Prix actuel</th>
        <th v-if="showSortie" class="px-4 py-3 text-right cursor-pointer hover:text-white select-none" @click="$emit('trierPar', 'prix_verdict')">Sortie <span>{{ icone('prix_verdict') }}</span></th>
        <th v-if="showSortie" class="px-4 py-3 text-right">PnL R</th>
        <th v-if="showSortie" class="px-4 py-3 text-center">LLM ✓</th>
        <th class="px-4 py-3 text-left cursor-pointer hover:text-white select-none" @click="$emit('trierPar', 'verdict')">Verdict <span>{{ icone('verdict') }}</span></th>
        <th class="px-4 py-3 text-center">IA</th>
        <th class="px-4 py-3 text-left cursor-pointer hover:text-white select-none" @click="$emit('trierPar', 'cree_le')">Ouvert le <span>{{ icone('cree_le') }}</span></th>
        <th v-if="showFermeLe" class="px-4 py-3 text-left cursor-pointer hover:text-white select-none" @click="$emit('trierPar', 'maj_le')">Fermé le <span>{{ icone('maj_le') }}</span></th>
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
        <td class="px-4 py-3">
          <span class="text-xs px-2 py-0.5 rounded-full font-medium" :class="classePhasePosition(r)">{{ labelPhasePosition(r) }}</span>
        </td>
        <td v-if="showPrixActuel" class="px-4 py-3 text-right font-mono">
          <span v-if="prixActuels[r.ticker]" :class="classePrixActuel(r)">{{ fmt(prixActuels[r.ticker]) }}</span>
          <span v-else class="text-gray-600">—</span>
        </td>
        <td v-if="showSortie" class="px-4 py-3 text-right font-mono text-white">{{ r.prix_verdict ? fmt(r.prix_verdict) : '—' }}</td>
        <td v-if="showSortie" class="px-4 py-3 text-right font-mono font-semibold" :class="classePnlR(r.pnl_r)">{{ fmtPnlR(r.pnl_r) }}</td>
        <td v-if="showSortie" class="px-4 py-3 text-center">
          <span :class="classeLlmVerif(r)" :title="labelLlmVerifTitle(r)">{{ labelLlmVerif(r) }}</span>
        </td>
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
        <td class="px-4 py-3 text-gray-500 text-xs">{{ new Date(r.cree_le.replace(' ', 'T') + 'Z').toLocaleString('fr-FR', { day: '2-digit', month: '2-digit', year: 'numeric', hour: '2-digit', minute: '2-digit', timeZone: Intl.DateTimeFormat().resolvedOptions().timeZone }) }}</td>
        <td v-if="showFermeLe" class="px-4 py-3 text-gray-500 text-xs">{{ r.maj_le ? new Date(r.maj_le.replace(' ', 'T') + 'Z').toLocaleString('fr-FR', { day: '2-digit', month: '2-digit', year: 'numeric', hour: '2-digit', minute: '2-digit', timeZone: Intl.DateTimeFormat().resolvedOptions().timeZone }) : '—' }}</td>
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
  showFermeLe?: boolean
}>()

const showPrixActuel = computed(() => props.showPrixActuel !== false)
const showSortie = computed(() => props.showSortie !== false)
const showFermeLe = computed(() => props.showFermeLe === true)

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
  if (v === 'TP1' || v === 'TP2' || v === 'TP3' || v === 'tp1' || v === 'tp2' || v === 'tp3' || v === 'confirme') return 'badge-green'
  if (v === 'sl') return 'badge-red'
  if (v === 'invalide') return 'badge-orange'
  if (v === 'expire') return 'badge-gray'
  if (v === 'be') return 'badge-gray'
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
  if (v === 'invalide') return '↩️ Entrée non atteinte'
  if (v === 'sl') return '❌ SL'
  if (v === 'be') return '⚪ BE'
  if (v === 'tp1' || v === 'TP1' || v === 'confirme') return '✅ TP1 (SL→BE)'
  if (v === 'tp2' || v === 'TP2') return '✅ TP2 (SL→TP1)'
  if (v === 'TP1') return '✅ +1R'
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
  if (r.target3 && prix >= r.target3) return '🟢 R+3 ✓ · Trail ON'
  if (r.target2 && prix >= r.target2) return '🔵 R+2 ✓ · Trail ON'
  if (prix >= r.target) return '🔵 R+1 ✓ · SL@BE'
  if (prix <= r.stop_loss) return '🔴 SL touché'
  return '⏳ En cours'
}

function labelPhasePosition(r: RocketSignalHistorique): string {
  const v = r.verdict
  const prix = props.prixActuels[r.ticker]
  // Verdict final (position fermée)
  if (v === 'TP3') return 'TRAILING'
  if (v === 'TP2') return 'POST_TP2'
  if (v === 'TP1' || v === 'confirme') return 'POST_TP1'
  if (v === 'invalide' || v === 'expire') return 'INIT'
  // Position ouverte : phase courante via prix
  if (prix) {
    if (r.target2 && prix >= r.target2) return 'TRAILING'
    if (prix >= r.target) return 'POST_TP1'
  }
  return 'INIT'
}

function classePhasePosition(r: RocketSignalHistorique): string {
  const p = labelPhasePosition(r)
  if (p === 'TRAILING') return 'bg-emerald-900/60 text-emerald-300'
  if (p === 'POST_TP2') return 'bg-blue-900/60 text-blue-300'
  if (p === 'POST_TP1') return 'bg-blue-900/40 text-blue-400'
  return 'bg-gray-800 text-gray-500'
}

// ── PnL R ─────────────────────────────────────────────────────────────────────

function fmtPnlR(v: number | null | undefined): string {
  if (v === null || v === undefined) return '—'
  return (v >= 0 ? '+' : '') + v.toFixed(2) + 'R'
}

function classePnlR(v: number | null | undefined): string {
  if (v === null || v === undefined) return 'text-gray-500'
  return v >= 0 ? 'text-emerald-400' : 'text-red-400'
}

// ── Badge LLM conviction vs résultat ─────────────────────────────────────────
// conviction ≥ 70 + gagnant = 1 → ✅ vert (LLM juste)
// conviction ≥ 70 + gagnant = 0 → ❌ rouge (LLM faux)
// conviction < 70 + gagnant = 1 → ⚠️ orange (chanceux)
// conviction < 70 + gagnant = 0 → — gris (cohérent)

function labelLlmVerif(r: RocketSignalHistorique): string {
  if (r.gagnant === null || r.gagnant === undefined) return '—'
  const conv = r.llm_conviction ?? 0
  if (conv >= 70 && r.gagnant === 1) return '✅'
  if (conv >= 70 && r.gagnant === 0) return '❌'
  if (conv < 70 && r.gagnant === 1) return '⚠️'
  return '—'
}

function labelLlmVerifTitle(r: RocketSignalHistorique): string {
  if (r.gagnant === null || r.gagnant === undefined) return 'Feedback non disponible'
  const conv = r.llm_conviction ?? 0
  if (conv >= 70 && r.gagnant === 1) return `LLM confiant (${conv}) — trade gagnant ✓`
  if (conv >= 70 && r.gagnant === 0) return `LLM confiant (${conv}) — trade perdant ✗`
  if (conv < 70 && r.gagnant === 1) return `LLM peu confiant (${conv}) — trade gagnant (chanceux)`
  return `LLM peu confiant (${conv}) — trade perdant (cohérent)`
}

function classeLlmVerif(r: RocketSignalHistorique): string {
  if (r.gagnant === null || r.gagnant === undefined) return 'text-gray-600 text-xs'
  const conv = r.llm_conviction ?? 0
  if (conv >= 70 && r.gagnant === 1) return 'text-emerald-400 text-base'
  if (conv >= 70 && r.gagnant === 0) return 'text-red-400 text-base'
  if (conv < 70 && r.gagnant === 1) return 'text-orange-400 text-base'
  return 'text-gray-600 text-xs'
}
</script>
