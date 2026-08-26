<template>
  <div class="relative h-full min-h-0">

    <!-- ── Zone graphique (plein format) ────────────────────────────── -->
    <div class="flex flex-col gap-4 w-full h-full min-h-0">
      <!-- Dernier prix + variation + metriques + sélecteurs -->
      <ChartPrixStats :dernier-prix="dernierPrix" :variation="variation" :stats="stats" :selected-asset="selectedAsset"
        :selected-timeframe="selectedTimeframe" :ws-connecte="marketStore.wsConnecte" :assets="assets"
        :timeframes="timeframes" @changer-asset="changerAsset" @changer-timeframe="changerTimeframe" />

      <!-- Canvas TradingView -->
      <div class="glass-card flex-1 min-h-0" style="min-height: 350px; position: relative;">
        <div v-if="marketStore.erreur"
          class="absolute inset-0 z-10 flex items-center justify-center bg-black/60 text-red-400 text-sm rounded-xl">
          ⚠ {{ marketStore.erreur }}
        </div>
        <div v-if="marketStore.erreurWs && !marketStore.wsConnecte"
          class="absolute bottom-2 left-2 z-10 px-3 py-1 rounded bg-yellow-900/70 text-yellow-300 text-xs border border-yellow-700/40">
          ⚠ {{ marketStore.erreurWs }}
        </div>
        <div v-if="marketStore.chargement"
          class="absolute inset-0 z-10 flex items-center justify-center bg-black/40 text-gray-400 text-sm rounded-xl">
          <span class="animate-pulse">Chargement des bougies...</span>
        </div>
        <div ref="chartContainer" class="w-full h-full" style="position: relative;" />
        <!-- Barre d'outils de dessin (superposée au chart, à gauche) -->
        <div style="right: 92px; bottom: 36px;" class="absolute z-20 flex flex-col gap-1 p-1 rounded-lg bg-slate-900/80 backdrop-blur border border-white/10 shadow-lg">
          <button v-for="t in outilsDessin" :key="t.outil"
            :title="t.titre + (dessins.outil.value === t.outil ? ' (actif — Échap pour désactiver)' : '')"
            :class="[
              'w-8 h-8 rounded-md text-sm flex items-center justify-center transition-colors',
              dessins.outil.value === t.outil
                ? 'bg-blue-600/40 text-blue-200 border border-blue-400/50'
                : 'text-slate-400 hover:text-slate-100 hover:bg-white/10 border border-transparent',
            ]"
            @click="dessins.choisirOutil(t.outil)"
          >{{ t.icone }}</button>
          <div class="h-px bg-white/10 mx-1" />
          <button title="Effacer tous les dessins de cet asset"
            class="w-8 h-8 rounded-md text-sm flex items-center justify-center text-slate-400 hover:text-red-300 hover:bg-red-500/10 transition-colors"
            @click="dessins.toutEffacer()"
          >🗑</button>
        </div>
        <EcoCalTooltip :annonce="tooltipAnnonce" :x="tooltipX" :y="tooltipY" />
        <TendanceMultiTF v-if="settingsStore.indicateurs.kasperTendance" :key="selectedAsset + '_' + selectedTimeframe"
          :asset="selectedAsset" :timeframe="selectedTimeframe"
          :periode-rapide="settingsStore.indicateurs.kasperPeriodeRapide"
          :periode-lente="settingsStore.indicateurs.kasperPeriodeLente"
          :mode-calcul="settingsStore.indicateurs.kasperModeCalcul" />
      </div>

      <!-- Sous-graphique RSI séparé -->
      <div v-if="settingsStore.indicateurs.rsi" ref="rsiContainer" class="glass-card"
        style="height: 140px; position: relative;" />

      <!-- Sous-graphique MACD séparé -->
      <div v-if="settingsStore.indicateurs.macd" ref="macdContainer" class="glass-card"
        style="height: 140px; position: relative;" />

      <!-- Sous-graphique ATR séparé -->
      <div v-if="settingsStore.indicateurs.atr" ref="atrContainer" class="glass-card"
        style="height: 110px; position: relative;" />

      <!-- Panneau indicateurs (techniques + SMC) -->
      <IndicatorPanel v-model="settingsStore.indicateurs" :chargement="marketStore.chargement"
        @appliquer="chargerIndicateurs" @actualiser="actualiser">
        <template #apres-smc>
          <button
            class="px-2.5 py-1 rounded-md border transition-colors bg-purple-600/20 border-purple-500/30 text-purple-300 hover:bg-purple-600/30 disabled:opacity-40 text-xs font-medium"
            :disabled="signalStore.analyseIaChargement" @click="lancerAnalyseSmc">{{ signalStore.analyseIaChargement ? '🔍 Analyse...' : '🔍 Analyse SMC' }}</button>
        </template>
      </IndicatorPanel>

    </div>

    <!-- Sidebar IA (toggle + drawer) -->
    <ChartSidebarIA :asset="selectedAsset" :timeframe="selectedTimeframe" :open="sidebarIA"
      @toggle="sidebarIA = !sidebarIA" />

    <!-- Modales (hors flux) -->
    <ChartAnalyseSmcModal
      :open="analyseSmcOuverte"
      :asset="selectedAsset"
      :timeframe="selectedTimeframe"
      :score-smc="signalStore.scoreSmc"
      :prix-entree="prixEntreeSnapshot"
      :sl-analyse="signalStore.slAnalyse"
      :tp1-analyse="signalStore.tp1Analyse"
      :tp2-analyse="signalStore.tp2Analyse"
      :chargement="signalStore.analyseIaChargement"
      :analyse-texte="signalStore.analyseIaTexte"
      @close="analyseSmcOuverte = false"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, nextTick, onMounted, onUnmounted, watch } from 'vue'
import { useChartStats } from '@/composables/useChartStats'
import { useChartTradingView } from '@/composables/useChartTradingView'
import { useMarketStore } from '@/stores/market.store'
import { useSettingsStore } from '@/stores/settings.store'
import { useChartIndicators } from '@/composables/useChartIndicators'
import { limitPourTimeframe } from '@/composables/useChartLimite'
import { useSmcV12Overlay } from '@/composables/useSmcV12Overlay'
import { useChartDessins } from '@/composables/useChartDessins'
import { useChartEcoCal } from '@/composables/useChartEcoCal'
import EcoCalTooltip from '@/components/common/EcoCalTooltip.vue'

import { useChartOrchestration } from '@/composables/useChartOrchestration'
import { useSignalStore } from '@/stores/signal.store'
import { apiService } from '@/services/api.service'
import ChartSidebarIA from '@/components/common/ChartSidebarIA.vue'
import ChartAnalyseSmcModal from '@/components/common/ChartAnalyseSmcModal.vue'
import IndicatorPanel from '@/components/common/IndicatorPanel.vue'
import TendanceMultiTF from '@/components/common/TendanceMultiTF.vue'
import ChartPrixStats from '@/components/common/ChartPrixStats.vue'

const marketStore = useMarketStore()
const settingsStore = useSettingsStore()
const signalStore = useSignalStore()

const timeframes = ['M1', 'M5', 'M15', 'M30', 'H1', 'H4', 'D1', 'W1']
const selectedAsset = ref(settingsStore.assetActif)
const selectedTimeframe = ref(settingsStore.timeframeActif)
const chartContainer = ref<HTMLElement | null>(null)
const rsiContainer = ref<HTMLElement | null>(null)
const macdContainer = ref<HTMLElement | null>(null)
const atrContainer = ref<HTMLElement | null>(null)

const bougies = computed(() =>
  marketStore.getBougies(selectedAsset.value, selectedTimeframe.value)
)

const { dernierPrix, variation, stats } = useChartStats(bougies)
const prixEntreeSnapshot = ref<number>(0)

const {
  initChart, mettreAJourSerie, mettreAJourEnDirect, detruireChart,
  configurerRedimensionnement, arreterRedimensionnement, getChart, getCandlestickSeries,
} = useChartTradingView(chartContainer, bougies)

const { chargerEtAppliquer, reinitialiser } = useChartIndicators()
const v12Overlay = useSmcV12Overlay()
const dessins = useChartDessins()

/// Outils de dessin superposés au chart (cliquer-glisser ; Échap désactive).
const outilsDessin: { outil: 'ligne' | 'rectangle' | 'fibo' | 'gomme'; icone: string; titre: string }[] = [
  { outil: 'ligne', icone: '╱', titre: 'Ligne de tendance' },
  { outil: 'rectangle', icone: '▭', titre: 'Rectangle (zone)' },
  { outil: 'fibo', icone: 'ƒ', titre: 'Retracement Fibonacci' },
  { outil: 'gomme', icone: '⌫', titre: 'Gomme (clic sur un dessin)' },
]
let minuteurOverlay: ReturnType<typeof setInterval> | null = null
const { initialiser: ecoCalInit, chargerAnnonces, detruire: ecoCalDetruire,
  tooltipAnnonce, tooltipX, tooltipY } = useChartEcoCal()


const timestampCurseur = ref<number | null>(null)
const sidebarIA = ref(false)
const analyseSmcOuverte = ref(false)


async function lancerAnalyseSmc() {
  analyseSmcOuverte.value = true
  prixEntreeSnapshot.value = dernierPrix.value ?? 0
  const confianceMl = signalStore.prediction?.confiance ?? 0
  await signalStore.chargerAnalyseIA(selectedAsset.value, selectedTimeframe.value, prixEntreeSnapshot.value, confianceMl, 0)
}

function configurerCrosshair() {
  getChart()?.subscribeCrosshairMove((param) => {
    timestampCurseur.value = param.time ? (param.time as number) : null
  })
}


async function chargerIndicateurs() {
  await nextTick()
  const chart = getChart()
  if (!chart) return
  const serie = getCandlestickSeries()
  if (chartContainer.value && serie) v12Overlay.initialiser(chart, serie, chartContainer.value)
  if (chartContainer.value && serie) dessins.initialiser(chart, serie, chartContainer.value, selectedAsset.value)
  if (chartContainer.value) ecoCalInit(chart, chartContainer.value)
  // Overlay SMC v12 : fetch du replay moteur (indépendant des indicateurs classiques).
  const derniereB = bougies.value?.[bougies.value.length - 1]
  const tsSecV12 = derniereB ? Math.floor(new Date(derniereB.timestamp).getTime() / 1000) : undefined
  // Même fenêtre que le chart (5 000 = TV Basic) : parité de l'âge max des
  // zones avec TradingView — avant, l'analyse ne voyait que 500 bougies
  // (~5 jours M15) et perdait les OB plus anciens encore vivants.
  void v12Overlay.charger(selectedAsset.value, selectedTimeframe.value, limitPourTimeframe(selectedTimeframe.value), tsSecV12)
  void chargerTradesExternes()
  await chargerEtAppliquer(
    chart, selectedAsset.value, selectedTimeframe.value, settingsStore.indicateurs,
    rsiContainer.value, macdContainer.value, atrContainer.value,
    serie,
    (data) => {
      const derniereB2 = bougies.value?.[bougies.value.length - 1]
      const tsMs = derniereB2 ? new Date(derniereB2.timestamp).getTime() : null
      const tsSec = tsMs ? Math.floor(tsMs / 1000) : undefined
      v12Overlay.setDernierTs(tsSec)
    },
  )
}

const { assets, changerAsset, changerTimeframe, actualiser } = useChartOrchestration({
  selectedAsset, selectedTimeframe, bougies,
  indicateurs: ref(settingsStore.indicateurs),
  getChart, getCandlestickSeries,
  smcMettreAJourZones: (_data, _prefs, ts) => {
    v12Overlay.setDernierTs(ts)
    dessins.definirDernierBougie(ts ?? null)
  },
  chargerEtAppliquer,
  mettreAJourSerie, mettreAJourEnDirect,
  initChart, detruireChart, reinitialiser,
  configurerCrosshair, chargerIndicateurs,
  configurerRedimensionnement, arreterRedimensionnement,
})

/// Rafraîchit l'overlay v12 (trades vivants : fills, BE, TP, clôtures) sans
/// recharger les bougies — le moteur rejoue côté backend et resert l'état.
function rafraichirOverlayV12() {
  const derniereB = bougies.value?.[bougies.value.length - 1]
  const tsSec = derniereB ? Math.floor(new Date(derniereB.timestamp).getTime() / 1000) : undefined
  void v12Overlay.charger(selectedAsset.value, selectedTimeframe.value, limitPourTimeframe(selectedTimeframe.value), tsSec)
  void chargerTradesExternes()
}

/// Trades MULTI-TF : les signaux ouverts du même actif sur les AUTRES
/// timeframes, dessinés en atténué avec badge (un trade M1 reste visible
/// sur les graphiques M5/M15/M30/H1 tant qu'il est ouvert).
const DUREE_BARRE: Record<string, number> = {
  M1: 60, M5: 300, M15: 900, M30: 1800, H1: 3600, H4: 14400, D1: 86400, W1: 604800,
}
const SMC_NOMS = ['SMC', 'SmcDirectional', 'SMC Directionnel', 'SMC+IA']

async function chargerTradesExternes() {
  try {
    const signaux = await apiService.getSignaux(150)
    type RowSign = { asset: string; timeframe: string; direction: string; statut: string; strategie: string; prix_entree: number; stop_loss: number; take_profit: number[]; score: number; heure_entree: number | null; cree_le: number }
    const ouverts = (signaux as RowSign[])
      .filter(x =>
        x.asset === selectedAsset.value
        && x.statut === 'Actif'
        && SMC_NOMS.includes(x.strategie)
        // Autres TF : tout trade ouvert. TF affiché : uniquement les
        // ordres EN ATTENTE (jamais remplis — le moteur ne les dessine
        // pas avant le fill, fidélité Pine ; les remplis du TF courant
        // gardent leur affichage riche).
        && (x.timeframe !== selectedTimeframe.value || x.heure_entree === null))
      .slice(0, 6)
      .map(x => ({
        ts: x.cree_le,
        entry: x.prix_entree,
        sl: x.stop_loss,
        tp1: x.take_profit?.[0] ?? x.prix_entree,
        tp2: x.take_profit?.[1] ?? x.prix_entree,
        tp3: x.take_profit?.[2] ?? x.prix_entree,
        dir: x.direction === 'Long' ? 'Long' as const : 'Short' as const,
        force: Math.max(1, Math.min(10, Math.round(x.score))),
        be: false,
        label: [] as string[],
        tfOrigine: x.timeframe,
        enAttente: x.heure_entree === null,
        tsFin: x.cree_le + 40 * (DUREE_BARRE[x.timeframe] ?? 900),
      }))
    v12Overlay.definirTradesExternes(ouverts)
  } catch { /* silencieux */ }
}

// Affichage VIVANT des trades (fidélité Pine : le label et les boxes évoluent
// barre après barre) — rafraîchissement léger toutes les 30 s.
onMounted(() => {
  minuteurOverlay = setInterval(rafraichirOverlayV12, 30_000)
})

// Changement d'asset : les dessins sont persistés PAR asset.
watch(selectedAsset, (a) => dessins.definirAsset(a))

// Nettoyage overlay v12 au démontage de la vue
onUnmounted(() => {
  if (minuteurOverlay !== null) clearInterval(minuteurOverlay)
  v12Overlay.detruire()
  dessins.detruire()
})

chargerAnnonces()
</script>

<style scoped>
.glass-card {
  @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm overflow-hidden;
}
</style>
