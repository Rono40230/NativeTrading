<template>
  <!-- REFONTE COCKPIT (24/09) — dessin propriétaire : ① bandeau annonceur
       (voyants + sessions + alertes) ; ⓪ la planche MÉTÉO DU MARCHÉ (jauges
       linéaires + ticker) ; ⓪ LE PANNEAU : bloc STRATÉGIES gravé, 4
       colonnes-instruments pleine largeur (commandes en marge gauche, jauge,
       écran courbe dessous) + instrumentation à droite ; ④ rangée basse
       comms/nav. -->
  <div class="flex flex-col gap-2 h-[calc(100vh-5.5rem)] overflow-y-auto xl:overflow-hidden">

    <!-- ① Bandeau : les 3 badges d'instrumentation (calendrier, créneaux,
         radar ATR) — lampes systèmes et alertes vivent au pedestal -->
    <BandeauAnnonceur class="shrink-0 sticky top-0 z-30" />

    <!-- ①½ La rangée de garde : les six places du monde -->
    <MarketClocks />

    <!-- ⓪ La planche météo : les instruments de surveillance du marché -->
    <PlancheMeteo />

    <!-- ⓪ LE PANNEAU — bloc STRATÉGIES gravé, pleine largeur (l'aside a
         quitté le dashboard 26/09 : ses trois blocs vivent dans les badges
         du bandeau et leurs modales) -->
    <div class="flex-1 min-h-0 overflow-y-auto">
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

    <!-- ④ Pedestal COMMUNICATIONS & NAVIGATION : la console centrale du
        cockpit — les canaux et outils (presse, rapport, IA, graphiques,
        système) en interrupteurs Korry gravés, hauteur doublée (owner
        25/09) pour des fenêtres digitales plus généreuses. -->
    <div class="relative shrink-0 rounded-xl border border-white/10 bg-white/[0.02] p-2 pt-5">
      <span class="absolute top-1 left-2.5 px-2 text-[9px] font-bold tracking-[0.25em] text-white/60 bg-[#111827] border border-white/10 rounded">COMMUNICATIONS &amp; NAVIGATION</span>
      <div class="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-5 gap-2 items-stretch">
        <DashboardTuilesNavigation :ids="['presse']" />
        <DashboardRapportActivite />
        <DashboardTuilesNavigation :ids="['ia']" />
        <DashboardTuilesNavigation :ids="['graphiques']" />
        <DashboardTuilesNavigation :ids="['systeme']" />
      </div>
    </div>

  </div>
</template>

<script setup lang="ts">
import { onMounted, onUnmounted } from 'vue'
import { useSignalStore } from '@/stores/signal.store'
import { useSettingsStore } from '@/stores/settings.store'
import { usePrixStore } from '@/stores/prix.store'
import { useSentimentStore } from '@/stores/sentiment.store'
import { apiService } from '@/services/api.service'
import { useAssetsStore } from '@/stores/assets.store'
import BandeauAnnonceur from '@/components/common/BandeauAnnonceur.vue'
import MarketClocks from '@/components/common/MarketClocks.vue'
import PlancheMeteo from '@/components/common/PlancheMeteo.vue'
import ColonneStrategie from '@/components/common/ColonneStrategie.vue'
import DashboardRapportActivite from '@/components/common/DashboardRapportActivite.vue'
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
const sentimentStore = useSentimentStore()
const assetsStore = useAssetsStore()

onMounted(async () => {
  try { await apiService.healthCheck() } catch { /* silencieux */ }
  await Promise.allSettled([
    signalStore.chargerSignaux(10),
    signalStore.chargerPrediction(settingsStore.assetActif, settingsStore.timeframeActif),
  ])
  const tousLesAssets = assetsStore.assets.map(a => a.id)
  if (tousLesAssets.length > 0) prixStore.demarrer(tousLesAssets)
  sentimentStore.demarrer()
})

onUnmounted(() => {
  sentimentStore.arreter()
})
</script>
