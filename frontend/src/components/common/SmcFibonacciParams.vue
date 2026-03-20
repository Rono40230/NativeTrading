<template>
  <div class="space-y-4">

    <!-- Sens -->
    <section>
      <p class="text-[11px] font-semibold text-slate-300 uppercase tracking-widest mb-2">Sens du retracement</p>
      <div class="grid grid-cols-2 gap-2">
        <button
          :class="prefs.smcFibSensHaussier
            ? 'bg-emerald-500/20 border-emerald-500/50 text-emerald-300'
            : 'bg-white/5 border-white/10 text-slate-400 hover:text-slate-200'"
          class="rounded-lg py-2 text-xs font-medium border transition-colors"
          @click="prefs.smcFibSensHaussier = true"
        >↑ Haussier (0 en bas)</button>
        <button
          :class="!prefs.smcFibSensHaussier
            ? 'bg-rose-500/20 border-rose-500/50 text-rose-300'
            : 'bg-white/5 border-white/10 text-slate-400 hover:text-slate-200'"
          class="rounded-lg py-2 text-xs font-medium border transition-colors"
          @click="prefs.smcFibSensHaussier = false"
        >↓ Baissier (0 en haut)</button>
      </div>
    </section>

    <div class="border-t border-white/8" />

    <!-- Couleurs par niveau -->
    <section>
      <p class="text-[11px] font-semibold text-slate-300 uppercase tracking-widest mb-2">Couleur par niveau</p>
      <div class="space-y-1.5">
        <div v-for="niv in niveaux" :key="niv.key"
          class="flex items-center justify-between bg-white/5 rounded-lg px-3 py-2">
          <span class="text-xs text-slate-300 font-mono w-14">{{ niv.label }}</span>
          <div class="flex items-center gap-2">
            <input type="color" v-model="(prefs as any)[niv.key]"
              class="w-8 h-6 rounded cursor-pointer border border-white/15 bg-transparent" />
            <span class="text-[10px] text-slate-500 font-mono">{{ (prefs as any)[niv.key] }}</span>
          </div>
        </div>
      </div>
    </section>

    <div class="border-t border-white/8" />

    <!-- Golden Zone -->
    <section>
      <div class="flex items-center justify-between mb-2">
        <p class="text-[11px] font-semibold text-slate-300 uppercase tracking-widest">Golden Zone (50%→61.8%)</p>
        <label class="flex items-center gap-1.5 cursor-pointer">
          <input type="checkbox" v-model="prefs.smcFibGoldenZone" class="rounded accent-amber-500" />
          <span class="text-[10px] text-slate-400">Actif</span>
        </label>
      </div>
      <div class="grid grid-cols-2 gap-2" :class="{ 'opacity-40 pointer-events-none': !prefs.smcFibGoldenZone }">
        <div class="bg-white/5 rounded-lg px-3 py-2">
          <p class="text-[10px] text-slate-500 mb-1.5">Couleur</p>
          <div class="flex items-center gap-2">
            <input type="color" v-model="prefs.smcFibGoldenCouleur"
              class="w-8 h-7 rounded cursor-pointer border border-white/15 bg-transparent" />
            <span class="text-[10px] text-slate-400 font-mono">{{ prefs.smcFibGoldenCouleur }}</span>
          </div>
        </div>
        <div class="bg-white/5 rounded-lg px-3 py-2">
          <p class="text-[10px] text-slate-500 mb-1.5">Opacité</p>
          <div class="flex items-center gap-2">
            <input type="range" min="0.05" max="0.6" step="0.05" v-model.number="prefs.smcFibGoldenOpacite"
              class="flex-1 accent-amber-500" />
            <span class="text-[10px] text-slate-400 w-8 text-right">{{ Math.round(prefs.smcFibGoldenOpacite * 100) }}%</span>
          </div>
        </div>
      </div>
    </section>

  </div>
</template>

<script setup lang="ts">
import type { PrefsIndicateurs } from '@/stores/settings.store'

const prefs = defineModel<PrefsIndicateurs>({ required: true })

const niveaux = [
  { key: 'smcFibCouleur0',   label: '0%' },
  { key: 'smcFibCouleur500', label: '50%' },
  { key: 'smcFibCouleur618', label: '61.8%' },
  { key: 'smcFibCouleur786', label: '78.6%' },
  { key: 'smcFibCouleur1',   label: '100%' },
]
</script>
