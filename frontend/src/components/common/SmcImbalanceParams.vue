<template>
  <div>
    <p class="text-[10px] text-slate-500 mb-3">
      <strong class="text-violet-400">FVG</strong> (Fair Value Gap) : déséquilibre entre 3 bougies consécutives.<br />
      <strong class="text-violet-300">OG</strong> (Opening Gap) : gap d'ouverture prioritaire sur FVG.
    </p>
    <div class="grid grid-cols-2 gap-3 mb-4">
      <div>
        <label class="block text-xs text-slate-400 mb-1">Couleur haussière</label>
        <div class="flex items-center gap-2">
          <input type="color" v-model="prefs.smcImbCouleurBull"
            class="w-10 h-8 rounded cursor-pointer border border-white/15 bg-transparent" />
          <span class="text-[10px] text-slate-500 font-mono">{{ prefs.smcImbCouleurBull }}</span>
        </div>
      </div>
      <div>
        <label class="block text-xs text-slate-400 mb-1">Couleur baissière</label>
        <div class="flex items-center gap-2">
          <input type="color" v-model="prefs.smcImbCouleurBear"
            class="w-10 h-8 rounded cursor-pointer border border-white/15 bg-transparent" />
          <span class="text-[10px] text-slate-500 font-mono">{{ prefs.smcImbCouleurBear }}</span>
        </div>
      </div>
    </div>
    <label class="block text-xs text-slate-400 mb-1">Opacité <span class="text-white">{{ prefs.smcImbOpacite }}</span></label>
    <input type="range" min="0.05" max="1" step="0.05" v-model.number="prefs.smcImbOpacite"
      class="w-full accent-violet-500 mb-4" />
    <div class="grid grid-cols-2 gap-3 mb-3">
      <div>
        <label class="block text-xs text-slate-400 mb-1">Afficher les N derniers</label>
        <input type="number" min="1" max="50" step="1" v-model.number="prefs.smcImbShowLast"
          class="w-full bg-white/5 border border-white/10 rounded px-2 py-1 text-xs text-white" />
      </div>
      <div>
        <label class="block text-xs text-slate-400 mb-1">Mitigation</label>
        <select v-model="prefs.smcImbMitigation"
          class="w-full border border-white/10 rounded-lg px-2 py-1.5 text-xs text-black bg-white focus:outline-none">
          <option value="close">Close</option>
          <option value="wick">Wick</option>
        </select>
      </div>
    </div>
    <div class="space-y-2">
      <label class="flex items-center gap-2 cursor-pointer">
        <input type="checkbox" v-model="prefs.smcImbShowFvg" class="rounded accent-violet-500" />
        <span class="text-xs text-slate-300">Afficher FVG (Fair Value Gap)</span>
      </label>
      <label class="flex items-center gap-2 cursor-pointer">
        <input type="checkbox" v-model="prefs.smcImbShowOg" class="rounded accent-violet-500" />
        <span class="text-xs text-slate-300">Afficher OG (Opening Gap) — prioritaire</span>
      </label>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { PrefsIndicateurs } from '@/stores/settings.store'
const prefs = defineModel<PrefsIndicateurs>({ required: true })
</script>
