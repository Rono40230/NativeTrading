<template>
  <div class="bg-white/5 border border-white/10 rounded-xl px-4 py-2.5">
    <div class="flex flex-wrap items-center gap-1.5">

      <!-- Techniques -->
      <span class="text-[10px] text-cyan-400/80 uppercase tracking-wide mr-1">Techniques</span>
      <div v-for="ind in techniques" :key="ind.key" class="flex items-center">
        <button
          @click="toggle(ind.key)"
          :class="[
            'px-2.5 py-1 rounded-l-md text-xs font-medium border-y border-l transition-all',
            (prefs as any)[ind.key] ? ind.activeClass : 'bg-white/5 border-white/10 text-slate-400 hover:text-slate-200',
            ind.params ? '' : 'rounded-r-md border-r'
          ]"
        >{{ ind.label }}</button>
        <button
          v-if="ind.params"
          @click="modaleOuverte = ind.key"
          :class="[
            'px-1.5 py-1 rounded-r-md text-[11px] border-y border-r transition-all',
            (prefs as any)[ind.key] ? ind.gearClass : 'bg-white/5 border-white/10 text-slate-500 hover:text-slate-300'
          ]"
          title="Paramètres"
        >⚙</button>
      </div>

      <!-- Tendance MTF (ex-section Analyse) — bouton + réglages -->
      <div class="flex items-center ml-1">
        <button
          @click="toggle('kasperTendance')"
          :class="[
            'px-2.5 py-1 rounded-l-md text-xs font-medium border-y border-l transition-all',
            prefs.kasperTendance
              ? 'bg-sky-500/20 border-sky-500/40 text-sky-300'
              : 'bg-white/5 border-white/10 text-slate-400 hover:text-slate-200'
          ]"
        >Tendance MTF</button>
        <button
          @click="modaleOuverte = 'kasperTendance'"
          :class="[
            'px-1.5 py-1 rounded-r-md text-[11px] border-y border-r transition-all',
            prefs.kasperTendance
              ? 'bg-sky-500/10 border-sky-500/40 text-sky-400 hover:bg-sky-500/20'
              : 'bg-white/5 border-white/10 text-slate-500 hover:text-slate-300'
          ]"
          title="Paramètres Tendance Kasper"
        >⚙</button>
      </div>

      <div class="w-px self-stretch bg-white/10 mx-1" />

      <!-- SMC + Signaux : dropdowns à cocher (même gabarit, mêmes dimensions) -->
      <template v-for="(section, idxSection) in sections" :key="section.label">
      <div v-if="idxSection > 0" class="w-px self-stretch bg-white/10 mx-1" />
      <span class="text-[10px] text-cyan-400/80 uppercase tracking-wide mr-1 ml-1">{{ section.label }}</span>
      <div v-for="groupe in section.groupes" :key="`g-${groupe.label}`" class="relative">
        <button
          @click="ouvert = ouvert === groupe.label ? null : groupe.label"
          :title="`${nbActifs(groupe)} actif(s) — clic pour ouvrir`"
          :class="[
            'flex items-center gap-1 px-2.5 py-1 rounded-md text-xs font-medium border transition-all',
            nbActifs(groupe) > 0
              ? 'bg-cyan-500/20 border-cyan-500/40 text-cyan-200'
              : 'bg-white/5 border-white/10 text-slate-400 hover:text-slate-200'
          ]"
        >
          {{ groupe.label }}
          <span
            v-if="nbActifs(groupe) > 0"
            class="min-w-[16px] px-1 rounded-full bg-cyan-500/30 text-cyan-100 text-[10px] leading-[14px] text-center"
          >{{ nbActifs(groupe) }}</span>
          <span :class="['text-[8px] transition-transform', ouvert === groupe.label && 'rotate-180']">▼</span>
        </button>

        <!-- Backdrop : clic extérieur = fermer -->
        <div v-if="ouvert === groupe.label" class="fixed inset-0 z-40" @click="ouvert = null" />
        <div
          v-if="ouvert === groupe.label"
          class="absolute left-0 bottom-[calc(100%+4px)] z-50 w-60 bg-slate-900/95 backdrop-blur border border-white/10 rounded-lg shadow-xl py-1"
        >
          <div class="flex items-center justify-between px-2.5 py-1 border-b border-white/5">
            <span class="text-[10px] uppercase tracking-wide text-slate-500">{{ groupe.label }}</span>
            <span class="flex gap-2 text-[10px]">
              <button class="text-cyan-400 hover:text-cyan-300" @click="toutOuRien(groupe, true)">Tout</button>
              <button class="text-slate-500 hover:text-slate-300" @click="toutOuRien(groupe, false)">Aucun</button>
            </span>
          </div>
          <label
            v-for="ind in groupe.items"
            :key="ind.key"
            :title="ind.pending ? 'Donnée non encore exposée par /api/smc/v12/analyse' : 'Afficher / Masquer'"
            :class="[
              'flex items-center gap-2 px-2.5 py-1.5 text-[11px] cursor-pointer hover:bg-white/5 text-slate-300'
            ]"
          >
            <input
              type="checkbox"
              :checked="(prefs as any)[ind.key]"
              class="accent-cyan-500 w-3 h-3"
              @change="toggle(ind.key)"
            />
            <span :class="(prefs as any)[ind.key] && 'text-cyan-200'">{{ ind.label }}</span>
          </label>
        </div>
      </div>
      </template>

      <!-- Slot actions SMC (ex. bouton Analyse SMC) — à droite des dropdowns -->
      <slot name="apres-smc" />

    </div>

    <IndicatorModal
      :indicateur="modaleOuverte"
      v-model="prefs"
      @fermer="modaleOuverte = null"
      @appliquer="validerModal"
    />
  </div>
</template>

<script setup lang="ts">
import { computed, onUnmounted, ref } from 'vue'
import type { PrefsIndicateurs } from '@/stores/settings.store'
import IndicatorModal from './IndicatorModal.vue'

const prefs = defineModel<PrefsIndicateurs>({ required: true })
defineProps<{}>()

const emit = defineEmits<{ appliquer: [] }>()

const modaleOuverte = ref<string | null>(null)
/** Famille dont le menu déroulant est ouvert (null = tous fermés). */
const ouvert = ref<string | null>(null)

/** Fermeture au clavier (Échap) — posé sur le window au montage. */
function surEchap(e: KeyboardEvent) {
  if (e.key === 'Escape') ouvert.value = null
}
window.addEventListener('keydown', surEchap)
onUnmounted(() => window.removeEventListener('keydown', surEchap))

function toggle(key: string) {
  ;(prefs.value as any)[key] = !(prefs.value as any)[key]
  emit('appliquer')
}

/** Nombre d'indicateurs actifs dans une famille (badge du bouton). */
function nbActifs(groupe: { items: { key: string }[] }): number {
  return groupe.items.filter((i) => (prefs.value as any)[i.key]).length
}

/** Tout afficher / tout masquer dans une famille. */
function toutOuRien(groupe: { items: { key: string }[] }, valeur: boolean) {
  for (const i of groupe.items) (prefs.value as any)[i.key] = valeur
  emit('appliquer')
}

function validerModal() {
  modaleOuverte.value = null
  emit('appliquer')
}

const techniques = [
  { key: 'ema',       label: 'EMA',       params: true,  activeClass: 'bg-amber-500/20 border-amber-500/40 text-amber-300',    gearClass: 'bg-amber-500/10 border-amber-500/40 text-amber-400 hover:bg-amber-500/20'   },
  { key: 'rsi',       label: 'RSI',       params: true,  activeClass: 'bg-purple-500/20 border-purple-500/40 text-purple-300', gearClass: 'bg-purple-500/10 border-purple-500/40 text-purple-400 hover:bg-purple-500/20' },
  { key: 'macd',      label: 'MACD',      params: true,  activeClass: 'bg-emerald-500/20 border-emerald-500/40 text-emerald-300', gearClass: 'bg-emerald-500/10 border-emerald-500/40 text-emerald-400 hover:bg-emerald-500/20' },
  { key: 'bollinger', label: 'Bollinger', params: true,  activeClass: 'bg-indigo-500/20 border-indigo-500/40 text-indigo-300',   gearClass: 'bg-indigo-500/10 border-indigo-500/40 text-indigo-400 hover:bg-indigo-500/20' },
  { key: 'atr',       label: 'ATR',       params: true,  activeClass: 'bg-rose-500/20 border-rose-500/40 text-rose-300',         gearClass: 'bg-rose-500/10 border-rose-500/40 text-rose-400 hover:bg-rose-500/20' },
]

// ── SMC v12 : bascules ON/OFF par indicateur (overlay useSmcV12Overlay).
// `pending = true` → aucune donnée dessinée derrière la case (à implémenter)
// bascule le flag mais rien n'est dessiné pour l'instant.
const v12Groupes = [
  {
    label: 'Structure',
    items: [
      { key: 'v12Structure', label: 'HH/HL/LH/LL', pending: false },
      { key: 'v12Bos',       label: 'Break of Structure', pending: false },
      { key: 'v12Mss',       label: 'Market Structure Shift', pending: false },
      { key: 'v12Choch',     label: 'CHange Of Character', pending: false },
      { key: 'v12Tendance',  label: 'Fond de tendance', pending: false },
      { key: 'v12Impulsion', label: 'Fond impulsion', pending: false },
    ],
  },
  {
    label: 'Niveaux de liquidité',
    items: [
      { key: 'v12NiveauxCles', label: 'Previous High/Low Day/Week', pending: false },
      { key: 'v12EqhEql',      label: 'EQual High / EQual Low', pending: false },
      { key: 'v12AsianHl',     label: 'Asian High / Asian Low', pending: false },
      { key: 'v12Sweeps',      label: 'Sweeps (prise de liquidité)', pending: false },
      { key: 'v12Ndog',        label: 'NDOG', pending: false },
      { key: 'v12Nwog',        label: 'NWOG', pending: false },
    ],
  },
  {
    label: 'Zones de liquidité (POI)',
    items: [
      { key: 'v12Ob',          label: 'Order Blocks', pending: false },
      { key: 'v12Fvg',         label: 'Fair Value Gap', pending: false },
      { key: 'v12Breaker',     label: 'Breaker Blocks', pending: false },
      { key: 'v12Propulsion',  label: 'Propulsion Blocks', pending: false },
      { key: 'v12Imbalance',   label: 'Imbalance', pending: false },
      { key: 'v12ZoneCoeur',   label: 'Zone cœur', pending: false },
      { key: 'v12Ote',         label: 'Optimal Trade Entry', pending: false },
      { key: 'v12Premium',     label: 'Premium / Discount', pending: false },
      { key: 'v12Equilibrium', label: 'Equilibrium', pending: false },
      { key: 'v12Volume',      label: 'Fond volume', pending: false },
    ],
  },
  {
    label: 'Sessions',
    items: [
      { key: 'v12SessionAsie',    label: 'Session Asiatique', pending: false },
      { key: 'v12SessionLondres', label: 'Session Européenne', pending: false },
      { key: 'v12SessionNy',      label: 'Session Américaine', pending: false },
    ],
  },
  {
    label: 'Confirmations MTF',
    items: [
      { key: 'v12ObH1', label: 'OB H1', pending: false },
      { key: 'v12ObH4', label: 'OB H4', pending: false },
      { key: 'v12ObW1', label: 'OB W1', pending: false },
      { key: 'v12ObMn', label: 'OB MN', pending: false },
    ],
  },
]

/** Stratégies du dropdown « Signaux » (multi-sélection ; Trades = SMC v12). */
const strategieSignaux = {
  label: 'Stratégies',
  items: [
    { key: 'v12Signals',         label: 'SMC',      pending: false },
    { key: 'v12SignauxRockets',  label: 'Rockets',  pending: true },
    { key: 'v12SignauxStraddle', label: 'Straddle', pending: true },
  ],
}

/** Sections du panneau : SMC (5 familles d'indicateurs) puis Signaux. */
const sections = computed(() => [
  { label: 'SMC', groupes: v12Groupes },
  { label: 'Signaux', groupes: [strategieSignaux] },
])
</script>

