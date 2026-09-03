<template>
  <div class="h-full min-h-0 overflow-y-auto flex flex-col gap-2 pr-0.5">
    <div
      v-for="b in blocs"
      :key="b.id"
      class="rounded-xl border transition-colors cursor-pointer px-4 py-3 flex flex-col gap-2"
      :class="teinteCarte(b.id)"
      :title="`Ouvrir la page ${b.nom}`"
      @click="ouvrir(b.id)"
    >
      <!-- En-tête : identité + état + 4 badges de métriques -->
      <div class="flex items-center gap-2 flex-wrap">
        <span class="text-lg leading-none">{{ b.icone }}</span>
        <span class="font-semibold text-white text-sm">{{ b.nom }}</span>
        <span
          class="text-[10px] font-semibold px-2 py-0.5 rounded-full border"
          :class="badgeClasse(b.etat)"
        >{{ b.etat }}</span>
        <span v-if="b.perf.source === 'rejeu' && b.perf.clotures.length"
          class="text-[9px] text-white"
          title="Métriques re-dérivées du TP1 réglé (re-jeu paramétrique — tous les couples armés)"
        >depuis {{ dateCourte(b.perf.clotures[0].ferme_le) }}</span>
        <div class="ml-auto flex items-center gap-1.5 text-[10px] font-semibold whitespace-nowrap">
          <span class="px-1.5 py-0.5 rounded bg-white/10 font-mono font-bold"
                :class="rArrondi(b.perf.r_total) > 0 ? 'text-emerald-400' : rArrondi(b.perf.r_total) < 0 ? 'text-red-400' : 'text-white'"
                title="R de référence : paliers max atteints">{{ rFormate(b.perf.r_total) }}</span>
          <span v-if="b.capital" class="px-1.5 py-0.5 rounded bg-white/10 font-mono font-bold"
                :class="b.capital.capital_actuel < 0 ? 'text-red-400' : b.capital.capital_actuel >= b.capital.capital_depart ? 'text-emerald-400' : 'text-white'"
                :title="`Capital simulé — départ ${fmtDollars(b.capital.capital_depart)}, compose à chaque clôture (risque ${(b.capital.fraction_risque * 100).toFixed(b.capital.fraction_risque < 0.01 ? 1 : 0)} %/trade). Le lot de chaque trade se calcule sur ce capital.`">{{ fmtDollars(b.capital.capital_actuel) }}</span>
          <span class="px-1.5 py-0.5 rounded bg-white/10 text-white" title="Taux de réussite (R de référence > 0)">WR {{ (b.perf.taux_reussite * 100).toFixed(0) }} %</span>
          <span v-if="b.perf.recalcul" class="px-1.5 py-0.5 rounded bg-white/10 text-white animate-pulse"
                title="Re-jeu paramétrique en cours (~35 s) — les métriques vont se mettre à jour">⏳ recalcul</span>
        </div>
      </div>

      <!-- Courbe du capital simulé ($) — pleine largeur, survol = valeur -->
      <div class="relative h-16 -mx-1" @mouseleave="survolCapital = null">
        <svg
          v-if="b.capital && b.capital.points.length > 0"
          :viewBox="`0 0 ${LARGEUR} ${HAUTEUR}`"
          preserveAspectRatio="none"
          class="w-full h-full"
        >
          <polyline
            :points="pointsCapital(b)"
            fill="none"
            stroke="#60a5fa"
            stroke-width="1.5" vector-effect="non-scaling-stroke"
            stroke-linejoin="round" stroke-linecap="round"
          />
        </svg>
        <!-- Zones de survol : une par clôture, ancrées sur la courbe -->
        <div v-if="b.capital && b.capital.points.length > 0" class="absolute inset-0">
          <div
            v-for="(z, i) in zonesCapital(b)"
            :key="b.id + '-zcap' + i"
            class="absolute w-3 h-4 -translate-x-1/2 -translate-y-1/2"
            :style="{ left: z.gauche, top: z.haut }"
            @mouseenter="survolPointCapital($event, b.id, z)"
          />
        </div>
        <div v-else class="w-full h-full flex items-center justify-center text-[11px] text-white">
          Courbe du capital — dès les premières clôtures
        </div>
        <!-- Tooltip : ancré en fixed pour n'être jamais rogné -->
        <div
          v-if="survolCapital && survolCapital.bloc === b.id"
          class="fixed z-50 pointer-events-none bg-slate-900/95 border border-blue-400/30 rounded-lg px-2.5 py-1.5 shadow-xl whitespace-nowrap"
          :style="styleTooltipCapital"
        >
          <p class="text-[10px] font-bold text-white">
            {{ libelleDateCapital(survolCapital.point.ferme_le) }} · capital {{ fmtDollars(survolCapital.point.capital_apres) }}
          </p>
          <p class="text-[9px]" :class="survolCapital.point.profit >= 0 ? 'text-emerald-400' : 'text-red-400'">
            trade {{ survolCapital.point.profit >= 0 ? '+' : '−' }}{{ fmtDollars(Math.abs(survolCapital.point.profit)).replace(' $', ' $') }}
          </p>
        </div>
      </div>

      <!-- Histogramme jour par jour : Σ pips (tooltip = trades du jour) -->
      <div v-if="b.jours.length" class="relative h-10 -mx-1" @mouseleave="survolJour = null">
        <svg :viewBox="`0 0 100 ${HIST_H}`" preserveAspectRatio="none" class="w-full h-full">
          <line :x1="0" :x2="100" :y1="yZeroHistogramme" :y2="yZeroHistogramme"
            stroke="rgba(255,255,255,0.15)" stroke-width="0.4" />
          <rect
            v-for="(j, i) in b.jours"
            :key="j.date"
            :x="i * (100 / b.jours.length) + 0.6"
            :y="j.pips >= 0 ? yHistogramme(b, j.pips) : yZeroHistogramme"
            :width="100 / b.jours.length - 1.2"
            :height="hauteurBarre(b, j.pips)"
            :fill="j.pips >= 0 ? '#34d399' : '#f87171'"
            opacity="0.85"
            @mouseenter="survolBarre($event, b.id, j)"
          />
        </svg>
        <!-- Tooltip : liste des trades du jour — en fixed pour ne jamais
             être rogné par le conteneur à défilement de la carte. -->
        <div
          v-if="survolJour && survolJour.bloc === b.id"
          class="fixed z-50 pointer-events-none bg-slate-900/95 border border-white/15 rounded-lg px-2.5 py-1.5 shadow-xl whitespace-nowrap"
          :style="styleTooltip"
        >
          <p class="text-[10px] font-bold text-white mb-0.5">{{ libelleJour(survolJour.jour.date) }} — {{ survolJour.jour.pips >= 0 ? '+' : '−' }}{{ Math.abs(Math.round(survolJour.jour.pips)) }} pips</p>
          <p v-for="t in survolJour.jour.trades" :key="t.id" class="text-[9px] text-white leading-snug">
            {{ t.asset }} {{ t.tf }} · {{ t.palier }} <span :class="t.pips >= 0 ? 'text-emerald-400' : 'text-red-400'">{{ t.pips >= 0 ? '+' : '−' }}{{ Math.abs(Math.round(t.pips)) }} pips</span>
          </p>
        </div>
      </div>

      <!-- Les 4 camemberts sur une ligne : deux groupes (« Nombre de
           trades » et « Nombre de PIPS »), titre à flèches au centre de
           chaque paire, filet entre les groupes. -->
      <div v-if="b.parTf.length || b.parAsset.length || b.topTf.length || b.topAsset.length" class="flex gap-2 items-stretch">
        <div v-if="b.parTf.length || b.parAsset.length" class="flex gap-1 min-w-0 flex-1">
        <!-- Répartition par timeframe -->
        <div v-if="b.parTf.length" class="flex flex-col items-center gap-0.5 min-w-0 flex-1">
          <svg viewBox="0 0 42 42" class="w-full max-w-[64px]">
            <circle cx="21" cy="21" r="15.915" fill="none" stroke="rgba(255,255,255,0.08)" stroke-width="5" />
            <circle
              v-for="(s, i) in b.parTf" :key="'tf' + s.label"
              cx="21" cy="21" r="15.915" fill="none"
              :stroke="couleurTf(s.label)"
              stroke-width="5"
              :stroke-dasharray="`${s.part} ${100 - s.part}`"
              :stroke-dashoffset="25 - decallage(b.parTf, i)"
            />
            <text x="21" y="22" text-anchor="middle" dominant-baseline="middle"
              class="fill-white" style="font-size: 8px; font-weight: 700">{{ totalParts(b.parTf) }}</text>
          </svg>
          <p class="text-[8px] uppercase text-white tracking-wide">TF</p>
          <p class="text-[8px] leading-tight text-white text-center">
            <span v-for="s in b.parTf.slice(0, 4)" :key="'tfl' + s.label" class="whitespace-nowrap">
              <span :style="{ color: couleurTf(s.label) }">■</span> {{ s.label }} {{ s.n }}{{ ' ' }}
            </span>
          </p>
        </div>

          <div class="flex items-center justify-center shrink-0 gap-1 px-0.5">
            <span class="text-white text-[10px] leading-none">◄</span>
            <span class="text-[8px] uppercase text-white font-bold tracking-wide whitespace-nowrap">Nombre de trades</span>
            <span class="text-white text-[10px] leading-none">►</span>
          </div>
        <!-- Répartition par asset -->
        <div v-if="b.parAsset.length" class="flex flex-col items-center gap-0.5 min-w-0 flex-1">
          <svg viewBox="0 0 42 42" class="w-full max-w-[64px]">
            <circle cx="21" cy="21" r="15.915" fill="none" stroke="rgba(255,255,255,0.08)" stroke-width="5" />
            <circle
              v-for="(s, i) in b.parAsset" :key="'as' + s.label"
              cx="21" cy="21" r="15.915" fill="none"
              :stroke="couleurAsset(s.label)"
              stroke-width="5"
              :stroke-dasharray="`${s.part} ${100 - s.part}`"
              :stroke-dashoffset="25 - decallage(b.parAsset, i)"
            />
            <text x="21" y="22" text-anchor="middle" dominant-baseline="middle"
              class="fill-white" style="font-size: 8px; font-weight: 700">{{ totalParts(b.parAsset) }}</text>
          </svg>
          <p class="text-[8px] uppercase text-white tracking-wide">Asset</p>
          <p class="text-[8px] leading-tight text-white text-center">
            <span v-for="s in b.parAsset.slice(0, 4)" :key="'asl' + s.label" class="whitespace-nowrap">
              <span :style="{ color: couleurAsset(s.label) }">■</span> {{ s.label }} {{ s.n }}{{ ' ' }}
            </span>
          </p>
        </div>

        </div>
        <div v-if="(b.parTf.length || b.parAsset.length) && (b.topTf.length || b.topAsset.length)" class="w-px bg-white/10 shrink-0" />
        <div v-if="b.topTf.length || b.topAsset.length" class="flex gap-1 min-w-0 flex-1">
        <!-- Classement TF : contribution aux gains (pips positifs) -->
        <div v-if="b.topTf.length" class="flex flex-col items-center gap-0.5 min-w-0 flex-1">
          <svg viewBox="0 0 42 42" class="w-full max-w-[64px]">
            <circle cx="21" cy="21" r="15.915" fill="none" stroke="rgba(255,255,255,0.08)" stroke-width="5" />
            <circle
              v-for="(s, i) in b.topTf.filter(x => x.part > 0)" :key="'ttf' + s.label"
              cx="21" cy="21" r="15.915" fill="none"
              :stroke="couleurTf(s.label)"
              stroke-width="5"
              :stroke-dasharray="`${s.part} ${100 - s.part}`"
              :stroke-dashoffset="25 - decallage(b.topTf.filter(x => x.part > 0), i)"
            />
            <text x="21" y="22" text-anchor="middle" dominant-baseline="middle"
              :class="b.pipsNet >= 0 ? 'fill-emerald-400' : 'fill-red-400'" style="font-size: 8px; font-weight: 700">{{ b.pipsNet >= 0 ? '+' : '−' }}{{ Math.abs(Math.round(b.pipsNet)) }}</text>
          </svg>
          <p class="text-[8px] uppercase text-white tracking-wide">Classement TF</p>
          <p class="text-[8px] leading-tight text-white text-center">
            <span v-for="s in lignesClassement(b.topTf)" :key="'ttfl' + s.label" class="whitespace-nowrap">
              <span :style="{ color: s.autres ? 'rgba(255,255,255,0.35)' : couleurTf(s.label) }">■</span> {{ s.label }} <span :class="s.pips >= 0 ? 'text-emerald-400' : 'text-red-400'">{{ s.pips >= 0 ? '+' : '−' }}{{ Math.abs(s.pips) }}</span>{{ ' ' }}
            </span>
          </p>
        </div>

          <div class="flex items-center justify-center shrink-0 gap-1 px-0.5">
            <span class="text-white text-[10px] leading-none">◄</span>
            <span class="text-[8px] uppercase text-white font-bold tracking-wide whitespace-nowrap">Nombre de PIPS</span>
            <span class="text-white text-[10px] leading-none">►</span>
          </div>
        <!-- Classement asset : contribution aux gains (pips positifs) -->
        <div v-if="b.topAsset.length" class="flex flex-col items-center gap-0.5 min-w-0 flex-1">
          <svg viewBox="0 0 42 42" class="w-full max-w-[64px]">
            <circle cx="21" cy="21" r="15.915" fill="none" stroke="rgba(255,255,255,0.08)" stroke-width="5" />
            <circle
              v-for="(s, i) in b.topAsset.filter(x => x.part > 0)" :key="'tas' + s.label"
              cx="21" cy="21" r="15.915" fill="none"
              :stroke="couleurAsset(s.label)"
              stroke-width="5"
              :stroke-dasharray="`${s.part} ${100 - s.part}`"
              :stroke-dashoffset="25 - decallage(b.topAsset.filter(x => x.part > 0), i)"
            />
            <text x="21" y="22" text-anchor="middle" dominant-baseline="middle"
              :class="b.pipsNet >= 0 ? 'fill-emerald-400' : 'fill-red-400'" style="font-size: 8px; font-weight: 700">{{ b.pipsNet >= 0 ? '+' : '−' }}{{ Math.abs(Math.round(b.pipsNet)) }}</text>
          </svg>
          <p class="text-[8px] uppercase text-white tracking-wide">Classement asset</p>
          <p class="text-[8px] leading-tight text-white text-center">
            <span v-for="s in lignesClassement(b.topAsset)" :key="'tasl' + s.label" class="whitespace-nowrap">
              <span :style="{ color: s.autres ? 'rgba(255,255,255,0.35)' : couleurAsset(s.label) }">■</span> {{ s.label }} <span :class="s.pips >= 0 ? 'text-emerald-400' : 'text-red-400'">{{ s.pips >= 0 ? '+' : '−' }}{{ Math.abs(s.pips) }}</span>{{ ' ' }}
            </span>
          </p>
        </div>
        </div>
      </div>
    </div>

    <div v-if="!blocs.length && !chargement" class="flex-1 flex items-center justify-center text-sm text-white">
      Aucune stratégie active (hors construction)
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { http } from '@/services/http.client'
import { useAssetParamsStore } from '@/stores/assetParams.store'
import { palierMax, labelPalierMax } from '@/composables/useSignalFormat'
import {
  PALETTE as _PALETTE, repartition, classement, couleurTf, couleurAsset,
  decallage, totalParts, lignesClassement,
} from '@/composables/useCamemberts'
import type { PartCamembert, PartClassement } from '@/composables/useCamemberts'

interface StrategieApi {
  id: string; nom: string; icone: string; etat: string
}
interface PerfApi {
  clotures: { ferme_le: number; r_cumule: number }[]
  en_cours: unknown[]
  total: number
  non_remplis: number
  taux_reussite: number
  /** R total de référence (paliers max atteints) — métrique primaire. */
  r_total: number
  /** Présents quand la performance vient du re-jeu paramétrique (SMC).
   *  recalcul = un re-jeu est en vol après un changement de TP1. */
  source?: string
  tp1?: number
  recalcul?: boolean
}
/// Simulation composée du capital en $ (backend capital_simule) — le capital
/// de départ évolue à chaque clôture : capital += R_réalisé × capital × risque.
interface CapitalApi {
  capital_depart: number
  fraction_risque: number
  capital_actuel: number
  points: { id: string; ferme_le: number; r: number; profit: number; capital_apres: number }[]
}
interface TradeJour {
  id: string; asset: string; tf: string; palier: string; pips: number
}
interface JourHistogramme {
  date: string
  pips: number
  trades: TradeJour[]
}
interface Bloc {
  id: string; nom: string; icone: string; etat: string; perf: PerfApi
  /** Capital simulé en $ (composé à chaque clôture) — null si indisponible. */
  capital: CapitalApi | null
  /** Histogramme journalier : Σ pips par jour + trades du jour (tooltip). */
  jours: JourHistogramme[]
  /** Σ pips nets de TOUS les fermés remplis — indépendant du regroupement
   *  (centre identique des deux camemberts pips). */
  pipsNet: number
  /** Répartitions (nombre de trades) et classements (pips) des fermés remplis. */
  parTf: PartCamembert[]
  parAsset: PartCamembert[]
  topTf: PartClassement[]
  topAsset: PartClassement[]
}

interface SignalApi {
  id: string; asset: string; timeframe: string; strategie: string; statut: string
  direction: string; prix_entree: number; stop_loss: number
  take_profit: number[]; verdict: string | null; heure_entree: number | null
  ferme_le: number | null
}

const LARGEUR = 100
const HAUTEUR = 32
const HIST_H = 30
const NB_JOURS = 14

const router = useRouter()
const assetParams = useAssetParamsStore()
const blocs = ref<Bloc[]>([])
const chargement = ref(true)
const signaux = ref<SignalApi[]>([])
/** Jour survolé dans l'histogramme (tooltip, ancré en viewport). */
const survolJour = ref<{ bloc: string; jour: JourHistogramme; x: number; y: number } | null>(null)

/// Survol d'un point de la courbe capital (tooltip, ancré en viewport).
const survolCapital = ref<{ bloc: string; point: CapitalApi['points'][number]; x: number; y: number } | null>(null)

function survolPointCapital(e: MouseEvent, bloc: string, z: { point: CapitalApi['points'][number] }) {
  const r = (e.target as Element).getBoundingClientRect()
  survolCapital.value = { bloc, point: z.point, x: r.left + r.width / 2, y: r.top }
}

/// Zones de survol de la courbe capital : une par clôture, positionnées en %
/// du conteneur (même géométrie que pointsCapital — SVG étiré « none »).
function zonesCapital(b: Bloc): { gauche: string; haut: string; point: CapitalApi['points'][number] }[] {
  if (!b.capital) return []
  const serie = [b.capital.capital_depart, ...b.capital.points.map(p => p.capital_apres)]
  const min = Math.min(...serie)
  const max = Math.max(...serie)
  const amplitude = max - min || 1
  const n = serie.length
  return b.capital.points.map((p, i) => {
    const x = n > 1 ? ((i + 1) / (n - 1)) * LARGEUR : 0
    const y = HAUTEUR - 2 - ((p.capital_apres - min) / amplitude) * (HAUTEUR - 4)
    return { gauche: `${(x / LARGEUR) * 100}%`, haut: `${(y / HAUTEUR) * 100}%`, point: p }
  })
}

/// Ancrage fixed du tooltip capital : centré, au-dessus (retourné dessous si
/// près du haut), borné aux bords.
const styleTooltipCapital = computed(() => {
  const s = survolCapital.value
  if (!s) return {}
  const demi = 95
  const x = Math.min(Math.max(s.x, demi + 8), window.innerWidth - demi - 8)
  const auDessus = s.y > 220
  return {
    top: `${auDessus ? s.y - 8 : s.y + 14}px`,
    left: `${x}px`,
    transform: `translate(-50%, ${auDessus ? '-100%' : '0'})`,
  }
})

function libelleDateCapital(ts: number): string {
  return new Date(ts * 1000).toLocaleString('fr-FR', {
    day: '2-digit', month: '2-digit', hour: '2-digit', minute: '2-digit',
  })
}

/// Capture le jour survolé + les coordonnées ÉCRAN de la barre : le
/// tooltip s'ancre en position fixed, hors de tout clipping.
function survolBarre(e: MouseEvent, bloc: string, jour: JourHistogramme) {
  if (!jour.trades.length) { survolJour.value = null; return }
  const r = (e.target as Element).getBoundingClientRect()
  survolJour.value = { bloc, jour, x: r.left + r.width / 2, y: r.top }
}

/// Ancrage fixed : centré sur la barre, au-dessus (retourné sous la barre
/// si près du haut de la fenêtre), borné aux bords de la fenêtre.
const styleTooltip = computed(() => {
  const s = survolJour.value
  if (!s) return {}
  const demi = 130 // demi-largeur estimée du tooltip (nowrap)
  const x = Math.min(Math.max(s.x, demi + 8), window.innerWidth - demi - 8)
  const auDessus = s.y > 220
  return {
    left: `${Math.round(x)}px`,
    top: `${Math.round(auDessus ? s.y - 6 : s.y + 14)}px`,
    transform: auDessus ? 'translate(-50%, -100%)' : 'translateX(-50%)',
  }
})
let minuteur: ReturnType<typeof setInterval> | null = null

const ROUTES: Record<string, string> = {
  SMC: '/smc',
  straddle: '/straddle',
  rockets: '/rockets',
}

/// Trades fermés remplis avec pips et date de clôture, pour la stratégie.
/// Expiré = R de référence nul : le trade EXISTE (rempli puis expiré) —
/// il compte dans les répartitions, à 0 pip.
function tradesFerme(idStrategie: string): (TradeJour & { fermeLe: number })[] {
  const res: (TradeJour & { fermeLe: number })[] = []
  for (const s of signaux.value) {
    const strats = s.strategie.toLowerCase()
    if (s.statut !== 'Fermé' || s.heure_entree === null || s.ferme_le === null) continue
    if (idStrategie === 'SMC' ? !strats.startsWith('smc') : strats !== idStrategie) continue
    const palier = palierMax(s)
    const p = assetParams.liste.find(x => x.asset === s.asset)
    const risque = Math.abs(s.prix_entree - s.stop_loss)
    if (!p || p.taille_pip <= 0 || risque <= 0) continue
    res.push({
      id: s.id, asset: s.asset, tf: s.timeframe,
      palier: labelPalierMax(palier.palier) ?? '',
      pips: (palier.rReference ?? 0) * (risque / p.taille_pip),
      fermeLe: s.ferme_le,
    })
  }
  return res.sort((a, b) => a.fermeLe - b.fermeLe)
}

/// Histogramme : Σ pips par jour local (14 derniers jours).
function joursHistogramme(idStrategie: string): JourHistogramme[] {
  const trades = tradesFerme(idStrategie)
  if (!trades.length) return []
  const cleJour = (d: Date) =>
    `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`
  const parJour = new Map<string, JourHistogramme>()
  for (let i = NB_JOURS - 1; i >= 0; i--) {
    const cle = cleJour(new Date(Date.now() - i * 86400_000))
    parJour.set(cle, { date: cle, pips: 0, trades: [] })
  }
  for (const t of trades) {
    const jour = parJour.get(cleJour(new Date(t.fermeLe * 1000)))
    if (!jour) continue // trade plus vieux que la fenêtre
    jour.pips += t.pips
    jour.trades.push(t)
  }
  return [...parJour.values()]
}

/// R arrondi au dixième — sert AUSSI à la couleur (fini le « -0.0 R » rouge).
/// Y du zéro de l'histogramme (pips positifs au-dessus, négatifs en dessous).
const yZeroHistogramme = HIST_H / 2

function yHistogramme(b: Bloc, pips: number): number {
  const maxAbs = Math.max(...b.jours.map(j => Math.abs(j.pips)), 1)
  return HIST_H / 2 - (pips / maxAbs) * (HIST_H / 2 - 2)
}

function hauteurBarre(b: Bloc, pips: number): number {
  const maxAbs = Math.max(...b.jours.map(j => Math.abs(j.pips)), 1)
  return Math.max(0.5, (Math.abs(pips) / maxAbs) * (HIST_H / 2 - 2))
}

function libelleJour(date: string): string {
  const [, m, j] = date.split('-')
  return `${j}/${m}`
}

/// R formaté : +2.1 R / −1.5 R / 0.0 R (jamais de « -0.0 »).
function rFormate(v: number): string {
  const r = rArrondi(v)
  return `${r > 0 ? '+' : r < 0 ? '−' : ''}${Math.abs(r).toFixed(1)} R`
}

function ouvrir(id: string) {
  const cible = ROUTES[id]
  if (cible) router.push(cible)
}

/// Teinte de fond par stratégie — reprise par la page qu'elle ouvre
/// (la couleur voyage jusqu'à la page).
const TEINTES: Record<string, string> = {
  SMC: 'bg-blue-500/10 border-blue-500/25 hover:border-blue-400/50',
  straddle: 'bg-amber-500/10 border-amber-500/25 hover:border-amber-400/50',
  rockets: 'bg-orange-500/10 border-orange-500/25 hover:border-orange-400/50',
}

function teinteCarte(id: string): string {
  return TEINTES[id] ?? 'bg-white/5 border-white/10 hover:border-white/25'
}

function badgeClasse(etat: string) {
  if (etat === 'Officielle') return 'bg-emerald-500/10 text-emerald-400 border-emerald-500/30'
  if (etat === 'Observation') return 'bg-amber-500/10 text-amber-400 border-amber-500/30'
  return 'bg-gray-500/10 text-white border-gray-500/30'
}

function rArrondi(v: number): number {
  return Math.round(v * 10) / 10
}

/// Date courte JJ/MM depuis un epoch secondes.
function dateCourte(ts: number): string {
  return new Date(ts * 1000).toLocaleDateString('fr-FR', { day: '2-digit', month: '2-digit' })
}

/// Format $ : 12 345 $ — signe − typographique devant la somme négative.
function fmtDollars(v: number): string {
  const n = Math.round(Math.abs(v)).toLocaleString('fr-FR')
  return `${v < 0 ? '−' : ''}${n} $`
}

/// Points SVG de la courbe capital (bleue) : départ + une valeur par clôture,
/// échelle $ propre (min→max de la série), x aligné sur la courbe R.
function pointsCapital(b: Bloc): string {
  if (!b.capital) return ''
  const serie = [b.capital.capital_depart, ...b.capital.points.map(p => p.capital_apres)]
  const min = Math.min(...serie)
  const max = Math.max(...serie)
  const amplitude = max - min || 1
  const n = serie.length
  return serie
    .map((v, j) => {
      const x = n > 1 ? (j / (n - 1)) * LARGEUR : 0
      const y = HAUTEUR - 2 - ((v - min) / amplitude) * (HAUTEUR - 4)
      return `${x.toFixed(2)},${y.toFixed(2)}`
    })
    .join(' ')
}

async function charger() {
  try {
    if (!assetParams.liste.length) await assetParams.charger().catch(() => {})
    try {
      const sig = await http.get<SignalApi[]>('/api/signaux', { params: { limit: 150 } })
      signaux.value = sig.data
    } catch { signaux.value = [] }
    const res = await http.get<StrategieApi[]>('/api/strategies')
    const actives = (res.data as StrategieApi[]).filter(s => s.etat !== 'Construction')
    const complets = await Promise.allSettled(
      actives.map(async s => {
        let perf = PERF_VIDE
        try {
          const p = await http.get<PerfApi>(`/api/strategies/${s.id}/performance`)
          perf = p.data as PerfApi
        } catch { /* perf indisponible → bloc vide */ }
        let capital: CapitalApi | null = null
        try {
          const c = await http.get<CapitalApi>(`/api/strategies/${s.id}/capital`)
          capital = c.data as CapitalApi
        } catch { /* simulation indisponible → pas de badge ni courbe */ }
        const trades = tradesFerme(s.id)
        return {
          id: s.id, nom: s.nom, icone: s.icone, etat: s.etat, perf, capital,
          pipsNet: trades.reduce((n, t) => n + t.pips, 0),
          jours: joursHistogramme(s.id),
          parTf: repartition(trades, t => t.tf),
          parAsset: repartition(trades, t => t.asset),
          topTf: classement(trades, t => t.tf),
          topAsset: classement(trades, t => t.asset),
        }
      }),
    )
    blocs.value = complets.flatMap(p => (p.status === 'fulfilled' ? [p.value] : []))
  } catch {
    blocs.value = []
  }
  chargement.value = false
}

const PERF_VIDE: PerfApi = {
  clotures: [], en_cours: [], total: 0, non_remplis: 0, taux_reussite: 0, r_total: 0,
}

onMounted(() => {
  void charger()
  minuteur = setInterval(charger, 60_000)
})
onUnmounted(() => { if (minuteur !== null) clearInterval(minuteur) })
</script>
