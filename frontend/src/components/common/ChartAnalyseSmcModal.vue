<template>
  <div
    v-if="open"
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/70"
    @click.self="$emit('close')"
  >
    <div
      class="rounded-xl border border-white/10 p-5 w-[700px] max-h-[90vh] flex flex-col gap-4"
      style="background: #0d1117;"
    >
      <!-- Header -->
      <div class="flex items-center justify-between flex-shrink-0">
        <h2 class="text-base font-bold">🧠 Analyse SMC — {{ asset }} {{ timeframe }}</h2>
        <button class="text-white hover:text-white text-xl leading-none" @click="$emit('close')">×</button>
      </div>

      <!-- Score SMC + composants -->
      <div v-if="scoreSmc" class="glass-card p-4 flex-shrink-0">
        <div class="flex items-center gap-6 mb-4">
          <div>
            <div class="text-xs text-white uppercase mb-1">Direction</div>
            <span class="text-xl font-bold" :class="dirCls">{{ scoreSmc.direction.toUpperCase() }}</span>
          </div>
          <div>
            <div class="text-xs text-white uppercase mb-1">Score SMC</div>
            <span class="text-xl font-bold" :class="scoreCls">{{ Math.round(scoreSmc.total) }}/100</span>
          </div>
          <div v-if="prixEntree">
            <div class="text-xs text-white uppercase mb-1">Prix entrée</div>
            <span class="text-base font-mono text-white">{{ fmt(prixEntree) }}</span>
          </div>
          <div class="flex gap-3 ml-auto text-xs">
            <span :class="scoreSmc.kill_zone_active ? 'text-emerald-400' : 'text-white'">{{ scoreSmc.kill_zone_active ? '✓' : '✗' }} Kill Zone</span>
            <span :class="scoreSmc.sweep_detecte ? 'text-emerald-400' : 'text-white'">{{ scoreSmc.sweep_detecte ? '✓' : '✗' }} Sweep</span>
            <span :class="scoreSmc.bos ? 'text-emerald-400' : 'text-white'">{{ scoreSmc.bos ? '✓' : '✗' }} BOS</span>
            <span :class="scoreSmc.choch ? 'text-yellow-400' : 'text-white'">{{ scoreSmc.choch ? '✓' : '✗' }} CHoCH</span>
          </div>
        </div>
        <div class="grid grid-cols-5 gap-3">
          <div v-for="c in composants" :key="c.label" class="text-center">
            <div class="text-xs text-white mb-1">{{ c.label }}</div>
            <div class="w-full bg-gray-800 rounded-full h-1.5 mb-1">
              <div class="h-1.5 rounded-full" :class="c.pts / c.max >= 0.6 ? 'bg-emerald-500' : c.pts / c.max >= 0.3 ? 'bg-yellow-500' : 'bg-red-600'" :style="{ width: `${(c.pts / c.max) * 100}%` }" />
            </div>
            <div class="text-xs font-bold text-white">{{ c.pts.toFixed(0) }}<span class="text-white font-normal">/{{ c.max }}</span></div>
          </div>
        </div>
      </div>

      <!-- Table position SL/TP -->
      <div v-if="slAnalyse && tp1Analyse" class="glass-card p-4 flex-shrink-0">
        <div class="text-xs text-white uppercase font-semibold tracking-wide mb-3">Position calculée (ATR ×2)</div>
        <div class="grid grid-cols-4 gap-3 text-center">
          <div>
            <div class="text-xs text-white mb-1">Stop Loss</div>
            <span class="font-mono font-bold text-red-400">{{ fmt(slAnalyse) }}</span>
          </div>
          <div>
            <div class="text-xs text-white mb-1">TP1</div>
            <span class="font-mono font-bold text-emerald-400">{{ fmt(tp1Analyse) }}</span>
          </div>
          <div v-if="tp2Analyse">
            <div class="text-xs text-white mb-1">TP2</div>
            <span class="font-mono font-bold text-emerald-300">{{ fmt(tp2Analyse) }}</span>
          </div>
          <div>
            <div class="text-xs text-white mb-1">R:R (TP1)</div>
            <span class="font-bold" :class="rrVal >= 1.5 ? 'text-emerald-400' : 'text-yellow-400'">1:{{ rrVal.toFixed(2) }}</span>
          </div>
        </div>
      </div>

      <!-- Chargement IA -->
      <div v-if="chargement" class="flex items-center gap-2 text-yellow-400 text-sm animate-pulse flex-shrink-0">
        <span>🔍</span> Analyse IA en cours…
      </div>

      <!-- Texte analyse IA -->
      <div v-else-if="analyseTexte" class="overflow-auto flex-1 glass-card p-4">
        <div class="space-y-3">
          <div
            v-for="(bloc, i) in blocsAnalyse"
            :key="i"
            class="text-sm text-white leading-relaxed border-l-2 pl-3"
            :class="bloc.cls"
            v-html="bloc.html"
          />
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import type { ScoreSmc } from '@/services/api.service'

const props = defineProps<{
  open: boolean
  asset: string
  timeframe: string
  scoreSmc: ScoreSmc | null
  prixEntree: number | null
  slAnalyse: number | null
  tp1Analyse: number | null
  tp2Analyse: number | null
  chargement: boolean
  analyseTexte: string | null
}>()

defineEmits<{ close: [] }>()

function fmt(v: number | null | undefined): string {
  if (v === null || v === undefined) return '—'
  return v > 100 ? v.toFixed(2) : v.toFixed(5)
}

const composants = computed(() => {
  const s = props.scoreSmc
  if (!s) return []
  return [
    { label: 'Tendance',   pts: s.tendance,    max: 25 },
    { label: 'Ord. Block', pts: s.order_block, max: 25 },
    { label: 'Imbalance',  pts: s.imbalance,   max: 15 },
    { label: 'IFVG',       pts: s.ifvg,        max: 20 },
    { label: 'Fibonacci',  pts: s.fibonacci,   max: 15 },
  ]
})

const dirCls = computed(() => {
  const d = props.scoreSmc?.direction?.toLowerCase() ?? ''
  if (d.includes('long') || d.includes('buy')) return 'text-emerald-400'
  if (d.includes('short') || d.includes('sell')) return 'text-red-400'
  return 'text-yellow-400'
})

const scoreCls = computed(() => {
  const t = props.scoreSmc?.total ?? 0
  return t >= 70 ? 'text-emerald-400' : t >= 50 ? 'text-yellow-400' : 'text-red-400'
})

const rrVal = computed(() => {
  if (!props.prixEntree || !props.slAnalyse || !props.tp1Analyse) return 0
  const risk = Math.abs(props.prixEntree - props.slAnalyse)
  if (risk === 0) return 0
  return Math.abs(props.tp1Analyse - props.prixEntree) / risk
})

/** Découpe le texte LLM en blocs numérotés avec mise en forme */
const blocsAnalyse = computed(() => {
  const t = props.analyseTexte
  if (!t) return []
  return t
    .split(/\n(?=\d+\.)/)
    .map(raw => raw.trim())
    .filter(Boolean)
    .map(raw => {
      // Retirer le préfixe ### s'il existe dans un bloc non numéroté
      const cleaned = raw.replace(/^###\s*/m, '')
      const html = cleaned
        .replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')
        .replace(/\*\*(.+?)\*\*/g, '<strong class="text-white">$1</strong>')
        .replace(/\n/g, '<br/>')
      // Coloration selon le contenu
      const lower = cleaned.toLowerCase()
      const isOk = lower.includes('force') || lower.includes('positif') || lower.includes('favorable')
      const isWarn = lower.includes('faible') || lower.includes('risque') || lower.includes('manque') || lower.includes('absent') || lower.includes('rejet')
      const cls = isWarn ? 'border-red-700/60' : isOk ? 'border-emerald-700/60' : 'border-white/10'
      return { html, cls }
    })
})
</script>

<style scoped>
.glass-card {
  @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm;
}
</style>
