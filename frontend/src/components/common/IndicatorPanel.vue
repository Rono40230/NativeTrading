<template>
  <div class="bg-white/5 border border-white/10 rounded-xl px-4 py-2.5">
    <div class="flex flex-wrap items-center gap-1.5">

      <!-- Techniques -->
      <span class="text-[10px] text-slate-500 uppercase tracking-wide mr-1">Techniques</span>
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

      <div class="w-px self-stretch bg-white/10 mx-1" />

      <!-- SMC v12 -->
      <span class="text-[10px] text-cyan-400/80 uppercase tracking-wide mr-1">SMC v12</span>
      <template v-for="groupe in v12Groupes" :key="groupe.label">
        <span class="text-[9px] text-slate-500 uppercase tracking-wide mr-0.5 ml-1">{{ groupe.label }}</span>
        <button
          v-for="ind in groupe.items"
          :key="ind.key"
          @click="toggle(ind.key)"
          :title="ind.pending ? 'Donnée non encore exposée par /api/smc/v12/analyse' : 'Afficher / Masquer'"
          :class="[
            'px-2 py-0.5 rounded-md text-[11px] font-medium border transition-all',
            (prefs as any)[ind.key]
              ? 'bg-cyan-500/20 border-cyan-500/40 text-cyan-200'
              : ind.pending
                ? 'bg-white/5 border-white/10 text-slate-500 hover:text-slate-300 italic'
                : 'bg-white/5 border-white/10 text-slate-400 hover:text-slate-200'
          ]"
        >{{ ind.label }}</button>
      </template>

      <div class="w-px self-stretch bg-white/10 mx-1" />

      <!-- Analyse -->
      <span class="text-[10px] text-slate-500 uppercase tracking-wide mr-1">Analyse</span>
      <div class="flex items-center">
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

      <!-- Actualiser -->
      <button
        class="ml-auto px-3 py-1 text-xs rounded-lg bg-white/5 border border-white/10 text-gray-300 hover:bg-white/10 transition-colors disabled:opacity-50"
        :disabled="chargement"
        @click="$emit('actualiser')"
      >{{ chargement ? '⏳...' : '🔄 Actualiser' }}</button>
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
import { ref } from 'vue'
import type { PrefsIndicateurs } from '@/stores/settings.store'
import IndicatorModal from './IndicatorModal.vue'

const prefs = defineModel<PrefsIndicateurs>({ required: true })
const props = defineProps<{ chargement?: boolean }>()

const emit = defineEmits<{ appliquer: []; actualiser: [] }>()

const modaleOuverte = ref<string | null>(null)

function toggle(key: string) {
  ;(prefs.value as any)[key] = !(prefs.value as any)[key]
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
// `pending = true` → l'API /api/smc/v12/analyse ne retourne pas encore cette
// donnée : le bouton bascule le flag mais rien n'est dessiné pour l'instant.
const v12Groupes = [
  {
    label: 'Structure',
    items: [
      { key: 'v12Structure', label: 'HH/HL/LH/LL', pending: false },
      { key: 'v12Bos',       label: 'Break of Structure', pending: false },
      { key: 'v12Mss',       label: 'Market Structure Shift', pending: false },
      { key: 'v12Choch',     label: 'CHange Of Character', pending: false },
      { key: 'v12Sweeps',    label: 'Sweeps',    pending: false },
      { key: 'v12EqhEql',    label: 'EQual High / EQual Low', pending: false },
      { key: 'v12Tendance',  label: 'Fond de tendance', pending: false },
    ],
  },
  {
    label: 'Zones',
    items: [
      { key: 'v12Ob',        label: 'Order Blocks', pending: false },
      { key: 'v12Fvg',       label: 'Fair Value Gap', pending: false },
      { key: 'v12Breaker',   label: 'Breakers',  pending: true  },
      { key: 'v12Imbalance', label: 'Imbalance', pending: true  },
      { key: 'v12ZoneCoeur', label: 'Zone cœur', pending: true  },
      { key: 'v12Signals',   label: 'Trades',    pending: false },
    ],
  },
  {
    label: 'Tech',
    items: [
      { key: 'v12Volume',     label: 'Fond volume',     pending: true },
      { key: 'v12Impulsion',  label: 'Fond impulsion',  pending: true },
    ],
  },
  {
    label: 'Sessions',
    items: [
      { key: 'v12SessionAsie',   label: 'Session Asiatique', pending: false },
      { key: 'v12SessionLondres',label: 'Session Européenne', pending: false },
      { key: 'v12SessionNy',     label: 'Session Américaine', pending: false },
      { key: 'v12AsianHl',       label: 'Asian High / Asian Low', pending: false },
      { key: 'v12NiveauxCles',   label: 'Previous High/Low Day/Week', pending: false },
      { key: 'v12Ndog',          label: 'NDOG',         pending: true },
      { key: 'v12Nwog',          label: 'NWOG',         pending: true },
    ],
  },
  {
    label: 'Multi-TF',
    items: [
      { key: 'v12Premium',     label: 'Fond Prem/Disc', pending: true },
      { key: 'v12Equilibrium', label: 'Equilibrium',    pending: true },
      { key: 'v12ObH1',        label: 'OB H1',          pending: true },
      { key: 'v12ObH4',        label: 'OB H4',          pending: true },
      { key: 'v12ObW1',        label: 'OB W1',          pending: true },
      { key: 'v12ObMn',        label: 'OB MN',          pending: true },
      { key: 'v12Ote',         label: 'Zone OTE',       pending: true },
    ],
  },
]
</script>

