<template>
  <div class="space-y-4">
    <p class="text-[10px] text-slate-500">
      Niveaux de liquidité ciblés par les institutionnels : swings locaux,
      highs/lows de sessions et quotidiens.
    </p>

    <!-- Swings -->
    <div>
      <div class="flex items-center justify-between mb-2">
        <p class="text-[10px] font-semibold text-slate-400 uppercase tracking-wide">Swings H/L</p>
        <label class="flex items-center gap-1.5 cursor-pointer">
          <input type="checkbox" v-model="prefs.smcLiqSwingsActif" class="rounded accent-emerald-500" />
          <span class="text-[10px] text-slate-400">Actif</span>
        </label>
      </div>
      <div :class="{ 'opacity-40 pointer-events-none': !prefs.smcLiqSwingsActif }">
        <div class="grid grid-cols-2 gap-3 mb-2">
          <div>
            <label class="block text-xs text-slate-400 mb-1">High</label>
            <div class="flex items-center gap-2">
              <input type="color" v-model="prefs.smcLiqCouleurBsl"
                class="w-8 h-7 rounded cursor-pointer border border-white/15 bg-transparent" />
              <span class="text-[10px] text-slate-500 font-mono">{{ prefs.smcLiqCouleurBsl }}</span>
            </div>
          </div>
          <div>
            <label class="block text-xs text-slate-400 mb-1">Low</label>
            <div class="flex items-center gap-2">
              <input type="color" v-model="prefs.smcLiqCouleurSsl"
                class="w-8 h-7 rounded cursor-pointer border border-white/15 bg-transparent" />
              <span class="text-[10px] text-slate-500 font-mono">{{ prefs.smcLiqCouleurSsl }}</span>
            </div>
          </div>
        </div>
        <div class="flex items-center justify-between">
          <label class="text-xs text-slate-400">Lookback <span class="text-white">{{ prefs.smcLiqSwingLookback }}</span></label>
          <input type="range" min="3" max="50" step="1" v-model.number="prefs.smcLiqSwingLookback"
            class="w-28 accent-emerald-500" />
        </div>
      </div>
    </div>

    <!-- Sessions -->
    <div>
      <div class="flex items-center justify-between mb-2">
        <p class="text-[10px] font-semibold text-slate-400 uppercase tracking-wide">Sessions H/L</p>
        <label class="flex items-center gap-1.5 cursor-pointer">
          <input type="checkbox" v-model="prefs.smcLiqSessionsActif" class="rounded accent-sky-500" />
          <span class="text-[10px] text-slate-400">Actif</span>
        </label>
      </div>
      <div class="space-y-2" :class="{ 'opacity-40 pointer-events-none': !prefs.smcLiqSessionsActif }">
        <div v-for="sess in sessions" :key="sess.key" class="flex items-center justify-between gap-2">
          <label class="flex items-center gap-1.5 cursor-pointer min-w-0">
            <input type="checkbox" v-model="(prefs as any)[sess.checkKey]" class="rounded accent-slate-400 shrink-0" />
            <span class="text-xs text-slate-300 truncate">{{ sess.label }}</span>
          </label>
          <input type="color" v-model="(prefs as any)[sess.colorKey]"
            class="w-7 h-6 rounded cursor-pointer border border-white/15 bg-transparent shrink-0" />
        </div>
      </div>
    </div>

    <!-- Daily H/L -->
    <div>
      <div class="flex items-center justify-between mb-2">
        <p class="text-[10px] font-semibold text-slate-400 uppercase tracking-wide">Daily H/L</p>
        <label class="flex items-center gap-1.5 cursor-pointer">
          <input type="checkbox" v-model="prefs.smcLiqDwmActif" class="rounded accent-slate-400" />
          <span class="text-[10px] text-slate-400">Actif</span>
        </label>
      </div>
      <div class="flex items-center justify-between gap-3" :class="{ 'opacity-40 pointer-events-none': !prefs.smcLiqDwmActif }">
        <div class="flex items-center gap-2">
          <label class="text-xs text-slate-400 shrink-0">Couleur</label>
          <input type="color" v-model="prefs.smcLiqCouleurDwm"
            class="w-7 h-6 rounded cursor-pointer border border-white/15 bg-transparent" />
        </div>
        <div class="flex items-center gap-2">
          <label class="text-xs text-slate-400 shrink-0">Jours</label>
          <input type="number" min="1" max="5" step="1" v-model.number="prefs.smcLiqDwmNbJours"
            class="w-14 text-center text-xs bg-[#1a2035] border border-white/10 rounded-lg px-2 py-1 text-white focus:outline-none" />
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { PrefsIndicateurs } from '@/stores/settings.store'

const prefs = defineModel<PrefsIndicateurs>({ required: true })

const sessions = [
  { key: 'asie', label: 'Asie (22h–07h UTC)', checkKey: 'smcLiqSessionAsie', colorKey: 'smcLiqCouleurAsie' },
]
</script>
