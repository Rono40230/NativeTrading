<template>
  <div class="space-y-6">
    <div class="flex items-center justify-between flex-wrap gap-3">
      <h1 class="text-2xl font-bold">🌡 Heatmap Volatilité</h1>
      <!-- Sélecteur d'onglets -->
      <div class="flex rounded-lg overflow-hidden border border-white/10">
        <button
          class="px-4 py-2 text-sm font-medium transition-colors"
          :class="onglet === 'atr' ? 'bg-blue-600 text-white' : 'bg-white/5 text-gray-400 hover:bg-white/10'"
          @click="onglet = 'atr'"
        >Heatmap ATR</button>
        <button
          class="px-4 py-2 text-sm font-medium transition-colors"
          :class="onglet === 'horaire' ? 'bg-blue-600 text-white' : 'bg-white/5 text-gray-400 hover:bg-white/10'"
          @click="onglet = 'horaire'"
        >Patterns Horaires</button>
      </div>
    </div>

    <!-- Onglet Heatmap ATR (existant) -->
    <template v-if="onglet === 'atr'">
      <div class="flex items-center justify-between">
        <div class="flex items-center gap-3 ml-auto">
          <span class="text-xs text-gray-400">MAJ auto toutes les 60s</span>
          <button class="btn-sm" :disabled="chargement" @click="actualiser">
            {{ chargement ? '⏳' : '🔄' }} Actualiser
          </button>
        </div>
      </div>

    <!-- Légende -->
    <div class="glass-card p-3 flex items-center gap-4 flex-wrap">
      <span class="text-xs text-gray-400 font-semibold">Volatilité ATR :</span>
      <span v-for="n in legendes" :key="n.label" class="flex items-center gap-1 text-xs text-gray-300">
        <span class="w-4 h-4 rounded-sm" :style="{ background: n.couleur }" />
        {{ n.label }}
      </span>
    </div>

    <!-- Grille -->
    <div class="glass-card p-5">
      <table class="w-full">
        <thead>
          <tr>
            <th class="text-left px-3 py-2 text-gray-400 text-sm">Asset \ TF</th>
            <th v-for="tf in timeframes" :key="tf" class="px-3 py-2 text-center text-sm text-gray-400 font-semibold">
              {{ tf }}
            </th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="asset in assets" :key="asset">
            <td class="px-3 py-2 font-bold text-white">{{ asset }}</td>
            <td
              v-for="tf in timeframes"
              :key="tf"
              class="px-3 py-2"
            >
              <div
                class="rounded-lg p-3 text-center transition-all cursor-default"
                :style="celluleStyle(asset, tf)"
                :title="`ATR ratio: ${celluleValeur(asset, tf).toFixed(1)}%`"
              >
                <div class="text-xs font-bold text-white drop-shadow">
                  {{ celluleLabel(asset, tf) }}
                </div>
                <div class="text-xs text-white/70 mt-0.5">
                  {{ celluleValeur(asset, tf).toFixed(1) }} <span>{{ uniteAsset(asset) }}</span>
                </div>
              </div>
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- Classement mini -->
    <div class="grid grid-cols-1 gap-4 lg:grid-cols-3">
      <div class="glass-card p-4">
        <h3 class="text-sm font-semibold text-gray-400 uppercase tracking-wider mb-3">Plus volatile</h3>
        <div v-for="item in classementVol.slice(0, 3)" :key="item.cle" class="flex justify-between py-1.5 border-b border-white/5 text-sm">
          <span class="text-gray-300">{{ item.asset }} {{ item.tf }}</span>
          <span class="text-red-400 font-semibold">{{ item.atr.toFixed(1) }} {{ uniteAsset(item.asset) }}</span>
        </div>
      </div>
      <div class="glass-card p-4">
        <h3 class="text-sm font-semibold text-gray-400 uppercase tracking-wider mb-3">Moins volatile</h3>
        <div v-for="item in classementVol.slice(-3).reverse()" :key="item.cle" class="flex justify-between py-1.5 border-b border-white/5 text-sm">
          <span class="text-gray-300">{{ item.asset }} {{ item.tf }}</span>
          <span class="text-emerald-400 font-semibold">{{ item.atr.toFixed(1) }} {{ uniteAsset(item.asset) }}</span>
        </div>
      </div>
      <!-- Analyse -->
      <div v-if="analyseAtr" class="glass-card p-4 space-y-2">
        <h3 class="text-sm font-semibold text-gray-400 uppercase tracking-wider mb-3">Analyse</h3>
        <div class="flex justify-between text-sm py-1 border-b border-white/5">
          <span class="text-gray-400">Asset le plus actif</span>
          <span class="text-white font-semibold">{{ analyseAtr.assetActif }}</span>
        </div>
        <div class="flex justify-between text-sm py-1 border-b border-white/5">
          <span class="text-gray-400">Asset le plus calme</span>
          <span class="text-white font-semibold">{{ analyseAtr.assetCalme }}</span>
        </div>
        <!-- TFs actifs par asset -->
        <div v-for="a in assets" :key="a" class="py-1 border-b border-white/5 last:border-0">
          <div class="flex items-center justify-between gap-2">
            <span class="text-gray-400 text-sm shrink-0">{{ a }} actif sur</span>
            <span v-if="analyseAtr.tfsActifsParAsset[a].length" class="flex flex-wrap gap-1 justify-end">
              <span
                v-for="tf in analyseAtr.tfsActifsParAsset[a]"
                :key="tf"
                class="bg-red-500/20 text-red-300 text-[10px] font-mono px-1.5 py-0.5 rounded"
              >{{ tf }}</span>
            </span>
            <span v-else class="text-gray-600 text-xs italic">aucun créneau élevé</span>
          </div>
        </div>
        <div class="mt-2 rounded-lg px-3 py-2 text-xs" :class="analyseAtr.straddleClass">
          {{ analyseAtr.straddleConseil }}
        </div>
      </div>
    </div>
    </template>

    <!-- Onglet Patterns Horaires (S21) -->
    <template v-if="onglet === 'horaire'">
      <HoraireHeatmap />
    </template>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { apiService } from '@/services/api.service'
import type { Candle, AssetInfo } from '@/services/api.service'
import { useAlerteStore } from '@/stores/alerte.store'
import { useAssetsStore } from '@/stores/assets.store'
import HoraireHeatmap from '@/components/common/HoraireHeatmap.vue'

const onglet = ref<'atr' | 'horaire'>('atr')
const alerteStore = useAlerteStore()
const assetsStore = useAssetsStore()
const assets = computed(() => assetsStore.assets.map(a => a.id))
const timeframes = ['M1', 'M5', 'M15', 'M30', 'H1', 'H4', 'D1', 'W1']
const chargement = ref(false)
const donnees = ref<Record<string, number>>({})

/** Unité d'affichage selon le type d'asset. */
function uniteAsset(assetId: string): string {
  const info = assetsStore.assets.find(a => a.id === assetId)
  return info?.type === 'crypto' ? '$' : 'pts'
}

const legendes = [
  { label: 'Faible (<80%)', couleur: '#10b981' },
  { label: 'Modérée (80-120%)', couleur: '#f59e0b' },
  { label: 'Élevée (>120%)', couleur: '#ef4444' },
]

function calcAtr(candles: Candle[], periode = 14): number {
  if (candles.length < 2) return 0
  const trs = candles.slice(1).map((c, i) => {
    const prev = candles[i].close
    return Math.max(c.high - c.low, Math.abs(c.high - prev), Math.abs(c.low - prev))
  })
  const fenetre = trs.slice(-Math.min(periode, trs.length))
  return fenetre.reduce((s, v) => s + v, 0) / fenetre.length
}

function calcAtrRatio(candles: Candle[]): number {
  if (candles.length < 30) return 0
  // ATR court terme : moyenne des 6 derniers TR (7 bougies)
  const atrActuel = calcAtr(candles.slice(-7), 6)
  // ATR long terme : moyenne des jusqu'à 60 derniers TR (fenêtre large)
  const atrMoyen = calcAtr(candles, Math.min(candles.length - 1, 60))
  return atrMoyen > 0 ? (atrActuel / atrMoyen) * 100 : 100
}

function cle(asset: string, tf: string): string { return `${asset}_${tf}` }

function celluleValeur(asset: string, tf: string): number {
  return donnees.value[cle(asset, tf)] ?? 0
}

function couleurRatio(ratio: number): string {
  if (ratio < 80) return '#10b981'
  if (ratio < 120) return '#f59e0b'
  return '#ef4444'
}

function celluleStyle(asset: string, tf: string) {
  const ratio = celluleValeur(asset, tf)
  const couleur = couleurRatio(ratio)
  return { background: `${couleur}25`, borderColor: `${couleur}60`, border: '1px solid', opacity: chargement.value ? 0.5 : 1 }
}

function celluleLabel(asset: string, tf: string): string {
  const r = celluleValeur(asset, tf)
  if (r === 0) return '—'
  if (r < 80) return '🟢 Calme'
  if (r < 120) return '🟡 Modéré'
  return '🔴 Élevé'
}

const classementVol = computed(() => {
  const items = assets.value.flatMap(a => timeframes.map(tf => ({
    cle: cle(a, tf), asset: a, tf, atr: donnees.value[cle(a, tf)] ?? 0
  })))
  return items.filter(i => i.atr > 0).sort((a, b) => b.atr - a.atr)
})

const analyseAtr = computed(() => {
  const items = classementVol.value
  if (!items.length) return null

  // ATR moyen par asset (toutes TF confondues)
  const moyParAsset = assets.value.map(a => {
    const pts = items.filter(i => i.asset === a)
    return { asset: a, moy: pts.length ? pts.reduce((s, i) => s + i.atr, 0) / pts.length : 0 }
  }).filter(x => x.moy > 0).sort((a, b) => b.moy - a.moy)

  const assetActif = moyParAsset[0]?.asset ?? '—'
  const assetCalme = moyParAsset[moyParAsset.length - 1]?.asset ?? '—'

  // TFs en volatilité élevée (>120%) par asset
  const tfsActifsParAsset: Record<string, string[]> = {}
  for (const a of assets.value) {
    tfsActifsParAsset[a] = items
      .filter(i => i.asset === a && i.atr > 120)
      .map(i => i.tf)
  }

  // Combien de cellules dépassent 120% (Élevé) ?
  const nbEleve = items.filter(i => i.atr > 120).length
  const topRatio = items[0]?.atr ?? 0
  let straddleConseil: string
  let straddleClass: string
  if (topRatio > 120) {
    straddleConseil = `Straddle favorable — ${nbEleve} créneau${nbEleve > 1 ? 'x' : ''} en volatilité élevée (>${120}%).`
    straddleClass = 'bg-red-500/10 border border-red-500/30 text-red-300'
  } else if (topRatio > 90) {
    straddleConseil = 'Volatilité modérée — surveiller avant d\'entrer en Straddle.'
    straddleClass = 'bg-amber-500/10 border border-amber-500/30 text-amber-300'
  } else {
    straddleConseil = 'Marché calme — privilégier SMC Directionnel sur breakout.'
    straddleClass = 'bg-emerald-500/10 border border-emerald-500/30 text-emerald-300'
  }

  return { assetActif, assetCalme, tfsActifsParAsset, straddleConseil, straddleClass }
})

async function actualiser() {
  chargement.value = true
  const paires = assets.value.flatMap(a => timeframes.map(tf => ({ a, tf })))
  const resultats = await Promise.allSettled(
    paires.map(({ a, tf }) => apiService.getCandles(a, tf, 80).then(c => ({ a, tf, c })))
  )
  for (const r of resultats) {
    if (r.status === 'fulfilled') {
      const { a, tf, c } = r.value
      donnees.value[cle(a, tf)] = calcAtrRatio(c)
    }
  }
  chargement.value = false
}

let intervalId: ReturnType<typeof setInterval> | null = null
onMounted(async () => {
  try {
    await actualiser()
    intervalId = setInterval(actualiser, 60_000)
  } catch (e: unknown) {
    alerteStore.afficherErreur(`Heatmap: ${(e as Error).message}`)
  }
})
onUnmounted(() => { if (intervalId) clearInterval(intervalId) })
</script>

<style scoped>
.glass-card { @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm; }
.btn-sm { @apply bg-gray-700 hover:bg-gray-600 disabled:opacity-40 text-white text-sm px-3 py-1.5 rounded-lg transition-all; }
</style>

