<template>
  <div class="relative h-full min-h-0">

    <div class="flex flex-col gap-4 w-full h-full min-h-0">
      <!-- Barre haute : layout + métriques/sélecteurs de la cellule ACTIVE -->
      <div class="flex items-start gap-3 flex-wrap">
        <!-- Dropdown layout multi-graphiques -->
        <div class="relative">
          <button
            title="Disposition des graphiques"
            class="h-10 px-2.5 rounded-lg bg-white/5 border border-white/10 hover:bg-white/10 transition-colors flex items-center justify-center"
            @click="layoutOuvert = !layoutOuvert"
          >
            <span class="mini-grille" :data-layout="layout">
              <span v-for="i in nbCellulesLayout[layout]" :key="i" />
            </span>
          </button>
          <div v-if="layoutOuvert" class="fixed inset-0 z-40" @click="layoutOuvert = false" />
          <div v-if="layoutOuvert" class="absolute left-0 top-[calc(100%+6px)] z-50 w-44 bg-slate-900/95 backdrop-blur border border-white/10 rounded-lg shadow-xl py-1.5">
            <div class="px-2.5 pb-1.5 text-[10px] uppercase tracking-wide text-slate-500 border-b border-white/5">Graphiques</div>
            <button v-for="l in LAYOUTS" :key="l.id"
              class="w-full flex items-center gap-3 px-2.5 py-1.5 text-xs text-slate-300 hover:bg-white/5"
              :class="layout === l.id && 'text-cyan-300'"
              @click="choisirLayout(l.id)"
            >
              <span class="mini-grille" :data-layout="l.id"><span v-for="i in l.nb" :key="i" /></span>
              <span>{{ l.nb }} graphique{{ l.nb > 1 ? 's' : '' }}</span>
              <span v-if="layout === l.id" class="ml-auto text-cyan-400">✓</span>
            </button>
          </div>
        </div>

        <!-- Métriques + dropdowns asset/TF (cellule active) -->
        <div class="flex-1 min-w-[280px]">
          <ChartPrixStats :dernier-prix="dernierPrixActif" :variation="variationActive" :stats="statsActives"
            :selected-asset="slotActif.asset" :selected-timeframe="slotActif.timeframe"
            :ws-connecte="marketStore.wsConnecte" :assets="assets" :timeframes="timeframes"
            @changer-asset="changerAssetSlotActif" @changer-timeframe="changerTfSlotActif" />
        </div>
      </div>

      <!-- Grille des cellules (parts égales) -->
      <div class="flex-1 min-h-0 grid gap-3" :style="styleGrille">
        <CelluleChart v-for="(slot, i) in slots" :key="`cellule-${i}`"
          :asset="slot.asset" :timeframe="slot.timeframe"
          :active="i === celluleActive" :avec-sous-graphes="nbCellules <= 4" :cle-prefs="clePrefs"
          @activer="celluleActive = i" />
      </div>

      <!-- Panneau indicateurs (techniques + SMC) — global, appliqué partout -->
      <IndicatorPanel v-model="settingsStore.indicateurs" @appliquer="clePrefs++">
        <template #apres-smc>
          <button
            class="px-2.5 py-1 rounded-md border transition-colors bg-purple-600/20 border-purple-500/30 text-purple-300 hover:bg-purple-600/30 disabled:opacity-40 text-xs font-medium"
            :disabled="signalStore.analyseIaChargement" @click="lancerAnalyseSmc">{{ signalStore.analyseIaChargement ? '🔍 Analyse...' : '🔍 Analyse SMC' }}</button>
        </template>
      </IndicatorPanel>

    </div>

    <!-- Sidebar IA (toggle + drawer) — cellule active -->
    <ChartSidebarIA :asset="slotActif.asset" :timeframe="slotActif.timeframe" :open="sidebarIA"
      @toggle="sidebarIA = !sidebarIA" />

    <!-- Modales (hors flux) -->
    <ChartAnalyseSmcModal
      :open="analyseSmcOuverte"
      :asset="slotActif.asset"
      :timeframe="slotActif.timeframe"
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
import { ref, computed, watch } from 'vue'
import { useMarketStore } from '@/stores/market.store'
import { useSettingsStore } from '@/stores/settings.store'
import { useAssetsStore } from '@/stores/assets.store'
import { useChartStats } from '@/composables/useChartStats'
import { useSignalStore } from '@/stores/signal.store'
import CelluleChart from '@/components/chart/CelluleChart.vue'
import ChartSidebarIA from '@/components/common/ChartSidebarIA.vue'
import ChartAnalyseSmcModal from '@/components/common/ChartAnalyseSmcModal.vue'
import IndicatorPanel from '@/components/common/IndicatorPanel.vue'
import ChartPrixStats from '@/components/common/ChartPrixStats.vue'

const marketStore = useMarketStore()
const settingsStore = useSettingsStore()
const signalStore = useSignalStore()
const assetsStore = useAssetsStore()

const timeframes = ['M1', 'M5', 'M10', 'M15', 'M30', 'H1', 'H4', 'D1', 'W1']

// ── Layout multi-graphiques (2/4/6/8, parts égales) ───────────────────────────
type LayoutId = '1' | '2' | '4' | '6' | '8'
const LAYOUTS: { id: LayoutId; nb: number; colonnes: number; lignes: number }[] = [
  { id: '1', nb: 1, colonnes: 1, lignes: 1 },
  { id: '2', nb: 2, colonnes: 2, lignes: 1 },
  { id: '4', nb: 4, colonnes: 2, lignes: 2 },
  { id: '6', nb: 6, colonnes: 3, lignes: 2 },
  { id: '8', nb: 8, colonnes: 4, lignes: 2 },
]
const nbCellulesLayout: Record<LayoutId, number> = { '1': 1, '2': 2, '4': 4, '6': 6, '8': 8 }
const CLE_LAYOUT = 'trading_layout'
const CLE_SLOTS = 'trading_slots_graphiques'

const layout = ref<LayoutId>((() => {
  const v = localStorage.getItem(CLE_LAYOUT) as LayoutId | null
  return v && nbCellulesLayout[v] ? v : '1'
})())
const layoutOuvert = ref(false)

interface Slot { asset: string; timeframe: string }
/// Slots par emplacement — la cellule active reprend la sélection courante ;
/// les autres cycle sur les assets SMC disponibles.
const rotationDefaut = ['BTC', 'XAUUSD', 'DAX', 'XAGUSD', 'NAS100', 'SP500', 'ETH', 'BTC']
const slots = ref<Slot[]>(chargerSlots())
const celluleActive = ref(0)
const clePrefs = ref(0)

function chargerSlots(): Slot[] {
  const nb = nbCellulesLayout[layout.value]
  try {
    const brut = JSON.parse(localStorage.getItem(CLE_SLOTS) ?? 'null') as Slot[] | null
    if (Array.isArray(brut)) {
      const base: Slot[] = brut.slice(0, nb).map(s => ({ asset: s.asset, timeframe: s.timeframe }))
      while (base.length < nb) {
        const i = base.length
        base.push({ asset: rotationDefaut[i % rotationDefaut.length], timeframe: 'M15' })
      }
      return base
    }
  } catch { /* stockage illisible */ }
  return Array.from({ length: nb }, (_, i) => ({
    asset: i === 0 ? (settingsStore.assetActif || 'BTC') : rotationDefaut[i % rotationDefaut.length],
    timeframe: i === 0 ? (settingsStore.timeframeActif || 'M15') : 'M15',
  }))
}

function persisterSlots() {
  localStorage.setItem(CLE_SLOTS, JSON.stringify(slots.value))
}

function choisirLayout(l: LayoutId) {
  layout.value = l
  localStorage.setItem(CLE_LAYOUT, l)
  const nb = nbCellulesLayout[l]
  slots.value = chargerSlots()
  if (slots.value.length > nb) slots.value = slots.value.slice(0, nb)
  celluleActive.value = Math.min(celluleActive.value, nb - 1)
  persisterSlots()
  layoutOuvert.value = false
}

const nbCellules = computed(() => nbCellulesLayout[layout.value])
const layoutCourant = computed(() => LAYOUTS.find(l => l.id === layout.value) ?? LAYOUTS[0])
const styleGrille = computed(() => ({
  gridTemplateColumns: `repeat(${layoutCourant.value.colonnes}, minmax(0, 1fr))`,
  gridTemplateRows: `repeat(${layoutCourant.value.lignes}, minmax(0, 1fr))`,
}))

// Le slot actif suit la cellule active.
const slotActif = computed(() => slots.value[celluleActive.value] ?? slots.value[0])

function changerAssetSlotActif(asset: string) {
  slotActif.value.asset = asset
  persisterSlots()
}
function changerTfSlotActif(tf: string) {
  slotActif.value.timeframe = tf
  persisterSlots()
}

// Assets proposés (comme avant : assets configurés, SMC only).
const CRYPTO_SMC = ['BTC', 'ETH']
const assets = computed(() => {
  const liste = assetsStore.assets
  if (liste.length === 0) return ['BTC', 'ETH', 'XAUUSD', 'XAGUSD']
  return liste
    .filter(a => a.type !== 'crypto' || CRYPTO_SMC.includes(a.id))
    .map(a => a.id)
})

// Métriques de la barre haute = cellule ACTIVE.
const bougiesActives = computed(() =>
  marketStore.getBougies(slotActif.value.asset, slotActif.value.timeframe)
)
const { dernierPrix: dernierPrixActif, variation: variationActive, stats: statsActives } = useChartStats(bougiesActives)

// Sidebar IA + Analyse SMC (cellule active).
const sidebarIA = ref(false)
const analyseSmcOuverte = ref(false)
const prixEntreeSnapshot = ref<number>(0)
async function lancerAnalyseSmc() {
  analyseSmcOuverte.value = true
  prixEntreeSnapshot.value = dernierPrixActif.value ?? 0
  const confianceMl = signalStore.prediction?.confiance ?? 0
  await signalStore.chargerAnalyseIA(slotActif.value.asset, slotActif.value.timeframe, prixEntreeSnapshot.value, confianceMl, 0)
}
</script>

<style scoped>
/* Mini-grille du dropdown layout — cases égales, reflet de la disposition. */
.mini-grille {
  display: grid;
  gap: 2px;
  width: 22px;
}
.mini-grille span {
  height: 7px;
  border-radius: 2px;
  background: #64748b;
}
.mini-grille[data-layout="1"] { grid-template-columns: repeat(1, 1fr); }
.mini-grille[data-layout="2"] { grid-template-columns: repeat(2, 1fr); }
.mini-grille[data-layout="4"] { grid-template-columns: repeat(2, 1fr); }
.mini-grille[data-layout="6"] { grid-template-columns: repeat(3, 1fr); }
.mini-grille[data-layout="8"] { grid-template-columns: repeat(4, 1fr); }
button:hover .mini-grille span { background: #94a3b8; }
</style>
