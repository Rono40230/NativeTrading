<template>
  <div class="relative h-full min-h-0 bg-emerald-500/5 rounded-xl">

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
            <div class="px-2.5 pb-1.5 text-[10px] uppercase tracking-wide text-white border-b border-white/5">Graphiques</div>
            <button v-for="l in LAYOUTS" :key="l.id"
              class="w-full flex items-center gap-3 px-2.5 py-1.5 text-xs text-white hover:bg-white/5"
              :class="layout === l.id && 'text-cyan-300'"
              @click="choisirLayout(l.id)"
            >
              <span class="mini-grille" :data-layout="l.id"><span v-for="i in l.nb" :key="i" /></span>
              <span>{{ l.nb }} graphique{{ l.nb > 1 ? 's' : '' }}</span>
              <span v-if="layout === l.id" class="ml-auto text-cyan-400">✓</span>
            </button>
          </div>
        </div>

        <!-- Replay : fige la cellule ACTIVE à un instant passé (owner 28/10) -->
        <div class="relative">
          <button
            title="Replay — figer le graphique actif à une date passée"
            :class="['h-10 px-2.5 rounded-lg border transition-colors flex items-center justify-center text-sm',
              replayInstantSec !== undefined ? 'bg-amber-500/20 border-amber-400/40 text-amber-300' : 'bg-white/5 border-white/10 text-white hover:bg-white/10']"
            @click="basculerReplay">⏱</button>
          <div v-if="replayOuvert" class="fixed inset-0 z-40" @click="replayOuvert = false" />
          <div v-if="replayOuvert" class="absolute left-0 top-[calc(100%+6px)] z-50 w-64 bg-slate-900/95 backdrop-blur border border-white/10 rounded-lg shadow-xl p-2.5 flex flex-col gap-2">
            <div class="text-[10px] uppercase tracking-wide text-white border-b border-white/5 pb-1.5">
              Replay — {{ slotActif.asset }} {{ slotActif.timeframe }}
            </div>
            <input v-model="replayDate" type="datetime-local"
              class="w-full px-2 py-1.5 rounded-md bg-white/5 border border-white/10 text-xs text-white" />
            <div class="flex gap-2">
              <button class="flex-1 px-2 py-1.5 rounded-md bg-amber-500/20 border border-amber-400/30 text-amber-300 text-xs hover:bg-amber-500/30"
                @click="appliquerReplay">Figer</button>
              <button v-if="replayInstantSec !== undefined"
                class="flex-1 px-2 py-1.5 rounded-md bg-white/5 border border-white/10 text-white/70 text-xs hover:bg-white/10"
                @click="quitterReplay">Retour direct</button>
            </div>
            <div class="text-[10px] text-white/50">Le moteur rejoue les bougies clôturées avant l'instant — le flux live est suspendu sur cette cellule.</div>
            <div class="border-t border-white/5 pt-2 flex flex-col gap-2">
              <div class="text-[10px] uppercase tracking-wide text-white">Rejouer une période</div>
              <div class="flex items-center gap-1.5">
                <input v-model="periodeDateDebut" type="datetime-local"
                  class="flex-1 min-w-0 px-2 py-1.5 rounded-md bg-white/5 border border-white/10 text-xs text-white" />
                <span class="text-white/40 text-xs">→</span>
                <input v-model="periodeDateFin" type="datetime-local"
                  class="flex-1 min-w-0 px-2 py-1.5 rounded-md bg-white/5 border border-white/10 text-xs text-white" />
              </div>
              <button class="px-2 py-1.5 rounded-md bg-cyan-500/20 border border-cyan-400/30 text-cyan-300 text-xs hover:bg-cyan-500/30"
                @click="lancerPeriode">▶ Rejouer la période (défilement des barres)</button>
            </div>
          </div>
        </div>

        <!-- Lecteur de période (visible pendant le replay animé) -->
        <div v-if="periodeActive" class="flex items-center gap-1 px-2 h-10 rounded-lg bg-amber-500/10 border border-amber-400/30 shrink-0">
          <button :title="periodeLecture ? 'Pause' : 'Lecture'"
            class="w-7 h-7 rounded-md text-amber-300 hover:bg-amber-500/20 text-sm"
            @click="periodeLecture ? pauserLecture() : demarrerLecture()">{{ periodeLecture ? '⏸' : '▶' }}</button>
          <button title="Barre précédente" class="w-7 h-7 rounded-md text-amber-300 hover:bg-amber-500/20 text-sm" @click="pasPeriode(-1)">◀</button>
          <button title="Barre suivante" class="w-7 h-7 rounded-md text-amber-300 hover:bg-amber-500/20 text-sm" @click="pasPeriode(1)">▶</button>
          <select v-model.number="periodeVitesse" title="Vitesse (barres/seconde)"
            class="px-1 py-1 rounded-md bg-white/5 border border-white/10 text-[11px] text-amber-200">
            <option :value="1">×1</option><option :value="2">×2</option><option :value="5">×5</option><option :value="10">×10</option>
          </select>
          <span class="text-[10px] text-amber-300 font-mono tabular-nums whitespace-nowrap">{{ horodatagePeriode }}</span>
          <button title="Quitter le replay de période" class="w-7 h-7 rounded-md text-white/60 hover:text-red-300 hover:bg-white/10 text-sm" @click="quitterPeriode">✕</button>
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
          :replay-instant="i === celluleActive ? replayInstantSec : undefined"
          :replay-debut="i === celluleActive ? (periodeDebut ?? undefined) : undefined"
          :replay-fin="i === celluleActive ? (periodeFin ?? undefined) : undefined"
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
import { ref, computed, watch , onBeforeUnmount } from 'vue'
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
/// les autres cyclent sur la rotation par défaut.
const rotationDefaut = ['BTC', 'XAUUSD', 'DAX', 'XAGUSD', 'NAS100', 'SP500', 'ETH', 'BTC']
const slots = ref<Slot[]>(chargerSlots())
const celluleActive = ref(0)
const clePrefs = ref(0)

// Fenêtre Graphiques indépendante (écart 18 du 11/10) : un « Voir » cliqué
// dans la fenêtre principale écrit le slot 1 — l'événement storage (même
// origine) re-cible la cellule 1 SANS recharger la page.
function surStorage(e: StorageEvent) {
  if (e.key !== CLE_SLOTS) return
  const maj = chargerSlots()
  if (maj[0]) slots.value[0] = { ...slots.value[0], ...maj[0] }
}
window.addEventListener('storage', surStorage)

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

// Assets proposés dans le sélecteur : les ACTIFS du pipeline, toutes
// classes (owner 24/10 — le dropdown doit suivre le pilotage ; l'ancien
// filtre BTC/ETH est obsolète depuis la bascule des 11 cryptos sur MT5).
const assets = computed(() => {
  const liste = assetsStore.assets
  if (liste.length === 0) return ['BTC', 'ETH', 'XAUUSD', 'XAGUSD']
  return liste.map(a => a.id)
})

// ── Replay (owner 28/10) : figer la cellule ACTIVE à un instant passé ────────
const replayOuvert = ref(false)
const replayDate = ref('')
const replayInstantBrut = ref<number | null>(null)

// ── Replay par PÉRIODE (owner 28/10 v2) : curseur animé barre par barre ─────
const DUREES_TF: Record<string, number> = { M1: 60, M5: 300, M10: 600, M15: 900, M30: 1800, H1: 3600, H4: 14400, D1: 86400, W1: 604800 }
const periodeDateDebut = ref('')
const periodeDateFin = ref('')
const periodeDebut = ref<number | null>(null)
const periodeFin = ref<number | null>(null)
const periodeCurseur = ref<number | null>(null)
const periodeLecture = ref(false)
const periodeVitesse = ref(2) // barres/seconde
let minuteurPeriode: ReturnType<typeof setInterval> | null = null

const periodeActive = computed(() => periodeCurseur.value !== null)
const horodatagePeriode = computed(() =>
  periodeCurseur.value ? new Date(periodeCurseur.value * 1000)
    .toLocaleString('fr-FR', { day: '2-digit', month: '2-digit', hour: '2-digit', minute: '2-digit' }) : '')

function arreterMinuteur() {
  if (minuteurPeriode !== null) { clearInterval(minuteurPeriode); minuteurPeriode = null }
}
function demarrerLecture() {
  if (periodeCurseur.value === null) return
  if (periodeCurseur.value >= (periodeFin.value ?? 0)) periodeCurseur.value = periodeDebut.value ?? periodeCurseur.value
  arreterMinuteur()
  periodeLecture.value = true
  const tf = DUREES_TF[slotActif.value.timeframe] ?? 900
  minuteurPeriode = setInterval(() => {
    if (periodeCurseur.value === null) return
    if (periodeCurseur.value >= (periodeFin.value ?? 0)) { pauserLecture(); return }
    periodeCurseur.value += tf
  }, Math.max(80, Math.round(1000 / periodeVitesse.value)))
}
function pauserLecture() { periodeLecture.value = false; arreterMinuteur() }
function pasPeriode(sens: 1 | -1) {
  if (periodeCurseur.value === null) return
  pauserLecture()
  const tf = DUREES_TF[slotActif.value.timeframe] ?? 900
  periodeCurseur.value = Math.max(0, Math.min((periodeFin.value ?? 0) + tf, periodeCurseur.value + sens * tf))
}
function lancerPeriode() {
  const d = new Date(periodeDateDebut.value).getTime()
  const f = new Date(periodeDateFin.value).getTime()
  if (Number.isNaN(d) || Number.isNaN(f) || f <= d) return
  periodeDebut.value = Math.floor(d / 1000)
  periodeFin.value = Math.floor(f / 1000)
  periodeCurseur.value = periodeDebut.value
  replayInstantBrut.value = null // le mode instant cède la place
  replayOuvert.value = false
  demarrerLecture()
}
function quitterPeriode() {
  pauserLecture()
  periodeCurseur.value = null
  periodeDebut.value = null
  periodeFin.value = null
}
watch(periodeVitesse, () => { if (periodeLecture.value) demarrerLecture() })

const replayInstantSec = computed(() => {
  if (periodeCurseur.value && periodeCurseur.value > 0) return periodeCurseur.value
  return replayInstantBrut.value && replayInstantBrut.value > 0 ? replayInstantBrut.value : undefined
})

function basculerReplay() {
  replayOuvert.value = !replayOuvert.value
  if (replayOuvert.value) {
    if (!replayDate.value) {
      const d = new Date(Date.now() - 3_600_000)
      replayDate.value = new Date(d.getTime() - d.getTimezoneOffset() * 60_000).toISOString().slice(0, 16)
    }
    if (!periodeDateDebut.value || !periodeDateFin.value) {
      const debut = new Date(Date.now() - 5 * 86_400_000)
      const fin = new Date(Date.now() - 86_400_000)
      periodeDateDebut.value = new Date(debut.getTime() - debut.getTimezoneOffset() * 60_000).toISOString().slice(0, 16)
      periodeDateFin.value = new Date(fin.getTime() - fin.getTimezoneOffset() * 60_000).toISOString().slice(0, 16)
    }
  }
}
function appliquerReplay() {
  const t = new Date(replayDate.value).getTime()
  quitterPeriode() // un seul mode à la fois
  if (!Number.isNaN(t) && t > 0) replayInstantBrut.value = Math.floor(t / 1000)
  replayOuvert.value = false
}
function quitterReplay() {
  replayInstantBrut.value = null
  replayOuvert.value = false
}

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
onBeforeUnmount(() => { window.removeEventListener('storage', surStorage); arreterMinuteur() })
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
