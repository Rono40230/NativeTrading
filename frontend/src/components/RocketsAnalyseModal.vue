<template>
  <div v-if="open" class="fixed inset-0 z-50 flex items-center justify-center bg-black/70" @click.self="$emit('close')">
    <div class="rounded-xl border border-white/10 p-5 w-[96vw] h-[92vh] flex flex-col gap-3" style="background: #0d1117;">

      <!-- Header -->
      <div class="flex items-center justify-between flex-shrink-0">
        <h2 class="text-lg font-bold">📊 Analyse Rockets</h2>
        <button class="text-gray-400 hover:text-white text-xl leading-none" @click="$emit('close')">×</button>
      </div>

      <!-- KPIs -->
      <div class="grid grid-cols-4 gap-3 flex-shrink-0">
        <div class="kpi-card text-center">
          <div class="text-xl font-bold text-white">{{ stats.total }}</div>
          <div class="text-xs text-gray-400 mt-0.5">Total clôturés</div>
        </div>
        <div class="kpi-card text-center">
          <div class="text-xl font-bold text-emerald-400">{{ stats.tauxGagnants }}%</div>
          <div class="text-xs text-gray-400 mt-0.5">Win rate</div>
        </div>
        <div class="kpi-card text-center">
          <div class="text-xl font-bold" :class="stats.rMoyen >= 0 ? 'text-emerald-400' : 'text-red-400'">{{ stats.rMoyen }}R</div>
          <div class="text-xs text-gray-400 mt-0.5">R moyen</div>
        </div>
        <div class="kpi-card text-center">
          <div class="text-xl font-bold text-red-400">{{ stats.tauxSL }}%</div>
          <div class="text-xs text-gray-400 mt-0.5">Loss rate réel</div>
        </div>
      </div>

      <!-- Contenu : 2 colonnes -->
      <div class="grid grid-cols-[1fr_1.8fr] gap-4 flex-1 min-h-0">

        <!-- Gauche : tranches + phases -->
        <div class="flex flex-col gap-4 min-h-0 overflow-auto pr-1">
          <div>
            <h3 class="section-title">Par tranche de score</h3>
            <table class="w-full text-xs">
              <thead>
                <tr class="text-gray-500 border-b border-white/10">
                  <th class="py-1 text-left">Score</th>
                  <th class="py-1 text-right">Nb</th>
                  <th class="py-1 text-right text-emerald-400">TP1</th>
                  <th class="py-1 text-right text-emerald-300">TP2</th>
                  <th class="py-1 text-right text-emerald-200">TP3</th>
                  <th class="py-1 text-right text-red-400">SL</th>
                  <th class="py-1 text-right text-gray-500">Exp</th>
                  <th class="py-1 text-right">Win%</th>
                  <th class="py-1 text-right">R</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="t in tranches" :key="t.label" class="border-b border-white/5">
                  <td class="py-1 font-mono text-white">{{ t.label }}</td>
                  <td class="py-1 text-right text-gray-400">{{ t.total }}</td>
                  <td class="py-1 text-right text-emerald-400">{{ t.tp1 }}</td>
                  <td class="py-1 text-right text-emerald-300">{{ t.tp2 }}</td>
                  <td class="py-1 text-right text-emerald-200">{{ t.tp3 }}</td>
                  <td class="py-1 text-right text-red-400">{{ t.sl }}</td>
                  <td class="py-1 text-right text-gray-500">{{ t.expire }}</td>
                  <td class="py-1 text-right font-bold" :class="t.winPct >= 50 ? 'text-emerald-400' : 'text-red-400'">{{ t.winPct }}%</td>
                  <td class="py-1 text-right font-bold" :class="t.rMoyen >= 0 ? 'text-emerald-400' : 'text-red-400'">{{ t.rMoyen }}</td>
                </tr>
              </tbody>
            </table>
          </div>

          <div>
            <h3 class="section-title">Par phase</h3>
            <div class="grid grid-cols-2 gap-2">
              <div v-for="p in phases" :key="p.phase" class="kpi-card">
                <div class="flex justify-between mb-1">
                  <span class="text-xs font-bold px-1.5 py-0.5 rounded-full" :class="classePhase(p.phase)">{{ p.phase }}</span>
                  <span class="text-gray-500 text-xs">{{ p.total }}</span>
                </div>
                <div class="text-xs">Win : <span class="font-bold" :class="p.winPct >= 50 ? 'text-emerald-400' : 'text-red-400'">{{ p.winPct }}%</span></div>
                <div class="text-xs">R : <span class="font-bold" :class="p.rMoyen >= 0 ? 'text-emerald-400' : 'text-red-400'">{{ p.rMoyen }}</span></div>
              </div>
            </div>
          </div>
        </div>

        <!-- Droite : tableau probabilités -->
        <div class="flex flex-col min-h-0">
          <div class="flex items-baseline gap-3 mb-1 flex-shrink-0">
            <h3 class="section-title">Probabilité de séries de SL consécutifs</h3>
            <span class="text-xs text-gray-500">sur {{ sampleSize }} trades clôturés</span>
          </div>
          <p class="text-xs text-gray-500 mb-2 flex-shrink-0">
            Ligne <span class="text-blue-400 font-bold">surlignée</span> = votre loss rate réel ({{ lossRateReel }}%).
            Colonnes = nombre de SL consécutifs. Vert = très probable, rouge = rare.
          </p>
          <div class="overflow-auto flex-1 rounded-lg">
            <table class="text-xs border-collapse w-full">
              <thead class="sticky top-0" style="background: #0d1117">
                <tr>
                  <th class="px-3 py-1.5 text-left text-gray-500 font-medium">Loss %</th>
                  <th v-for="k in kValues" :key="k" class="px-3 py-1.5 text-center text-gray-500 font-medium">{{ k }}</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="row in tableauPertes" :key="row.lossRate">
                  <td class="px-3 py-1 font-mono font-bold sticky left-0"
                      :style="row.isActual ? 'background:#1e3a5f; color:#93c5fd' : 'background:#0d1117; color:#9ca3af'">
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
import { computed } from 'vue'
import type { RocketSignalHistorique } from '@/services/api.types'

const props = defineProps<{ open: boolean; rockets: RocketSignalHistorique[] }>()
defineEmits(['close'])

// ── Stats helpers ──────────────────────────────────────────────────────────

function rocketR(r: RocketSignalHistorique): number | null {
  const v = r.verdict
  if (!v) return null
  const risk = r.prix_entree - r.stop_loss
  if (risk <= 0) return null
  if (v === 'invalide') return -1
  if (v === 'TP1' || v === 'confirme') return 1
  if (v === 'TP2') return 2
  if (v === 'TP3' && r.prix_verdict) return (r.prix_verdict - r.prix_entree) / risk
  return null
}

function calcStats(liste: RocketSignalHistorique[]) {
  const clos    = liste.filter(r => r.verdict && r.verdict !== 'expire')
  const total   = clos.length
  const tp1     = clos.filter(r => r.verdict === 'TP1' || r.verdict === 'confirme').length
  const tp2     = clos.filter(r => r.verdict === 'TP2').length
  const tp3     = clos.filter(r => r.verdict === 'TP3').length
  const sl      = clos.filter(r => r.verdict === 'invalide').length
  const expire  = liste.filter(r => r.verdict === 'expire').length
  const gain    = tp1 + tp2 + tp3
  const winPct  = total > 0 ? Math.round(gain / total * 100) : 0
  const rs      = clos.map(r => rocketR(r)).filter((v): v is number => v !== null)
  const rMoyen  = rs.length > 0 ? parseFloat((rs.reduce((a, b) => a + b, 0) / rs.length).toFixed(2)) : 0
  return { total, tp1, tp2, tp3, sl, expire, winPct, rMoyen }
}

const TRANCHES_DEF = [
  { label: '15–39', min: 15, max: 39 },
  { label: '40–59', min: 40, max: 59 },
  { label: '60–79', min: 60, max: 79 },
  { label: '80–100', min: 80, max: 100 },
]

const stats = computed(() => {
  const s = calcStats(props.rockets)
  return { ...s, tauxGagnants: s.winPct, tauxSL: s.total > 0 ? Math.round(s.sl / s.total * 100) : 0 }
})

const tranches = computed(() =>
  TRANCHES_DEF.map(t => ({
    label: t.label,
    ...calcStats(props.rockets.filter(r => r.score >= t.min && r.score <= t.max)),
  }))
)

const phases = computed(() => {
  const ps = [...new Set(props.rockets.map(r => r.phase))]
  return ps.map(phase => ({ phase, ...calcStats(props.rockets.filter(r => r.phase === phase)) }))
})

function classePhase(phase: string): string {
  if (phase.toLowerCase().includes('break')) return 'bg-emerald-900/60 text-emerald-300'
  if (phase.toLowerCase().includes('bull'))  return 'bg-blue-900/60 text-blue-300'
  if (phase.toLowerCase().includes('bear'))  return 'bg-red-900/60 text-red-300'
  return 'bg-yellow-900/60 text-yellow-300'
}

// ── Tableau probabilités SL consécutifs ────────────────────────────────────

const kValues    = [2, 3, 4, 5, 6, 7, 8, 9, 10]
const lossRates  = [5, 10, 15, 20, 25, 30, 35, 40, 45, 50, 55, 60, 65, 70, 75, 80, 85, 90, 95]
const sampleSize   = computed(() => Math.max(props.rockets.length, 10))
const lossRateReel = computed(() => stats.value.tauxSL)

function probAtLeastKCons(n: number, p: number, k: number): number {
  if (n < k || p <= 0) return 0
  if (p >= 1) return 100
  const dp = new Array(k).fill(0)
  dp[0] = 1.0
  for (let i = 0; i < n; i++) {
    const next = new Array(k).fill(0)
    for (let j = 0; j < k; j++) {
      if (dp[j] === 0) continue
      next[0] += dp[j] * (1 - p)
      if (j + 1 < k) next[j + 1] += dp[j] * p
    }
    dp.splice(0, dp.length, ...next)
  }
  const pNever = dp.reduce((a, b) => a + b, 0)
  return Math.round(Math.max(0, Math.min(100, (1 - pNever) * 100)) * 10) / 10
}

const tableauPertes = computed(() => {
  const n      = sampleSize.value
  const actual = lossRateReel.value
  const nearest = lossRates.reduce((prev, cur) =>
    Math.abs(cur - actual) < Math.abs(prev - actual) ? cur : prev, lossRates[0])
  return lossRates.map(lr => ({
    lossRate: lr,
    isActual: lr === nearest && actual > 0,
    probs: kValues.map(k => probAtLeastKCons(n, lr / 100, k)),
  }))
})

function couleurProba(pct: number): string {
  const hue       = Math.round(pct * 1.2) // 0 = rouge, 120 = vert
  const lightness = pct > 5 ? 28 : 12
  return `hsl(${hue}, 70%, ${lightness}%)`
}
</script>

<style scoped>
.kpi-card    { @apply bg-white/5 rounded-lg p-3 border border-white/10; }
.section-title { @apply text-xs font-semibold text-gray-400 mb-2 uppercase tracking-wide; }
</style>
