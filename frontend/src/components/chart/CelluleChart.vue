<template>
  <!-- Cellule de graphique multi-layout — une instance par emplacement.
       Toute la mécanique chart (candles, overlays SMC, dessins, alertes,
       éco-cal, sous-graphiques, orchestration) vit ici, indépendamment. -->
  <div :class="[pleinEcran ? 'cellule-plein-ecran' : [active ? 'cellule-active' : 'cellule-inactive', 'h-full min-h-0'].join(' ')]"
    class="flex flex-col gap-2 p-3" @mousedown="$emit('activer')">

    <!-- Mini-bandeau de la cellule : asset, TF, prix, variation, flux -->
    <div class="flex items-center gap-2 px-2 py-1 rounded-lg bg-white/5 border border-white/10 text-xs shrink-0 cursor-pointer select-none"
      title="Double-clic : plein écran"
      @dblclick="basculerPleinEcran">
      <span class="font-bold text-white">{{ asset }}</span>
      <span class="px-1.5 rounded bg-cyan-500/20 text-cyan-300 text-[10px] font-semibold">{{ timeframe }}</span>
      <span class="font-mono tabular-nums ml-1" :class="variation >= 0 ? 'text-emerald-400' : 'text-red-400'">
        {{ dernierPrix !== null ? dernierPrix.toLocaleString('fr-FR', { maximumFractionDigits: 2 }) : '—' }}
      </span>
      <span class="font-mono tabular-nums text-[10px]" :class="variation >= 0 ? 'text-emerald-500' : 'text-red-500'">
        {{ variation >= 0 ? '+' : '' }}{{ variation.toFixed(2) }}%
      </span>
      <span class="ml-auto flex items-center gap-1 text-[10px]" :class="marketStore.wsConnecte ? 'text-emerald-400' : 'text-slate-500'">
        <span class="w-1.5 h-1.5 rounded-full" :class="marketStore.wsConnecte ? 'bg-emerald-400' : 'bg-slate-500'" />
        {{ marketStore.wsConnecte ? 'flux' : 'silence' }}
      </span>
      <button v-if="pleinEcran" title="Quitter le plein écran (Échap)"
        class="w-6 h-6 rounded-md flex items-center justify-center text-slate-400 hover:text-white hover:bg-white/10 transition-colors"
        @click.stop="pleinEcran = false">✕</button>
    </div>

    <!-- Canvas TradingView -->
    <div class="glass-card flex-1 min-h-0" style="min-height: 120px; position: relative;">
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
      <!-- Barre d'outils de dessin + alertes (superposée au chart) -->
      <div style="right: 92px; bottom: 36px;" class="absolute z-20 flex flex-col gap-1 p-1 rounded-lg bg-slate-900/80 backdrop-blur border border-white/10 shadow-lg">
        <button v-for="t in outilsDessin" :key="t.outil"
          :title="t.titre"
          :class="[
            'w-8 h-8 rounded-md text-sm flex items-center justify-center transition-colors',
            dessins.outil.value === t.outil
              ? 'bg-blue-600/40 text-blue-200 border border-blue-400/50'
              : 'text-slate-400 hover:text-slate-100 hover:bg-white/10 border border-transparent',
          ]"
          @click="dessins.choisirOutil(t.outil)"
        >{{ t.icone }}</button>
        <!-- Alertes de prix : pose au clic + liste -->
        <div class="relative">
          <button
            :title="alertesPrix.modePose.value ? 'Cliquer sur le chart pour poser une alerte' : 'Alerte de prix (pose au clic)'"
            :class="[
              'w-8 h-8 rounded-md text-sm flex items-center justify-center transition-colors',
              alertesPrix.modePose.value || listeAlertesOuverte
                ? 'bg-amber-500/40 text-amber-200 border border-amber-400/50'
                : 'text-slate-400 hover:text-slate-100 hover:bg-white/10 border border-transparent',
            ]"
            @click="alertesPrix.nbActives.value > 0 && !alertesPrix.modePose.value ? (listeAlertesOuverte = !listeAlertesOuverte) : alertesPrix.basculerModePose()"
          >🔔<span v-if="alertesPrix.nbActives.value > 0" class="absolute -top-1 -right-1 min-w-[14px] px-0.5 rounded-full bg-amber-500 text-[9px] leading-[14px] text-amber-950 font-bold text-center">{{ alertesPrix.nbActives.value }}</span></button>
          <div v-if="listeAlertesOuverte" class="fixed inset-0 z-40" @click="listeAlertesOuverte = false" />
          <div v-if="listeAlertesOuverte" class="absolute bottom-[calc(100%+6px)] right-0 z-50 w-64 bg-slate-900/95 backdrop-blur border border-white/10 rounded-lg shadow-xl py-1.5">
            <div class="flex items-center justify-between px-2.5 pb-1.5 border-b border-white/5">
              <span class="text-[10px] uppercase tracking-wide text-slate-500">Alertes — {{ asset }}</span>
              <button class="text-[10px] text-amber-400 hover:text-amber-300" @click="listeAlertesOuverte = false; alertesPrix.basculerModePose()">+ Poser au clic</button>
            </div>
            <div v-if="!alertesPrix.alertesAsset.value.length" class="px-2.5 py-3 text-[11px] text-slate-500 text-center">Aucune alerte sur cet asset</div>
            <div v-for="a in alertesPrix.alertesAsset.value" :key="a.id"
                 class="flex items-center gap-2 px-2.5 py-1.5 text-[11px] hover:bg-white/5">
              <span class="font-mono tabular-nums" :class="a.active ? 'text-amber-300' : 'text-slate-500 line-through'">{{ a.prix.toFixed(2) }}</span>
              <span class="text-slate-500">{{ a.sens === 'au_dessus' ? '↑' : '↓' }}</span>
              <span class="flex-1 truncate text-slate-400">{{ a.note ?? '' }}</span>
              <button v-if="!a.active" title="Réarmer" class="text-slate-400 hover:text-amber-300" @click="alertesPrix.rearmer(a.id)">🔄</button>
              <button title="Supprimer" class="text-slate-500 hover:text-red-300" @click="alertesPrix.supprimer(a.id)">🗑</button>
            </div>
          </div>
        </div>
        <div class="h-px bg-white/10 mx-1" />
        <button title="Effacer tous les dessins de cet asset"
          class="w-8 h-8 rounded-md text-sm flex items-center justify-center text-slate-400 hover:text-red-300 hover:bg-red-500/10 transition-colors"
          @click="dessins.toutEffacer()"
        >🗑</button>
      </div>
      <EcoCalTooltip :annonce="tooltipAnnonce" :x="tooltipX" :y="tooltipY" />
      <TendanceMultiTF v-if="settingsStore.indicateurs.kasperTendance" :key="asset + '_' + timeframe"
        :asset="asset" :timeframe="timeframe"
        :periode-rapide="settingsStore.indicateurs.kasperPeriodeRapide"
        :periode-lente="settingsStore.indicateurs.kasperPeriodeLente"
        :mode-calcul="settingsStore.indicateurs.kasperModeCalcul" />
    </div>

    <!-- Sous-graphiques (option 2 : coupés au-delà de 4 cellules) -->
    <div v-if="avecSousGraphes && settingsStore.indicateurs.rsi" ref="rsiContainer" class="glass-card shrink-0"
      style="height: 140px; position: relative;" />
    <div v-if="avecSousGraphes && settingsStore.indicateurs.macd" ref="macdContainer" class="glass-card shrink-0"
      style="height: 140px; position: relative;" />
    <div v-if="avecSousGraphes && settingsStore.indicateurs.atr" ref="atrContainer" class="glass-card shrink-0"
      style="height: 110px; position: relative;" />
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
import { useAlertesPrix } from '@/composables/useAlertesPrix'
import { useChartEcoCal } from '@/composables/useChartEcoCal'
import { useChartOrchestration } from '@/composables/useChartOrchestration'
import EcoCalTooltip from '@/components/common/EcoCalTooltip.vue'
import TendanceMultiTF from '@/components/common/TendanceMultiTF.vue'
import { apiService } from '@/services/api.service'

const props = defineProps<{
  asset: string
  timeframe: string
  active: boolean
  /** Option 2 : sous-graphiques RSI/MACD/ATR réservés aux layouts ≤ 4 cellules. */
  avecSousGraphes: boolean
  /** Incrémenté par le parent à chaque « Appliquer » du panneau d'indicateurs. */
  clePrefs: number
}>()
defineEmits<{ (e: 'activer'): void }>()

const marketStore = useMarketStore()
const settingsStore = useSettingsStore()

const selectedAsset = ref(props.asset)
const selectedTimeframe = ref(props.timeframe)
const chartContainer = ref<HTMLElement | null>(null)
const rsiContainer = ref<HTMLElement | null>(null)
const macdContainer = ref<HTMLElement | null>(null)
const atrContainer = ref<HTMLElement | null>(null)

const bougies = computed(() =>
  marketStore.getBougies(selectedAsset.value, selectedTimeframe.value)
)
const { dernierPrix, variation } = useChartStats(bougies)

const {
  initChart, mettreAJourSerie, mettreAJourEnDirect, detruireChart,
  configurerRedimensionnement, arreterRedimensionnement, getChart, getCandlestickSeries,
} = useChartTradingView(chartContainer, bougies)

const { chargerEtAppliquer, reinitialiser } = useChartIndicators()
const v12Overlay = useSmcV12Overlay()
const dessins = useChartDessins()
const alertesPrix = useAlertesPrix()
const listeAlertesOuverte = ref(false)

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

/// Double-clic sur le bandeau : la cellule couvre toute la fenêtre (les
/// autres graphiques et la barre haute passent dessous). Échap ou ✕ revient.
const pleinEcran = ref(false)
function basculerPleinEcran() { pleinEcran.value = !pleinEcran.value }
function surEscape(e: KeyboardEvent) {
  if (e.key === 'Escape' && pleinEcran.value) pleinEcran.value = false
}
window.addEventListener('keydown', surEscape)

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
  if (serie) alertesPrix.initialiser(chart, serie, selectedAsset.value)
  if (chartContainer.value) ecoCalInit(chart, chartContainer.value)
  const derniereB = bougies.value?.[bougies.value.length - 1]
  const tsSecV12 = derniereB ? Math.floor(new Date(derniereB.timestamp).getTime() / 1000) : undefined
  void v12Overlay.charger(selectedAsset.value, selectedTimeframe.value, limitPourTimeframe(selectedTimeframe.value), tsSecV12)
  void chargerTradesExternes()
  await chargerEtAppliquer(
    chart, selectedAsset.value, selectedTimeframe.value, settingsStore.indicateurs,
    avecSousGraphes.value ? rsiContainer.value : null,
    avecSousGraphes.value ? macdContainer.value : null,
    avecSousGraphes.value ? atrContainer.value : null,
    serie,
    (data) => {
      const derniereB2 = bougies.value?.[bougies.value.length - 1]
      const tsMs = derniereB2 ? new Date(derniereB2.timestamp).getTime() : null
      const tsSec = tsMs ? Math.floor(tsMs / 1000) : undefined
      v12Overlay.setDernierTs(tsSec)
    },
  )
}
const avecSousGraphes = computed(() => props.avecSousGraphes)

const { changerAsset, changerTimeframe } = useChartOrchestration({
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

/// Rafraîchit l'overlay v12 (trades vivants) sans recharger les bougies.
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
  M1: 60, M5: 300, M10: 600, M15: 900, M30: 1800, H1: 3600, H4: 14400, D1: 86400, W1: 604800,
}
const NOMS_DESSINES = ['SMC', 'SmcDirectional', 'SMC Directionnel', 'SMC+IA', 'straddle', 'Straddle']

async function chargerTradesExternes() {
  try {
    const signaux = await apiService.getSignaux(150)
    type RowSign = { asset: string; timeframe: string; direction: string; statut: string; strategie: string; prix_entree: number; stop_loss: number; take_profit: number[]; score: number; heure_entree: number | null; cree_le: number; cle_moteur?: string }
    const candidats = (signaux as RowSign[])
      .filter(x =>
        x.asset === selectedAsset.value
        && x.statut === 'Actif'
        && NOMS_DESSINES.includes(x.strategie)
        // Autres TF : tout trade ouvert. TF affiché : uniquement les
        // ordres EN ATTENTE (jamais remplis — fidélité Pine).
        && (x.timeframe !== selectedTimeframe.value || x.heure_entree === null))
      .slice(0, 6)

    // Récupérer les niveaux du REPLAY du TF d'origine (pas de la base —
    // le SL/TPs diffèrent entre création temps réel et replay, et le
    // graphique du TF d'origine dessine depuis le replay).
    const tfsOrigine = [...new Set(candidats.filter(x => x.heure_entree !== null).map(x => x.timeframe))]
    const replays: Record<string, { signals: Array<{ ts: number; entry: number; sl: number; tp1: number; tp2: number; tp3: number; be?: boolean; ferme?: boolean }> | undefined }> = {}
    for (const tf of tfsOrigine) {
      if (tf === selectedTimeframe.value) continue
      try {
        const replay = await apiService.getSmcV12Analyse(selectedAsset.value, tf, 200)
        replays[tf] = replay as any
      } catch { /* replay indisponible — repli sur la base */ }
    }

    const ouverts = candidats.map(x => {
      // Chercher le signal correspondant dans le replay du TF d'origine.
      const replay = replays[x.timeframe]
      const match = replay?.signals?.find(s => !s.ferme && Math.abs(s.entry - x.prix_entree) < 0.01)
      const niveaux = match ?? { entry: x.prix_entree, sl: x.stop_loss, tp1: x.take_profit?.[0] ?? x.prix_entree, tp2: x.take_profit?.[1] ?? x.prix_entree, tp3: x.take_profit?.[2] ?? x.prix_entree, be: false }
      return {
        ts: x.cree_le,
        entry: niveaux.entry,
        sl: niveaux.sl,
        tp1: niveaux.tp1,
        tp2: niveaux.tp2,
        tp3: niveaux.tp3,
        dir: x.direction === 'Long' ? 'Long' as const : 'Short' as const,
        force: Math.max(1, Math.min(10, Math.round(x.score))),
        be: (niveaux as { be?: boolean }).be ?? false,
        label: [] as string[],
        tfOrigine: x.timeframe,
        enAttente: x.heure_entree === null,
        tsFin: x.cree_le + 40 * (DUREE_BARRE[x.timeframe] ?? 900),
      }
    })
    v12Overlay.definirTradesExternes(ouverts)
  } catch { /* silencieux */ }
}

// ── Réactions aux props (le parent est la source de vérité asset/TF) ─────────
watch(() => props.asset, (nouveau) => {
  if (nouveau !== selectedAsset.value) void changerAsset(nouveau)
})
watch(() => props.timeframe, (nouveau) => {
  if (nouveau !== selectedTimeframe.value) void changerTimeframe(nouveau)
})
// Préférences d'indicateurs appliquées (panneau global) → recharger.
watch(() => props.clePrefs, (nouvelle) => {
  if (nouvelle > 0) void chargerIndicateurs()
})

onMounted(() => {
  minuteurOverlay = setInterval(rafraichirOverlayV12, 30_000)
  chargerAnnonces()
})

onUnmounted(() => {
  window.removeEventListener('keydown', surEscape)
  if (minuteurOverlay !== null) clearInterval(minuteurOverlay)
  v12Overlay.detruire()
  dessins.detruire()
  alertesPrix.detruire()
  ecoCalDetruire()
})
</script>

<style scoped>
.cellule-plein-ecran {
  position: fixed;
  inset: 0;
  z-index: 60;
  background: #0b1220;
}
.cellule-active { outline: 2px solid rgba(34, 211, 238, 0.5); outline-offset: 2px; border-radius: 12px; }
.cellule-inactive { outline: 2px solid transparent; outline-offset: 2px; border-radius: 12px; }
</style>
