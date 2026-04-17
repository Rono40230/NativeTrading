<template>
  <div class="glass-bar px-4 py-2.5 flex flex-col gap-2 h-full overflow-y-auto">
    <span class="text-xs font-semibold uppercase tracking-widest text-blue-400 shrink-0">⚡ Veille Straddle</span>

    <!-- 1. Meilleure fenêtre du jour actuel -->
    <section v-if="fenDujour" class="border-b border-white/5 pb-1.5 space-y-0.5 shrink-0">
      <div class="flex items-center gap-2 text-[10px] uppercase tracking-wider font-semibold text-orange-300">
        📅 {{ jourLabel }} — meilleure fenêtre
        <span v-if="estAujourdHui"
          class="ml-auto bg-blue-600/30 border border-blue-500/40 px-1.5 rounded text-blue-300 normal-case tracking-normal">Aujourd'hui</span>
      </div>
      <div class="flex items-center gap-2 text-xs">
        <span class="font-bold text-white">{{ fenDujour.asset }}</span>
        <span class="text-gray-500">{{ fenDujour.heure_debut }}–{{ fenDujour.heure_fin }} UTC</span>
        <span v-if="fenDujour.backtest_winrate !== null" class="ml-auto text-gray-500">
          WR <span :class="fenDujour.backtest_winrate >= 60 ? 'text-emerald-400' : 'text-yellow-400'"
            class="font-semibold">{{ fenDujour.backtest_winrate }}%</span>
        </span>
      </div>
      <div v-if="fenDujour.timing_optimal || fenDujour.precision_nb_occurrences" class="text-[10px] text-gray-600">
        <span v-if="fenDujour.timing_optimal">Pic {{ fenDujour.timing_optimal }}</span>
        <span v-if="fenDujour.precision_nb_occurrences"> · {{ fenDujour.precision_nb_occurrences }} occ.</span>
      </div>
    </section>

    <!-- 2. Top 2 ATR M15 actuel -->
    <section class="border-b border-white/5 pb-1.5 shrink-0">
      <div class="text-[10px] uppercase tracking-wider font-semibold text-orange-300 mb-1">⚡ ATR M15 actuel</div>
      <div v-if="chargementAtr" class="text-[10px] text-gray-600 animate-pulse">Chargement…</div>
      <div v-else-if="topAtr.length" class="flex flex-wrap gap-3">
        <div v-for="a in topAtr" :key="a.asset" class="flex items-center gap-1 text-xs">
          <span class="w-1.5 h-1.5 rounded-full shrink-0" :style="{ background: couleurRatio(a.ratio) }"></span>
          <span class="font-bold text-white">{{ a.asset }}</span>
          <span class="font-mono" :style="{ color: couleurRatio(a.ratio) }">{{ a.ratio.toFixed(0) }}%</span>
        </div>
      </div>
      <div v-else class="text-[10px] text-gray-600 italic">Pas de données ATR</div>
    </section>

    <!-- 3. Prochains créneaux (assets surveillés uniquement) -->
    <div class="text-[10px] uppercase tracking-wider font-semibold text-blue-400 shrink-0">🕐 Prochains créneaux</div>
    <div v-if="chargement" class="text-xs text-gray-600 animate-pulse">Chargement…</div>
    <div v-else-if="!prochainsList.length" class="text-xs text-gray-500 italic">Aucun créneau validé pour vos assets
    </div>
    <div v-else class="flex flex-col gap-1.5 overflow-y-auto flex-1 min-h-0">
      <div v-for="c in prochainsList" :key="c.id"
        class="flex flex-col gap-0.5 border-t border-white/5 pt-1 first:border-0 first:pt-0">
        <div class="flex items-center gap-2 text-xs">
          <span class="font-bold text-white">{{ c.asset }}</span>
          <span class="text-gray-600">{{ c.heure_debut }}–{{ c.heure_fin }} UTC</span>
          <span class="font-mono font-bold ml-auto" :class="reboursCls(c)">{{ rebours(c) }}</span>
        </div>
        <div class="flex items-center gap-2 text-[10px] text-gray-500 flex-wrap">
          <span v-if="c.timing_optimal">Pic <span class="font-mono text-white/70">{{ c.timing_optimal }}</span></span>
          <span v-if="c.precision_atr_pic !== null" class="text-amber-400/80">ATR {{ c.precision_atr_pic?.toFixed(1)
            }}</span>
          <span v-if="c.backtest_winrate !== null">WR <span class="font-semibold"
              :class="c.backtest_winrate >= 60 ? 'text-emerald-400' : c.backtest_winrate >= 50 ? 'text-yellow-400' : 'text-red-400'">{{
                c.backtest_winrate }}%</span></span>
          <span v-if="c.backtest_profit_factor !== null">PF {{ c.backtest_profit_factor?.toFixed(1) }}</span>
          <span v-if="c.whipsaw_minutes" class="text-orange-400 ml-auto">⚠ ws {{ c.whipsaw_minutes }}min</span>
        </div>
        <div v-if="c.llm_raison" class="text-[10px] text-gray-600 italic truncate">{{ c.llm_raison }}</div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { apiService } from '@/services/api.service'
import type { Candle } from '@/services/api.service'
import { useAssetsStore } from '@/stores/assets.store'
import type { StraddleCreneau } from '@/services/api.types'

const JOURS = ['Dimanche', 'Lundi', 'Mardi', 'Mercredi', 'Jeudi', 'Vendredi', 'Samedi']

const assetsStore = useAssetsStore()
const creneaux = ref<StraddleCreneau[]>([])
const chargement = ref(true)
const now = ref(Date.now())
const atrRatios = ref<Record<string, number>>({})
const chargementAtr = ref(true)

// ── ATR helpers ──────────────────────────────────────────────────────────────

function calcAtr(candles: Candle[], periode: number): number {
  if (candles.length < 2) return 0
  const trs = candles.slice(1).map((c, i) => {
    const prev = candles[i].close
    return Math.max(c.high - c.low, Math.abs(c.high - prev), Math.abs(c.low - prev))
  })
  const f = trs.slice(-Math.min(periode, trs.length))
  return f.reduce((s, v) => s + v, 0) / f.length
}

function calcAtrRatio(candles: Candle[]): number {
  if (candles.length < 30) return 0
  const court = calcAtr(candles.slice(-7), 6)
  const long = calcAtr(candles, Math.min(candles.length - 1, 60))
  return long > 0 ? (court / long) * 100 : 100
}

function couleurRatio(r: number): string {
  if (r < 80) return '#10b981'
  if (r < 120) return '#f59e0b'
  return '#ef4444'
}

// ── Computed ─────────────────────────────────────────────────────────────────

const jourUTCAujourdhui = computed(() => new Date().getUTCDay())
const jourLabel = computed(() => JOURS[jourUTCAujourdhui.value] ?? '')

/** Meilleur créneau validé pour le jour courant UTC, trié par winrate. */
const fenDujour = computed<StraddleCreneau | null>(() => {
  const actifs = new Set(assetsStore.assets.map(a => a.id))
  const d = jourUTCAujourdhui.value
  const candidats = creneaux.value.filter(c =>
    c.statut === 'valide' && actifs.has(c.asset) && c.jour_semaine === d
  )
  if (!candidats.length) return null
  return candidats.sort((a, b) =>
    (b.backtest_winrate ?? 0) - (a.backtest_winrate ?? 0) ||
    (b.llm_conviction ?? 0) - (a.llm_conviction ?? 0)
  )[0]
})

const estAujourdHui = computed(() => fenDujour.value?.jour_semaine === jourUTCAujourdhui.value)

/** Top 2 assets par ATR M15 ratio décroissant. */
const topAtr = computed(() =>
  Object.entries(atrRatios.value)
    .map(([asset, ratio]) => ({ asset, ratio }))
    .sort((a, b) => b.ratio - a.ratio)
    .slice(0, 2)
)

// ── Prochains créneaux ───────────────────────────────────────────────────────

function secondesAvant(c: StraddleCreneau): number {
  const [hd, md] = c.heure_debut.split(':').map(Number)
  const base = new Date()
  base.setUTCHours(hd, md ?? 0, 0, 0)
  if (c.jour_semaine !== null) {
    const jourAujourdhuiUTC = new Date().getUTCDay()
    let delta = (c.jour_semaine - jourAujourdhuiUTC + 7) % 7
    if (delta === 0 && base.getTime() <= Date.now()) delta = 7
    base.setUTCDate(base.getUTCDate() + delta)
  } else if (base.getTime() <= Date.now()) {
    base.setUTCDate(base.getUTCDate() + 1)
  }
  return Math.max(0, Math.floor((base.getTime() - Date.now()) / 1000))
}

const prochainsList = computed<StraddleCreneau[]>(() => {
  void now.value
  const actifs = new Set(assetsStore.assets.map(a => a.id))
  return creneaux.value
    .filter(c => c.statut === 'valide' && actifs.has(c.asset))
    .sort((a, b) => secondesAvant(a) - secondesAvant(b))
    .slice(0, 3)
})

function rebours(c: StraddleCreneau): string {
  const s = secondesAvant(c)
  if (s === 0) return 'Maintenant !'
  const h = Math.floor(s / 3600)
  const m = Math.floor((s % 3600) / 60)
  const sec = s % 60
  if (h > 0) return `${h}h ${String(m).padStart(2, '0')}m`
  if (m > 0) return `${m}m ${String(sec).padStart(2, '0')}s`
  return `${sec}s`
}

function reboursCls(c: StraddleCreneau): string {
  const s = secondesAvant(c)
  if (s === 0) return 'text-emerald-400 animate-pulse'
  if (s < 300) return 'text-red-400'
  if (s < 1800) return 'text-yellow-400'
  return 'text-blue-300'
}

// ── Chargements ──────────────────────────────────────────────────────────────

async function charger() {
  try { creneaux.value = await apiService.getStraddleCreneaux() }
  catch { creneaux.value = [] }
  finally { chargement.value = false }
}

async function chargerAtr() {
  chargementAtr.value = true
  const resultats = await Promise.allSettled(
    assetsStore.assets.map(a => apiService.getCandles(a.id, 'M15', 80).then(c => ({ asset: a.id, c })))
  )
  const nouveaux: Record<string, number> = {}
  for (const r of resultats) {
    if (r.status === 'fulfilled') nouveaux[r.value.asset] = calcAtrRatio(r.value.c)
  }
  atrRatios.value = nouveaux
  chargementAtr.value = false
}

let _tick: ReturnType<typeof setInterval> | null = null
let _poll: ReturnType<typeof setInterval> | null = null
let _pollAtr: ReturnType<typeof setInterval> | null = null
onMounted(async () => {
  if (!assetsStore.assets.length) await assetsStore.chargerAssets()
  charger()
  chargerAtr()
  _tick = setInterval(() => { now.value = Date.now() }, 1000)
  _poll = setInterval(charger, 5 * 60_000)
  _pollAtr = setInterval(chargerAtr, 60_000)
})
onUnmounted(() => {
  if (_tick !== null) { clearInterval(_tick); _tick = null }
  if (_poll !== null) { clearInterval(_poll); _poll = null }
  if (_pollAtr !== null) { clearInterval(_pollAtr); _pollAtr = null }
})
</script>

<style scoped>
.glass-bar {
  @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm;
}
</style>
