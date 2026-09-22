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
        <span v-if="b.analyse && b.analyse.nb_trades"
              class="text-[9px] text-white"
              :title="`Base vécue complète : ${b.analyse.nb_trades} clôtures, ${dateCourte(b.analyse.fenetre_debut)} → ${dateCourte(b.analyse.fenetre_fin)}`"
        >{{ b.analyse.nb_trades }} clôtures</span>
        <!-- Accès directs (workflow 09/09) : caractéristiques + réglages
             en modale, par stratégie (composant dédié). -->
        <ReglagesCarteBoutons :id="b.id" />
        <!-- Laboratoire : re-jeu paramétrique à la demande, sans jamais
             toucher aux chiffres officiels (décision 15/09 nuit). -->
        <button
          class="text-[10px] font-semibold px-2 py-0.5 rounded-md border border-teal-500/30 bg-teal-500/10 text-teal-300 hover:bg-teal-500/20 transition-colors whitespace-nowrap"
          title="Laboratoire de simulation — tester des réglages sans toucher aux chiffres officiels"
          @click.stop="router.push(`/simulation?strategie=${b.id}`)"
        >🧪 Simulation</button>
        <div class="ml-auto flex items-center gap-1.5 text-[10px] font-semibold whitespace-nowrap">
          <!-- Son Telegram : bascule directe du réglage de la stratégie
               (PUT partiel — effet immédiat, l'envoi lit le drapeau à
               chaque signal). Ne déclenche pas l'ouverture de la carte. -->
          <button
            class="px-1.5 py-0.5 rounded bg-white/10 font-semibold transition-colors hover:bg-white/20 disabled:opacity-40"
            :class="b.notifications ? 'text-emerald-300' : 'text-white/60'"
            :title="titreTelegram(b)"
            :disabled="basculeTelegram === b.id"
            @click.stop="basculerTelegram(b)"
          >{{ b.notifications ? '🔔' : '🔕' }}</button>
          <span class="px-1.5 py-0.5 rounded bg-white/10 font-mono font-bold"
                :class="rArrondi(b.analyse?.r_total ?? 0) > 0 ? 'text-emerald-400' : rArrondi(b.analyse?.r_total ?? 0) < 0 ? 'text-red-400' : 'text-white'"
                :title="titleR">{{ rFormate(b.analyse?.r_total ?? 0) }}</span>
          <span v-if="b.capital" class="px-1.5 py-0.5 rounded bg-white/10 font-mono font-bold"
                :class="b.capital.capital_actuel < 0 ? 'text-red-400' : b.capital.capital_actuel >= b.capital.capital_depart ? 'text-emerald-400' : 'text-white'"
                :title="`Capital simulé — départ ${fmtDollars(b.capital.capital_depart)}, compose à chaque clôture (risque ${(b.capital.fraction_risque * 100).toFixed(b.capital.fraction_risque < 0.01 ? 1 : 0)} %/trade). Le lot de chaque trade se calcule sur ce capital.`">{{ fmtDollars(b.capital.capital_actuel) }}</span>
          <span class="px-1.5 py-0.5 rounded bg-white/10 text-white" title="WR — part des clôtures gagnantes ($ > 0), base vécue complète">WR {{ ((b.analyse?.taux_reussite ?? 0) * 100).toFixed(0) }} %</span>
        <span v-if="b.id === 'rockets' && nbPositionsRocket > 0"
              class="px-1.5 py-0.5 rounded bg-purple-700/40 text-purple-200 font-mono font-bold cursor-help"
              :title="titrePositionsRocket">{{ nbPositionsRocket }} en cours 🚀</span>
        <span v-else-if="(enCoursBloc[b.id] || []).length > 0"
              class="px-1.5 py-0.5 rounded bg-blue-500/30 text-blue-200 font-mono font-bold cursor-help"
              :title="titreEnCours(b.id)">{{ (enCoursBloc[b.id] || []).length }} en cours</span>
        </div>
      </div>

      <!-- Courbe du capital simulé ($) — pleine largeur, survol = valeur.
           Bicolore au capital de départ (CourbeCapital) : verte au-dessus
           de la ligne pointillée, rouge en dessous. -->
      <div class="relative h-16 -mx-1" @mouseleave="survolCapital = null">
        <CourbeCapital
          v-if="b.capital && b.capital.points.length > 0"
          :capital="b.capital"
          :id-bloc="b.id"
        />
        <!-- Zones de survol : une par clôture, ancrées sur la courbe -->
        <div v-if="b.capital && b.capital.points.length > 0" class="absolute inset-0">
          <div
            v-for="(z, i) in zonesCapital(b.capital)"
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

      <!-- Histogramme jour par jour : Σ $ (sans survol — décision 15/09) -->
      <div v-if="b.jours.length" class="relative h-10 -mx-1">
        <svg :viewBox="`0 0 100 ${HIST_H}`" preserveAspectRatio="none" class="w-full h-full">
          <line :x1="0" :x2="100" :y1="yZeroHistogramme" :y2="yZeroHistogramme"
            stroke="rgba(255,255,255,0.15)" stroke-width="0.4" />
          <rect
            v-for="(j, i) in b.jours"
            :key="j.date"
            :x="i * (100 / b.jours.length) + 0.6"
            :y="j.dollars >= 0 ? yHistogramme(b, j.dollars) : yZeroHistogramme"
            :width="100 / b.jours.length - 1.2"
            :height="hauteurBarre(b, j.dollars)"
            :fill="j.dollars >= 0 ? '#34d399' : '#f87171'"
            opacity="0.85"
          />
        </svg>
      </div>

      <!-- Les 4 camemberts sur une ligne : deux groupes (« Nombre de
           trades » et « Dollars réels »), titre à flèches au centre de
           chaque paire, filet entre les groupes. -->
      <div v-if="b.parTf.length || b.parAsset.length || b.topTf.length || b.topAsset.length" class="flex gap-2 items-stretch" title="Base vécue complète (expirés compris) — mêmes clôtures que le badge R, le WR et la courbe de capital. Dollars : contribution composée au capital.">
        <div v-if="b.parTf.length || b.parAsset.length" class="flex gap-1 min-w-0 flex-1">
        <!-- Répartition par timeframe -->
        <div v-if="b.parTf.length" class="flex flex-col items-center gap-0.5 min-w-0 flex-1">
          <svg viewBox="0 0 42 42" class="w-full max-w-[84px]">
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
          <p class="text-[8px] uppercase text-white tracking-wide">{{ b.id === 'rockets' ? 'Verdicts' : 'TF' }}</p>
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
          <svg viewBox="0 0 42 42" class="w-full max-w-[84px]">
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
        <!-- Classement TF : contribution réelle au capital ($) -->
        <div v-if="b.topTf.length" class="flex flex-col items-center gap-0.5 min-w-0 flex-1">
          <svg viewBox="0 0 42 42" class="w-full max-w-[84px]">
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
              :class="b.dollarsNet >= 0 ? 'fill-emerald-400' : 'fill-red-400'" style="font-size: 7px; font-weight: 700">{{ fmtDollarsCourt(b.dollarsNet) }}</text>
          </svg>
          <p class="text-[8px] uppercase text-white tracking-wide">{{ b.id === 'rockets' ? 'Classement univers' : 'Classement TF' }}</p>
          <p class="text-[8px] leading-tight text-white text-center">
            <span v-for="s in lignesClassement(b.topTf)" :key="'ttfl' + s.label" class="whitespace-nowrap">
              <span :style="{ color: s.autres ? 'rgba(255,255,255,0.35)' : couleurTf(s.label) }">■</span> {{ s.label }} <span :class="s.valeur >= 0 ? 'text-emerald-400' : 'text-red-400'">{{ fmtDollarsCourt(s.valeur) }}</span>{{ ' ' }}
            </span>
          </p>
        </div>

          <div class="flex items-center justify-center shrink-0 gap-1 px-0.5">
            <span class="text-white text-[10px] leading-none">◄</span>
            <span class="text-[8px] uppercase text-white font-bold tracking-wide whitespace-nowrap">Dollars réels</span>
            <span class="text-white text-[10px] leading-none">►</span>
          </div>
        <!-- Classement asset : contribution réelle au capital ($) -->
        <div v-if="b.topAsset.length" class="flex flex-col items-center gap-0.5 min-w-0 flex-1">
          <svg viewBox="0 0 42 42" class="w-full max-w-[84px]">
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
              :class="b.dollarsNet >= 0 ? 'fill-emerald-400' : 'fill-red-400'" style="font-size: 7px; font-weight: 700">{{ fmtDollarsCourt(b.dollarsNet) }}</text>
          </svg>
          <p class="text-[8px] uppercase text-white tracking-wide">Classement asset</p>
          <p class="text-[8px] leading-tight text-white text-center">
            <span v-for="s in lignesClassement(b.topAsset)" :key="'tasl' + s.label" class="whitespace-nowrap">
              <span :style="{ color: s.autres ? 'rgba(255,255,255,0.35)' : couleurAsset(s.label) }">■</span> {{ s.label }} <span :class="s.valeur >= 0 ? 'text-emerald-400' : 'text-red-400'">{{ fmtDollarsCourt(s.valeur) }}</span>{{ ' ' }}
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
import CourbeCapital from './CourbeCapital.vue'
import ReglagesCarteBoutons from './ReglagesCarteBoutons.vue'
import { usePositionsRockets, plLatent, plNeutralisee } from '@/composables/usePositionsRockets'
import { useEnCoursStrategies } from '@/composables/useEnCoursStrategies'
import { useAlerteStore } from '@/stores/alerte.store'
import { zonesCapital, type PointCapital } from '@/composables/useCourbeCapital'
import { chargerAnalyse, type AnalyseStrategie, type CategorieAnalyse } from '@/composables/useAnalyses'
import {
  classement, couleurTf, couleurAsset,
  decallage, totalParts, lignesClassement,
  type PartCamembert, type PartClassement,
} from '@/composables/useCamemberts'

interface StrategieApi {
  id: string; nom: string; icone: string; etat: string; notifications: boolean
}
/// Simulation composée du capital en $ (backend capital_simule) — le capital
/// de départ évolue à chaque clôture : capital += R_réalisé × capital × risque.
/// SERVÉ UNIQUEMENT POUR LA COURBE (points) : tous les agrégats affichés
/// (R, WR, camemberts, histogramme) viennent de l'analyse — même source que
/// le rapport d'activité, harmonisation 15/09 soir.
interface CapitalApi {
  capital_depart: number
  fraction_risque: number
  capital_actuel: number
  points: { id: string; ferme_le: number; r: number; profit: number; capital_apres: number; asset?: string; tf?: string; verdict?: string }[]
}
interface JourHistogramme {
  date: string
  dollars: number
}
interface Bloc {
  id: string; nom: string; icone: string; etat: string
  /** Son Telegram de la stratégie (drapeau registre) — bascule directe. */
  notifications: boolean
  /** L'analyse officielle (backend analyses.rs) — LA source de tous les
   *  chiffres affichés : R-distance, WR $, effectifs, catégories. */
  analyse: AnalyseStrategie | null
  /** Capital simulé en $ — courbe uniquement (points composés). */
  capital: CapitalApi | null
  /** Histogramme journalier : Σ $ par jour (14 derniers jours, analyser). */
  jours: JourHistogramme[]
  /** Σ $ nets de TOUTES les clôtures composées (= capital_actuel − départ) —
   *  centre des camemberts $, indépendant du regroupement. */
  dollarsNet: number
  /** Répartitions (nombre de clôtures vécues) et classements ($) —
   *  catégories de l'analyse, effectif = nb_trades garanti. */
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

const HIST_H = 30
const NB_JOURS = 14

const alerteStore = useAlerteStore()
const router = useRouter()

// ── Son Telegram : bascule directe sur la carte ─────────────────────────────
// PUT partiel du registre ({ notifications }) — l'envoi relit le drapeau à
// CHAQUE signal (signaux_officiels) : effet immédiat, sans relance.
const basculeTelegram = ref('')

async function basculerTelegram(b: Bloc) {
  basculeTelegram.value = b.id
  try {
    const res = await http.put<{ notifications: boolean }>(`/api/strategies/${b.id}`, {
      notifications: !b.notifications,
    })
    b.notifications = res.data.notifications
  } catch (e) {
    alerteStore.afficherErreur(`Telegram ${b.id} : bascule échouée — ${(e as Error).message}`)
  }
  basculeTelegram.value = ''
}

/// Info-bulle du bouton : la règle d'envoi complète — le drapeau ET l'état
/// (Observation = silencieux, décision 15/09 ; le bouton pré-règle le drapeau).
function titreTelegram(b: Bloc): string {
  const lignes = [
    `Telegram — messages d'imminence : ${b.notifications ? 'ACTIVÉS' : 'COUPÉS'} (clic pour ${b.notifications ? 'couper' : 'activer'}).`,
    "Condition complète d'envoi : réglage activé ET stratégie Officielle.",
  ]
  if (b.etat !== 'Officielle') {
    lignes.push(`Ici état ${b.etat} → silencieux tant que la stratégie ne repasse pas Officielle.`)
  }
  return lignes.join('\n')
}

// Positions rockets ouvertes : badge vivant de la carte (poste
// d'observation — P/L latent en infobulle).
const { risque: risqueRocket, neutralisees: neutraliseesRocket, live: liveRocket } = usePositionsRockets()
const nbPositionsRocket = computed(() => risqueRocket.value.length + neutraliseesRocket.value.length)
const titrePositionsRocket = computed(() => {
  const lignes: string[] = ['Positions ouvertes — pilotage automatique (30 s)']
  for (const p of risqueRocket.value) {
    const pl = plLatent(liveRocket.value, p)
    lignes.push(`${p.symbole} : ${p.r1 >= 0 ? '' : ''}R1 ${p.r1.toFixed(2)} · P/L latent ${pl === null ? '—' : (pl >= 0 ? '+' : '−') + Math.abs(pl).toFixed(2) + ' $'}`)
  }
  for (const p of neutraliseesRocket.value) {
    const pl = plNeutralisee(liveRocket.value, p)
    lignes.push(`${p.symbole} : neutralisée · trailing ${p.trailing?.toFixed(2) ?? '—'} · P/L ${pl === null ? '—' : (pl >= 0 ? '+' : '−') + Math.abs(pl).toFixed(2) + ' $'}`)
  }
  return lignes.join('\n')
})
const blocs = ref<Bloc[]>([])
const chargement = ref(true)
const signaux = ref<SignalApi[]>([])
// Badge « N en cours » des autres cartes (15/09) : signaux Actifs remplis.
const { parBloc: enCoursBloc, titre: titreEnCours } = useEnCoursStrategies(signaux)
/// Survol d'un point de la courbe capital (tooltip, ancré en viewport).
const survolCapital = ref<{ bloc: string; point: PointCapital; x: number; y: number } | null>(null)

function survolPointCapital(e: MouseEvent, bloc: string, z: { point: PointCapital }) {
  const r = (e.target as Element).getBoundingClientRect()
  survolCapital.value = { bloc, point: z.point, x: r.left + r.width / 2, y: r.top }
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

let minuteur: ReturnType<typeof setInterval> | null = null

const ROUTES: Record<string, string> = {
  SMC: '/smc',
  straddle: '/straddle',
  rockets: '/rockets',
  kdj_halftrend: '/kdj',
}

/// Parts « nombre de clôtures » d'une catégorie de l'analyse (TF, asset,
/// verdict) — effectif TOUJOURS égal au nb_trades du badge (harmonisation
/// 15/09 : plus aucun recalcul front, plus de fenêtre 150 ni d'expirés exclus).
function partsDesCategories(cats: CategorieAnalyse[]): PartCamembert[] {
  const total = cats.reduce((n, c) => n + c.n, 0) || 1
  return cats
    .map(c => ({ label: c.label, n: c.n, part: (c.n / total) * 100 }))
    .sort((a, b) => b.n - a.n)
}

/// Histogramme : Σ $ par jour local — derniers jours du journalier servi par
/// l'analyse (mêmes clôtures que le reste de la carte).
function joursHistogramme(journalier: { cle: string; dollars: number }[]): JourHistogramme[] {
  return journalier.slice(-NB_JOURS).map(p => ({ date: p.cle, dollars: p.dollars }))
}

/// Y du zéro de l'histogramme ($ positifs au-dessus, négatifs en dessous).
const yZeroHistogramme = HIST_H / 2

function yHistogramme(b: Bloc, dollars: number): number {
  const maxAbs = Math.max(...b.jours.map(j => Math.abs(j.dollars)), 1)
  return HIST_H / 2 - (dollars / maxAbs) * (HIST_H / 2 - 2)
}

function hauteurBarre(b: Bloc, dollars: number): number {
  const maxAbs = Math.max(...b.jours.map(j => Math.abs(j.dollars)), 1)
  return Math.max(0.5, (Math.abs(dollars) / maxAbs) * (HIST_H / 2 - 2))
}

/// R formaté : +2.1 R / −1.5 R / 0.0 R (jamais de « -0.0 »).
function rFormate(v: number): string {
  const r = rArrondi(v)
  return `${r > 0 ? '+' : r < 0 ? '−' : ''}${Math.abs(r).toFixed(1)} R`
}

/// Info-bulle du badge R : la convention officielle (encaissé = gagnants −
/// perdants, décision 16/09) — la même que le rapport et l'historique.
const titleR = 'R encaissés : gagnants − perdants, ventes partielles comprises (décision 16/09).\nLe badge $ compose exactement ces R — même histoire, unités différentes.\nLa distance (meilleur palier atteint) reste visible au laboratoire de simulation.'

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
  kdj_halftrend: 'bg-cyan-500/10 border-cyan-500/25 hover:border-cyan-400/50',
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

/// Format $ compact pour les centres/légendes de camemberts : +93 $ / −1.2k $.
function fmtDollarsCourt(v: number): string {
  const a = Math.abs(v)
  const corps = a >= 1000 ? `${(a / 1000).toFixed(a >= 10_000 ? 0 : 1).replace('.', ',')}k` : `${Math.round(a)}`
  return `${v > 0 ? '+' : v < 0 ? '−' : ''}${corps} $`
}

async function charger() {
  try {
    try {
      const sig = await http.get<SignalApi[]>('/api/signaux', { params: { limit: 150 } })
      signaux.value = sig.data
    } catch { signaux.value = [] }
    const res = await http.get<StrategieApi[]>('/api/strategies')
    const actives = (res.data as StrategieApi[]).filter(s => s.etat !== 'Construction')
    const complets = await Promise.allSettled(
      actives.map(async s => {
        // L'ANALYSE est la source unique des chiffres (harmonisation 15/09) :
        // badge R, WR, camemberts, histogramme — exactement les mêmes nombres
        // que le rapport d'activité. Le /capital ne sert plus que la courbe.
        const analyse = await chargerAnalyse(s.id)
        let capital: CapitalApi | null = null
        try {
          const c = await http.get<CapitalApi>(`/api/strategies/${s.id}/capital`)
          capital = c.data as CapitalApi
        } catch { /* simulation indisponible → pas de courbe */ }
        const cats = analyse?.tfs ?? []
        const catsAsset = analyse?.assets ?? []
        // Rockets : le TF ne dit rien (D1 unique) — le camembert montre les
        // VERDICTS (TS/SL) et le classement les UNIVERS (crypto vs actions).
        const estRocket = s.id === 'rockets'
        const univers = (a: string) => (a.endsWith('USDT') ? 'Crypto' : 'Action')
        return {
          id: s.id, nom: s.nom, icone: s.icone, etat: s.etat,
          notifications: s.notifications, analyse, capital,
          dollarsNet: (analyse?.capital_actuel ?? 0) - (analyse?.capital_depart ?? 0),
          jours: joursHistogramme(analyse?.journalier ?? []),
          parTf: estRocket
            ? partsDesCategories(analyse?.verdicts ?? [])
            : partsDesCategories(cats),
          parAsset: partsDesCategories(catsAsset),
          topTf: estRocket
            ? classement(catsAsset, c => univers(c.label), c => c.dollars)
            : classement(cats, c => c.label, c => c.dollars),
          topAsset: classement(catsAsset, c => c.label, c => c.dollars),
        }
      }),
    )
    blocs.value = complets.flatMap(p => (p.status === 'fulfilled' ? [p.value] : []))
  } catch {
    blocs.value = []
  }
  chargement.value = false
}

onMounted(() => {
  void charger()
  minuteur = setInterval(charger, 60_000)
})
onUnmounted(() => { if (minuteur !== null) clearInterval(minuteur) })
</script>
