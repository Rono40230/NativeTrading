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

      <!-- SMC -->
      <span class="text-[10px] text-slate-500 uppercase tracking-wide mr-1">SMC</span>
      <div v-for="ind in smcOptions" :key="ind.key" class="flex items-center">
        <button
          @click="toggle(ind.key)"
          :class="[
            'px-2.5 py-1 rounded-l-md text-xs font-medium border-y border-l transition-all',
            (prefs as any)[ind.key] ? ind.activeClass : 'bg-white/5 border-white/10 text-slate-400 hover:text-slate-200'
          ]"
        >{{ ind.label }}</button>
        <button
          @click="smcModaleOuverte = ind.key"
          :class="[
            'px-1.5 py-1 rounded-r-md text-[11px] border-y border-r transition-all',
            (prefs as any)[ind.key] ? ind.gearClass : 'bg-white/5 border-white/10 text-slate-500 hover:text-slate-300'
          ]"
          title="Paramètres"
        >⚙</button>
      </div>

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

    </div>

    <IndicatorModal
      :indicateur="modaleOuverte"
      v-model="prefs"
      @fermer="modaleOuverte = null"
      @appliquer="validerModal"
    />
    <SmcIndicatorModal
      :indicateur="smcModaleOuverte"
      v-model="prefs"
      @fermer="smcModaleOuverte = null"
      @appliquer="validerSmcModal"
    />
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import type { PrefsIndicateurs } from '@/stores/settings.store'
import IndicatorModal from './IndicatorModal.vue'
import SmcIndicatorModal from './SmcIndicatorModal.vue'

const prefs = defineModel<PrefsIndicateurs>({ required: true })
const emit = defineEmits<{ appliquer: [] }>()

const modaleOuverte = ref<string | null>(null)
const smcModaleOuverte = ref<string | null>(null)

function toggle(key: string) {
  ;(prefs.value as any)[key] = !(prefs.value as any)[key]
  emit('appliquer')
}

function validerModal() {
  modaleOuverte.value = null
  emit('appliquer')
}

function validerSmcModal() {
  smcModaleOuverte.value = null
  emit('appliquer')
}

const techniques = [
  { key: 'ema',       label: 'EMA',       params: true,  activeClass: 'bg-amber-500/20 border-amber-500/40 text-amber-300',    gearClass: 'bg-amber-500/10 border-amber-500/40 text-amber-400 hover:bg-amber-500/20'   },
  { key: 'rsi',       label: 'RSI',       params: true,  activeClass: 'bg-purple-500/20 border-purple-500/40 text-purple-300', gearClass: 'bg-purple-500/10 border-purple-500/40 text-purple-400 hover:bg-purple-500/20' },
  { key: 'macd',      label: 'MACD',      params: true,  activeClass: 'bg-emerald-500/20 border-emerald-500/40 text-emerald-300', gearClass: 'bg-emerald-500/10 border-emerald-500/40 text-emerald-400 hover:bg-emerald-500/20' },
  { key: 'bollinger', label: 'Bollinger', params: true,  activeClass: 'bg-indigo-500/20 border-indigo-500/40 text-indigo-300',   gearClass: 'bg-indigo-500/10 border-indigo-500/40 text-indigo-400 hover:bg-indigo-500/20' },
  { key: 'atr',       label: 'ATR',       params: true,  activeClass: 'bg-rose-500/20 border-rose-500/40 text-rose-300',         gearClass: 'bg-rose-500/10 border-rose-500/40 text-rose-400 hover:bg-rose-500/20' },
]

const smcOptions = [
  { key: 'smcOb',         label: 'Order Blocks', activeClass: 'bg-emerald-500/20 border-emerald-500/40 text-emerald-300',   gearClass: 'bg-emerald-500/10 border-emerald-500/40 text-emerald-400 hover:bg-emerald-500/20' },
  { key: 'smcBpr',        label: 'IFVG/BPR',    activeClass: 'bg-sky-500/20 border-sky-500/40 text-sky-300',               gearClass: 'bg-sky-500/10 border-sky-500/40 text-sky-400 hover:bg-sky-500/20'               },
  { key: 'smcImbalance',   label: 'Imbalances',   activeClass: 'bg-violet-500/20 border-violet-500/40 text-violet-300',    gearClass: 'bg-violet-500/10 border-violet-500/40 text-violet-400 hover:bg-violet-500/20'  },
  { key: 'smcFib',        label: 'Fibonacci',    activeClass: 'bg-slate-400/20 border-slate-400/40 text-slate-300',        gearClass: 'bg-slate-400/10 border-slate-400/40 text-slate-400 hover:bg-slate-400/20'       },
  { key: 'smcLiquidites', label: 'Liquidité',    activeClass: 'bg-rose-500/20 border-rose-500/40 text-rose-300',           gearClass: 'bg-rose-500/10 border-rose-500/40 text-rose-400 hover:bg-rose-500/20'           },
]
</script>

