<template>
  <div class="glass-card p-4 flex flex-col gap-3 overflow-hidden">
    <!-- En-tête -->
    <div class="flex items-center justify-between flex-wrap gap-2">
      <span class="text-xs uppercase font-bold text-white">📐 Métriques</span>
      <div class="flex items-center gap-3 text-xs">
        <span class="text-gray-500 flex items-center gap-1">
          {{ data?.nb_trades_saisis ?? 0 }} trades clôturés
          <TooltipIcon>Trades réellement ouverts (hors invalides/expirés)</TooltipIcon>
        </span>
        <button class="text-gray-600 hover:text-gray-400 transition-colors" @click="charger">↺</button>
      </div>
    </div>

    <div v-if="chargement" class="text-center text-gray-600 text-xs py-4">Chargement…</div>
    <div v-else-if="!data || data.points.length === 0" class="text-center text-gray-600 text-xs py-4">
      Aucun trade clôturé — le bloc se remplira automatiquement.
    </div>

    <template v-else>
      <!-- Alerte sample size -->
      <div v-if="data.points.length < 100" class="text-xs text-yellow-400/80 bg-yellow-900/20 rounded px-2 py-1">
        ⚠️ {{ data.points.length }} trades — statistiques non significatives (&lt; 100 requis)
      </div>

      <!-- Métriques repensées Ergonomie -->
      <div class="grid grid-cols-5 gap-y-4 gap-x-2 pt-2">
        <div class="flex flex-col gap-0.5 bg-white/5 p-2 rounded-lg relative overflow-hidden">
          <div class="absolute right-0 top-0 bottom-0 w-1 bg-emerald-500/20"></div>
          <span class="text-gray-400 text-[10px] uppercase font-bold tracking-wider">Capital Net</span>
          <span class="font-mono font-bold text-sm" :class="capitalNet >= data.capital_initial ? 'text-emerald-400' : 'text-red-400'">
            {{ formatEuro(capitalNet) }}
            <span class="text-[9px] font-normal text-gray-500">{{ pctNet >= 0 ? '+' : '' }}{{ pctNet.toFixed(1) }}%</span>
          </span>
        </div>
        <div class="flex flex-col gap-0.5 bg-white/5 p-2 rounded-lg">
          <span class="text-gray-400 text-[10px] uppercase font-bold tracking-wider flex items-center gap-1">Win Rate <TooltipIcon>Wilson IC 95%</TooltipIcon></span>
          <span class="font-mono font-bold text-sm" :class="winRate >= 0.5 ? 'text-emerald-400' : 'text-red-400'">
            {{ Math.round(winRate * 100) }}% <span class="text-[9px] text-gray-500 font-normal">±{{ icWr }}%</span>
          </span>
        </div>
        <div class="flex flex-col gap-0.5 bg-white/5 p-2 rounded-lg">
          <span class="text-gray-400 text-[10px] uppercase font-bold tracking-wider">Ratio G/P</span>
          <span class="font-mono font-bold text-sm">
            <span class="text-emerald-400">{{ nbGagnants }}</span><span class="text-gray-600 px-1">/</span><span class="text-red-400">{{ nbPerdants }}</span>
          </span>
        </div>
        <div class="flex flex-col gap-0.5 bg-white/5 p-2 rounded-lg relative overflow-hidden">
           <div class="absolute right-0 top-0 bottom-0 w-1" :class="profitFactor >= 1.5 ? 'bg-emerald-500/30' : 'bg-red-500/30'"></div>
          <span class="text-gray-400 text-[10px] uppercase font-bold tracking-wider flex items-center gap-1">Profit Fact <TooltipIcon>Gains / Pertes</TooltipIcon></span>
          <span class="font-mono font-bold text-sm" :class="profitFactor >= 1.5 ? 'text-emerald-400' : profitFactor >= 1 ? 'text-yellow-400' : 'text-red-400'">
            {{ profitFactor === Infinity ? '∞' : profitFactor.toFixed(2) }}
          </span>
        </div>
        <div class="flex flex-col gap-0.5 bg-white/5 p-2 rounded-lg">
          <span class="text-gray-400 text-[10px] uppercase font-bold tracking-wider flex items-center gap-1">Trades BE <TooltipIcon>Fermés sans gain ni perte significative</TooltipIcon></span>
          <span class="font-mono font-bold text-sm text-gray-300">
            {{ nbBE }} <span class="text-gray-500 text-[9px] font-normal font-sans tracking-tight">Neutres</span>
          </span>
        </div>

        <div class="flex flex-col gap-0.5 bg-white/5 p-2 rounded-lg">
          <span class="text-gray-400 text-[10px] uppercase font-bold tracking-wider flex items-center gap-1">Expectancy <TooltipIcon>Gain moy par trade (R)</TooltipIcon></span>
          <span class="font-mono font-bold text-sm" :class="expectancy >= 0 ? 'text-emerald-400' : 'text-red-400'">
            {{ expectancy >= 0 ? '+' : '' }}{{ expectancy.toFixed(2) }}R
          </span>
        </div>
        <div class="flex flex-col gap-0.5 bg-white/5 p-2 rounded-lg">
          <span class="text-gray-400 text-[10px] uppercase font-bold tracking-wider flex items-center gap-1">ROI Ann. <span v-if="roiAnnualise !== null && nbJours < 30" class="text-yellow-500 text-[9px]">⚠️</span></span>
          <span class="font-mono font-bold text-sm" :class="(roiAnnualise ?? 0) >= 0 ? 'text-emerald-400' : 'text-red-400'">
            <template v-if="roiAnnualise === null">N/A</template>
            <template v-else>{{ Math.round(roiAnnualise) }}%/an</template>
          </span>
        </div>
        <div class="flex flex-col gap-0.5 bg-white/5 p-2 rounded-lg relative overflow-hidden">
          <div class="absolute right-0 top-0 bottom-0 w-1 bg-red-500/20"></div>
          <span class="text-gray-400 text-[10px] uppercase font-bold tracking-wider flex items-center gap-1">Max DD <TooltipIcon>Max Drawdown (%)</TooltipIcon></span>
          <span class="font-mono font-bold text-sm text-red-300">-{{ maxDrawdownPct.toFixed(1) }}%</span>
        </div>
        <div class="flex flex-col gap-0.5 bg-white/5 p-2 rounded-lg">
          <span class="text-gray-400 text-[10px] uppercase font-bold tracking-wider flex items-center gap-1">Durée Trade <TooltipIcon>Temps moyen en position</TooltipIcon></span>
          <span class="font-mono font-bold text-sm text-gray-300">
            {{ dureeMoyenne }}
          </span>
        </div>
        <div class="flex flex-col gap-px bg-white/5 p-2 rounded-lg relative overflow-hidden">
          <span class="text-gray-400 text-[10px] uppercase font-bold tracking-wider flex items-center gap-1">Frais simulés <TooltipIcon>Basé sur une estimation de 0.2% par transaction</TooltipIcon></span>
          <span class="font-mono font-bold text-xs text-orange-400/90 leading-tight">
            -{{ formatEuro(fraisEstimes) }}
          </span>
          <span class="text-[8px] text-gray-500 font-sans tracking-tight">
            {{ formatEuro((data?.capital_initial ?? 10000) * (data?.risk_pct ?? 0.015) * FRAIS_RT) }}/tr x {{ data?.points.length ?? 0 }} tr
          </span>
        </div>
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useSmcPerf } from '@/composables/useStrategiesPerf'
import TooltipIcon from './TooltipIcon.vue'

const { data, chargement, charger } = useSmcPerf()

const FRAIS_RT = 0.002 // 0.2% aller-retour estimé sur le montant risqué

// ── Base ─────────────────────────────────────────────────────────────────────
const capitalFinal = computed(() =>
  data.value?.points.at(-1)?.equity_cumulee ?? data.value?.capital_initial ?? 10000
)
const fraisEstimes = computed(() => {
  if (!data.value) return 0
  return data.value.points.length * data.value.capital_initial * data.value.risk_pct * FRAIS_RT
})
const capitalNet = computed(() => capitalFinal.value - fraisEstimes.value)
const pctNet = computed(() => {
  if (!data.value) return 0
  return (capitalNet.value - data.value.capital_initial) / data.value.capital_initial * 100
})
const nbGagnants = computed(() =>
  data.value?.points.filter(p => p.verdict.startsWith('tp')).length ?? 0
)
const nbBE = computed(() =>
  data.value?.points.filter(p =>
    p.verdict.toLowerCase().includes('be') ||
    p.verdict.toLowerCase().includes('neutre') ||
    p.pnl_r === 0
  ).length ?? 0
)
const nbPerdants = computed(() =>
  data.value ? data.value.points.length - nbGagnants.value - nbBE.value : 0
)
const winRate = computed(() => {
  const n = data.value?.points.length ?? 0
  return n > 0 ? nbGagnants.value / n : 0
})
const icWr = computed(() => {
  const n = data.value?.points.length ?? 0
  if (n === 0) return 0
  return Math.round(1.96 * Math.sqrt((winRate.value * (1 - winRate.value)) / n) * 100)
})

// ── Métriques avancées ───────────────────────────────────────────────────────
const avgRWins = computed(() => {
  const wins = data.value?.points.filter(p => p.verdict.startsWith('tp')) ?? []
  return wins.length ? wins.reduce((s, p) => s + p.pnl_r, 0) / wins.length : 0
})
const avgRLosses = computed(() => {
  const losses = data.value?.points.filter(p => !p.verdict.startsWith('tp')) ?? []
  return losses.length ? losses.reduce((s, p) => s + p.pnl_r, 0) / losses.length : 0
})
const profitFactor = computed(() => {
  const gains = data.value?.points.filter(p => p.verdict.startsWith('tp')).reduce((s, p) => s + p.pnl_r, 0) ?? 0
  const pertes = Math.abs(data.value?.points.filter(p => !p.verdict.startsWith('tp')).reduce((s, p) => s + p.pnl_r, 0) ?? 0)
  return pertes === 0 ? Infinity : gains / pertes
})
const expectancy = computed(() =>
  winRate.value * avgRWins.value + (1 - winRate.value) * avgRLosses.value
)

// ── Temporel ─────────────────────────────────────────────────────────────────
const nbJours = computed(() => {
  const pts = data.value?.points
  if (!pts?.length) return 0
  return Math.max(1, (pts.at(-1)!.ferme_le - pts[0].ferme_le) / 86400)
})
const roiAnnualise = computed(() => {
  if (!data.value || nbJours.value < 2) return null
  const roi = (capitalNet.value - data.value.capital_initial) / data.value.capital_initial
  const annuel = ((1 + roi) ** (365 / nbJours.value) - 1) * 100
  return Math.max(-9999, Math.min(9999, annuel))
})
const maxDrawdownPct = computed(() => {
  const pts = data.value?.points
  if (!pts?.length || !data.value) return 0
  let peak = data.value.capital_initial
  let maxDD = 0
  for (const p of pts) {
    if (p.equity_cumulee > peak) peak = p.equity_cumulee
    const dd = (peak - p.equity_cumulee) / peak * 100
    if (dd > maxDD) maxDD = dd
  }
  return maxDD
})
const dureeMoyenne = computed(() => {
  const pts = data.value?.points
  if (!pts?.length) return '—'
  const valides = pts.filter(p => p.duree_min > 0)
  if (!valides.length) return '—'
  const avg = valides.reduce((s, p) => s + p.duree_min, 0) / valides.length
  if (avg < 60) return `${Math.round(avg)}min`
  const h = Math.floor(avg / 60)
  const m = Math.round(avg % 60)
  return `${h}h${m.toString().padStart(2, '0')}`
})

function formatEuro(v: number): string {
  return new Intl.NumberFormat('fr-FR', { style: 'currency', currency: 'EUR', maximumFractionDigits: 0 }).format(v)
}
</script>

<style scoped>
.glass-card { @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm; }
</style>
