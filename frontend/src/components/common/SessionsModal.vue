<template>
  <div>
    <p class="text-[10px] text-slate-500 mb-3">
      Bandes colorées sur le graphique aux heures d'ouverture/fermeture de chaque bourse.<br />
      <span class="text-amber-400/70">Les horaires sont en heure locale de chaque place — DST géré automatiquement.</span>
    </p>

    <!-- Opacité + Labels -->
    <div class="grid grid-cols-2 gap-3 mb-4">
      <div>
        <label class="block text-xs text-slate-400 mb-1">Opacité <span class="text-white">{{ prefs.sessionsOpacite }}</span></label>
        <input type="range" min="0.02" max="0.3" step="0.01" v-model.number="prefs.sessionsOpacite"
          class="w-full accent-cyan-500" />
      </div>
      <div class="flex items-center gap-2 mt-4">
        <input type="checkbox" v-model="prefs.sessionsLabels" class="rounded accent-cyan-500" />
        <span class="text-xs text-slate-300">Afficher les labels</span>
      </div>
    </div>

    <!-- Sessions -->
    <div class="space-y-2">
      <div v-for="s in SESSIONS" :key="s.key" class="flex items-center gap-3">
        <input type="checkbox" v-model="(prefs as any)[s.key]" class="rounded accent-cyan-500" />
        <input type="color" :value="(prefs as any)[s.couleurKey]"
          @input="(prefs as any)[s.couleurKey] = ($event.target as HTMLInputElement).value"
          class="w-8 h-6 rounded cursor-pointer border border-white/15 bg-transparent" />
        <span class="text-xs text-slate-300 w-20">{{ s.nom }}</span>
        <span class="text-[10px] text-slate-500">{{ s.plage }}</span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { PrefsIndicateurs } from '@/stores/settings.store'

const prefs = defineModel<PrefsIndicateurs>({ required: true })

const SESSIONS = [
  { key: 'sessionsSydney',   couleurKey: 'sessionsCouleurSydney',   nom: 'Sydney',    plage: '10:00 – 16:00 AEDT' },
  { key: 'sessionsTokyo',    couleurKey: 'sessionsCouleurTokyo',    nom: 'Tokyo',     plage: '09:00 – 18:00 JST'  },
  { key: 'sessionsHongKong', couleurKey: 'sessionsCouleurHongKong', nom: 'Hong Kong', plage: '09:30 – 16:00 HKT'  },
  { key: 'sessionsLondres',  couleurKey: 'sessionsCouleurLondres',  nom: 'Londres',   plage: '08:00 – 16:30 GMT/BST' },
  { key: 'sessionsNewYork',  couleurKey: 'sessionsCouleurNewYork',  nom: 'New York',  plage: '09:30 – 16:00 EST/EDT' },
]
</script>
