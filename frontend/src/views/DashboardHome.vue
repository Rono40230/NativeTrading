<template>
  <!-- REFONTE COCKPIT (24/09) — dessin propriétaire : ① bandeau annonceur
       (voyants + sessions + alertes) ; ⓪ la planche MÉTÉO DU MARCHÉ (jauges
       linéaires + ticker) ; ⓪ LE PANNEAU : bloc STRATÉGIES gravé, 4
       colonnes-instruments pleine largeur (commandes en marge gauche, jauge,
       écran courbe dessous) + instrumentation à droite ; ④ rangée basse
       comms/nav. -->
  <div class="flex flex-col gap-2 h-[calc(100vh-5.5rem)] overflow-y-auto xl:overflow-hidden">

    <!-- ① Bandeau annonceur (sticky quand la page défile sous xl) -->
    <BandeauAnnonceur
      :backend-ok="backendOk"
      :btc-prix="btcPrix"
      :ollama-ok="ollamaOk"
      class="shrink-0 sticky top-0 z-30"
    />

    <!-- ①½ La rangée de garde : les six places du monde -->
    <MarketClocks />

    <!-- ⓪ La planche météo : les instruments de surveillance du marché -->
    <PlancheMeteo />

    <!-- ⓪ LE PANNEAU — bloc STRATÉGIES gravé + l'instrumentation -->
    <div class="flex-1 min-h-0 flex flex-col xl:flex-row gap-3 overflow-y-auto xl:overflow-hidden">
      <div class="flex-1 min-w-0 overflow-y-auto">
        <div class="relative rounded-xl border border-white/10 bg-white/[0.02] p-2.5 pt-5">
          <span class="absolute top-1 left-2.5 px-2 text-[9px] font-bold tracking-[0.25em] text-white/60 bg-[#111827] border border-white/10 rounded">STRATÉGIES</span>
          <div class="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-4 gap-2.5">
            <ColonneStrategie
              v-for="s in STRATEGIES"
              :key="s.id"
              v-bind="s"
            />
          </div>
        </div>
      </div>

      <!-- Instrumentation : radar, créneaux, calendrier -->
      <aside class="xl:w-80 shrink-0 flex flex-col gap-3 xl:min-h-0 xl:overflow-y-auto pr-0.5">
        <RadarAtrBloc class="shrink-0" />
        <CreneauxVolatiliteBloc class="shrink-0" />
        <EconomicCalendar class="shrink-0" />
      </aside>
    </div>

    <!-- ④ Rangée basse : comms + navigation (le bas du glass cockpit) -->
    <div class="shrink-0 grid grid-cols-2 md:grid-cols-3 lg:grid-cols-5 gap-2">
      <DashboardTuilesNavigation :ids="['presse']" />
      <DashboardRapportActivite />
      <DashboardTuilesNavigation :ids="['ia']" />
      <DashboardTuilesNavigation :ids="['graphiques']" />
      <DashboardTuilesNavigation :ids="['systeme']" />
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
import EconomicCalendar from '@/components/common/EconomicCalendar.vue'
import BandeauAnnonceur from '@/components/common/BandeauAnnonceur.vue'
import MarketClocks from '@/components/common/MarketClocks.vue'
import PlancheMeteo from '@/components/common/PlancheMeteo.vue'
import ColonneStrategie from '@/components/common/ColonneStrategie.vue'
import DashboardRapportActivite from '@/components/common/DashboardRapportActivite.vue'
import CreneauxVolatiliteBloc from '@/components/common/CreneauxVolatiliteBloc.vue'
import RadarAtrBloc from '@/components/common/RadarAtrBloc.vue'
import DashboardTuilesNavigation from '@/components/common/DashboardTuilesNavigation.vue'

/// Le panneau : une colonne-instrument par stratégie, même ordre partout.
const STRATEGIES = [
  { id: 'SMC', nom: 'SMC', icone: '📐', route: '/smc', teinte: 'bg-blue-500/10 border-blue-500/25' },
  { id: 'straddle', nom: 'Straddle', icone: '⚡', route: '/straddle', teinte: 'bg-amber-500/10 border-amber-500/25' },
  { id: 'rockets', nom: 'Rockets', icone: '🚀', route: '/rockets', teinte: 'bg-orange-500/10 border-orange-500/25' },
  { id: 'kdj_halftrend', nom: 'KDJ', icone: '📈', route: '/kdj', teinte: 'bg-cyan-500/10 border-cyan-500/25' },
]

const signalStore = useSignalStore()
const settingsStore = useSettingsStore()
const prixStore = usePrixStore()
const { variationLive } = storeToRefs(prixStore)
const sentimentStore = useSentimentStore()
const assetsStore = useAssetsStore()

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
  setTimeout(rafraichirStatuts, 6000)
  await Promise.allSettled([
    signalStore.chargerSignaux(10),
    signalStore.chargerPrediction(settingsStore.assetActif, settingsStore.timeframeActif),
  ])
  const tousLesAssets = assetsStore.assets.map(a => a.id)
  if (tousLesAssets.length > 0) prixStore.demarrer(tousLesAssets)
  sentimentStore.demarrer()
  intervalStatuts = setInterval(rafraichirStatuts, 30000)
})

onUnmounted(() => {
  if (intervalStatuts !== null) clearInterval(intervalStatuts)
  sentimentStore.arreter()
})
</script>
