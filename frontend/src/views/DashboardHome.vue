<template>
  <!-- Layout 2 colonnes : contenu | sentiment+calendrier (la revue de presse
       vit désormais dans sa propre vue /presse avec liseuse intégrée) -->
  <div class="flex flex-col gap-3">
    <!-- Bandeau alerte critique (conditionnel) -->
    <AlerteBandeau />

    <div class="flex gap-3">

    <!-- Contenu principal -->
    <div class="flex-1 min-w-0 flex flex-col gap-2 h-[calc(100vh-3rem)] overflow-hidden pb-1">

      <!-- En-tête : Horloges + System Status -->
      <div class="flex gap-2 shrink-0 h-[140px] mb-1">
        <!-- Clocks (pleine largeur — le statut système vit dans la colonne gauche) -->
        <div class="flex-1 min-w-0"><MarketClocks class="h-full" /></div>
      </div>

      <div class="flex gap-2 flex-1 min-h-0">
        <!-- Colonne gauche : Surveillance assets + Setups en formation -->
        <div class="w-64 shrink-0 flex flex-col gap-2 min-h-0">
          <DashboardSystemStatus
            :backend-ok="backendOk"
            :btc-prix="btcPrix"
            :ollama-ok="ollamaOk"
            :ml-pret="mlPret"
            class="shrink-0"
          />
          <SurveillanceAssets class="shrink-0" :assets="assetsDisplay.slice(0, 5)" :chargement="assetsAvecPrix.length === 0" />
          <CreneauxVolatiliteBloc class="shrink-0" />
        </div>

        <!-- Centre : blocs par stratégie (étape 3 — architecture verticale).
             Chaque bloc : courbe des trades clôturés (R cumulé) + stats +
             signaux en cours + badge d'état du registre. -->
        <div class="flex-1 min-w-0 min-h-0">
          <DashboardStrategiesBlocs />
        </div>
      </div>
    </div>

      <!-- Colonne droite : Sentiment + Calendrier -->
      <aside class="w-64 shrink-0 sticky top-0 h-[calc(100vh-3rem)] flex flex-col gap-3">
        <SentimentMarche class="shrink-0" />
        <div class="flex-1 min-h-0">
          <EconomicCalendar class="h-full" />
        </div>
      </aside>

    </div>
  </div>

</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { storeToRefs } from 'pinia'
import { useSignalStore } from '@/stores/signal.store'
import { useSettingsStore } from '@/stores/settings.store'
import { usePrixStore } from '@/stores/prix.store'
import { useNewsStore } from '@/stores/news.store'
import { useSentimentStore } from '@/stores/sentiment.store'
import { apiService } from '@/services/api.service'
import type { Candle } from '@/services/api.service'
import { useAssetsStore } from '@/stores/assets.store'
import MarketClocks from '@/components/common/MarketClocks.vue'
import EconomicCalendar from '@/components/common/EconomicCalendar.vue'
import SentimentMarche from '@/components/common/SentimentMarche.vue'
import AlerteBandeau from '@/components/common/AlerteBandeau.vue'
import DashboardSystemStatus from '@/components/common/DashboardSystemStatus.vue'
import SurveillanceAssets from '@/components/common/SurveillanceAssets.vue'
import CreneauxVolatiliteBloc from '@/components/common/CreneauxVolatiliteBloc.vue'
import DashboardStrategiesBlocs from '@/components/common/DashboardStrategiesBlocs.vue'

type VariationsMultiTF = { h1: number | null; h4: number | null; d1: number | null; w1: number | null; m1: number | null }
type AssetAvecPrix = { id: string; prix: number | null; variation: number | null; variationsMultiTF: VariationsMultiTF | null; clotures: Record<string, number[]>; chargement: boolean }

const signalStore = useSignalStore()
const settingsStore = useSettingsStore()
const prixStore = usePrixStore()
const { variationLive } = storeToRefs(prixStore)
const newsStore = useNewsStore()
const sentimentStore = useSentimentStore()
const assetsStore = useAssetsStore()

const mlPret = computed(() => signalStore.prediction?.modele_pret ?? false)
const backendOk = ref(false)
const ollamaOk = ref<boolean | null>(null)
const assetsAvecPrix = ref<AssetAvecPrix[]>([])
  

const assetsDisplay = computed(() => {
  const sent = sentimentStore.data
  const sentMap: Record<string, number> = {}
  if (sent) {
    for (const cat of [sent.usa, sent.europe, sent.matieres_premieres, sent.cryptos]) {
      for (const e of cat || []) {
        if (e.nom === 'Bitcoin') sentMap['BTC'] = e.variation_pct
        if (e.nom === 'Ethereum') sentMap['ETH'] = e.variation_pct
        if (e.nom === 'Or') sentMap['XAUUSD'] = e.variation_pct
        if (e.nom === 'Argent') sentMap['XAGUSD'] = e.variation_pct
        if (e.nom === 'S&P500') sentMap['SP500'] = e.variation_pct
        if (e.nom === 'Nasdaq') sentMap['NASDAQ'] = e.variation_pct
        if (e.nom === 'Dow Jones') sentMap['DOW'] = e.variation_pct
        if (e.nom === 'Dax') sentMap['DAX'] = e.variation_pct
        if (e.nom === 'Cac 40') sentMap['CAC40'] = e.variation_pct
        if (e.nom === 'Pétrole') sentMap['OIL'] = e.variation_pct
      }
    }
  }
  return assetsAvecPrix.value.map(a => {
    // Si on a la vraie variation daily 24h via Yahoo/Binance dans le sentiment, on priorise
    const varFinale = sentMap[a.id] !== undefined ? sentMap[a.id] : a.variationsMultiTF?.d1 ?? null
    return {
      ...a,
      prix: prixStore.getPrix(a.id) ?? a.prix,
      variation: varFinale,
      chargement: a.chargement && prixStore.getPrix(a.id) === null,
    }
  })
})
const btcPrix = computed(() => prixStore.getPrix('BTC') ?? assetsAvecPrix.value.find(a => a.id === 'BTC')?.prix ?? null)

let intervalPrix: ReturnType<typeof setInterval> | null = null
let intervalStatuts: ReturnType<typeof setInterval> | null = null

async function rafraichirStatuts() {
  try { const ia = await apiService.statutIA(); ollamaOk.value = ia.ollama_disponible } catch { /* silencieux */ }
}

async function chargerPrixActifs() {
  function calcVar(candles: Candle[], idxOld = -2, idxNew = -1): number | null {
    const a = candles.at(idxNew)?.close
    const b = candles.at(idxOld)?.close
    return a != null && b != null && b !== 0 ? ((a - b) / b) * 100 : null
  }
  try {
    const liste = assetsStore.assets
    assetsAvecPrix.value = liste.map(a => ({ id: a.id, prix: null, variation: null, variationsMultiTF: null, clotures: {}, chargement: true }))
    await Promise.allSettled(liste.map(async (a, i) => {
      try {
        const [bM15, bH1, bH4, bD1, bW1] = await Promise.allSettled([
          apiService.getCandles(a.id, 'M15', 2),
          apiService.getCandles(a.id, 'H1', 48),
          apiService.getCandles(a.id, 'H4', 30),
          apiService.getCandles(a.id, 'D1', 32),
          apiService.getCandles(a.id, 'W1', 20),
        ])
        const m15 = bM15.status === 'fulfilled' ? bM15.value : []
        const h1  = bH1.status  === 'fulfilled' ? bH1.value  : []
        const h4  = bH4.status  === 'fulfilled' ? bH4.value  : []
        const d1  = bD1.status  === 'fulfilled' ? bD1.value  : []
        const w1  = bW1.status  === 'fulfilled' ? bW1.value  : []
        const curr = m15.at(-1)?.close ?? null
        const prev = m15.at(-2)?.close ?? null
        assetsAvecPrix.value[i] = {
          id: a.id,
          prix: curr,
          variation: curr != null && prev != null && prev !== 0 ? ((curr - prev) / prev) * 100 : null,
          variationsMultiTF: {
            h1: calcVar(h1), h4: calcVar(h4), d1: calcVar(d1), w1: calcVar(w1),
            m1: d1.length >= 2 ? calcVar(d1, 0, -1) : null,
          },
          clotures: { h1: h1.map(c => c.close), h4: h4.map(c => c.close), d1: d1.map(c => c.close), w1: w1.map(c => c.close) },
          chargement: false,
        }
      } catch { assetsAvecPrix.value[i] = { id: a.id, prix: null, variation: null, variationsMultiTF: null, clotures: {}, chargement: false } }
    }))
  } catch { /* silencieux */ }
}

onMounted(async () => {
  try { await apiService.healthCheck(); backendOk.value = true } catch { backendOk.value = false }
  try { const ia = await apiService.statutIA(); ollamaOk.value = ia.ollama_disponible } catch { ollamaOk.value = false }
  // Re-vérification après 6s
  setTimeout(rafraichirStatuts, 6000)
  await Promise.allSettled([
    chargerPrixActifs(),
    signalStore.chargerSignaux(10),
    signalStore.chargerPrediction(settingsStore.assetActif, settingsStore.timeframeActif),
  ])
  const tousLesAssets = assetsAvecPrix.value.map(a => a.id)
  if (tousLesAssets.length > 0) prixStore.demarrer(tousLesAssets)
  newsStore.demarrerPolling()
  sentimentStore.demarrer()
  intervalPrix = setInterval(chargerPrixActifs, 60000)
  intervalStatuts = setInterval(rafraichirStatuts, 30000)
})

onUnmounted(() => {
  if (intervalPrix !== null) clearInterval(intervalPrix)
  if (intervalStatuts !== null) clearInterval(intervalStatuts)
  // prixStore reste actif pour les autres vues (Rockets, etc.)
  newsStore.arreterPolling()
  sentimentStore.arreter()
})
</script>

<style scoped>
.glass-card { @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm; }
.label { @apply text-xs text-gray-400 font-medium; }
.kpi-value { @apply text-2xl font-bold text-white mt-1; }
</style>
