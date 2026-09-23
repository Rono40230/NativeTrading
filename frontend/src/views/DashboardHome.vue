<template>
  <!-- Layout 2 zones : contenu élargi | colonne latérale unique (décision
       14/09 : l'ancienne colonne gauche est fusionnée dans la droite, dans
       l'ordre du propriétaire — Data&IA, Sentiment, Calendrier, Presse,
       Rapport d'activité, Graphiques, IA, Données). -->
  <div class="flex flex-col gap-3">
    <!-- Hauteur définie sur la rangée : les zones ne dépassent JAMAIS la
         page — le centre (cartes stratégies) scrolle en bloc, la colonne
         latérale scrolle en interne (8 blocs empilés). -->
    <div class="flex gap-3 h-[calc(100vh-5.5rem)]">

      <!-- Contenu principal : horloges en TÊTE, créneaux, puis blocs par
           stratégie (courbe R + stats + signaux en cours). -->
      <div class="flex-1 min-w-0 flex flex-col gap-2 min-h-0 overflow-hidden pb-1">
        <MarketClocks class="shrink-0 h-[130px]" />
        <CreneauxVolatiliteBloc class="shrink-0" />
        <RadarAtrBloc class="shrink-0" />
        <div class="flex-1 min-h-0">
          <DashboardStrategiesBlocs />
        </div>
      </div>

      <!-- Colonne latérale : TOUS les blocs de suivi, scroll interne -->
      <aside class="w-96 shrink-0 h-full min-h-0 flex flex-col gap-3 overflow-y-auto pr-0.5">
        <DashboardSystemStatus
          :backend-ok="backendOk"
          :btc-prix="btcPrix"
          :ollama-ok="ollamaOk"
          class="shrink-0"
        />
        <SentimentMarche class="shrink-0" />
        <EconomicCalendar class="shrink-0" />
        <DashboardTuilesNavigation :ids="['presse']" class="shrink-0" />
        <!-- Rangées de boutons (14/09) : rapport + IA, puis graphiques +
             données — mêmes hauteurs, cartes épurées cliquables. -->
        <div class="grid grid-cols-2 gap-2 shrink-0">
          <DashboardRapportActivite />
          <DashboardTuilesNavigation :ids="['ia']" />
        </div>
        <div class="grid grid-cols-2 gap-2 shrink-0 items-start">
          <DashboardTuilesNavigation :ids="['graphiques']" />
          <DashboardTuilesNavigation :ids="['systeme']" />
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
import { useSentimentStore } from '@/stores/sentiment.store'
import { apiService } from '@/services/api.service'
import { useAssetsStore } from '@/stores/assets.store'
import MarketClocks from '@/components/common/MarketClocks.vue'
import EconomicCalendar from '@/components/common/EconomicCalendar.vue'
import SentimentMarche from '@/components/common/SentimentMarche.vue'
import DashboardSystemStatus from '@/components/common/DashboardSystemStatus.vue'
import DashboardRapportActivite from '@/components/common/DashboardRapportActivite.vue'
import CreneauxVolatiliteBloc from '@/components/common/CreneauxVolatiliteBloc.vue'
import RadarAtrBloc from '@/components/common/RadarAtrBloc.vue'
import DashboardStrategiesBlocs from '@/components/common/DashboardStrategiesBlocs.vue'
import DashboardTuilesNavigation from '@/components/common/DashboardTuilesNavigation.vue'

const signalStore = useSignalStore()
const settingsStore = useSettingsStore()
const prixStore = usePrixStore()
const { variationLive } = storeToRefs(prixStore)
const sentimentStore = useSentimentStore()
const assetsStore = useAssetsStore()

const mlPret = computed(() => signalStore.prediction?.modele_pret ?? false)
const backendOk = ref(false)
const ollamaOk = ref<boolean | null>(null)


const btcPrix = computed(() => prixStore.getPrix('BTC'))

let intervalStatuts: ReturnType<typeof setInterval> | null = null

async function rafraichirStatuts() {
  try { const ia = await apiService.statutIA(); ollamaOk.value = ia.ollama_disponible } catch { /* silencieux */ }
}


onMounted(async () => {
  try { await apiService.healthCheck(); backendOk.value = true } catch { backendOk.value = false }
  try { const ia = await apiService.statutIA(); ollamaOk.value = ia.ollama_disponible } catch { ollamaOk.value = false }
  // Re-vérification après 6s
  setTimeout(rafraichirStatuts, 6000)
  await Promise.allSettled([
    signalStore.chargerSignaux(10),
    signalStore.chargerPrediction(settingsStore.assetActif, settingsStore.timeframeActif),
  ])
  // Prix temps réel : le store WebSocket suffit (la plomberie de bougies du
  // bloc surveillance est partie avec lui — 5 fetchs/asset/min économisés).
  const tousLesAssets = assetsStore.assets.map(a => a.id)
  if (tousLesAssets.length > 0) prixStore.demarrer(tousLesAssets)
  sentimentStore.demarrer()
  intervalStatuts = setInterval(rafraichirStatuts, 30000)
})

onUnmounted(() => {
  if (intervalStatuts !== null) clearInterval(intervalStatuts)
  // prixStore reste actif pour les autres vues (Rockets, etc.)
  sentimentStore.arreter()
})
</script>

<style scoped>
.glass-card { @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm; }
.label { @apply text-xs text-white font-medium; }
.kpi-value { @apply text-2xl font-bold text-white mt-1; }
</style>
