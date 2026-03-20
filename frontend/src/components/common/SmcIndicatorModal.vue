<template>
  <Teleport to="body">
    <Transition name="modal">
      <div v-if="indicateur" class="fixed inset-0 z-50 flex items-center justify-center">
        <div class="absolute inset-0 bg-black/60 backdrop-blur-sm" @click="$emit('fermer')" />
        <div class="relative bg-[#0f1629] border border-white/10 rounded-2xl p-6 w-72 shadow-2xl">
          <h3 class="text-sm font-semibold text-white mb-4">{{ titreModale }}</h3>

          <!-- Order Blocks -->
          <template v-if="indicateur === 'smcOb'">
            <p class="text-[10px] text-slate-500 mb-3">
              Dernière bougie avant une impulsion forte. Zone de support/résistance institutionnelle.
            </p>
            <div class="grid grid-cols-2 gap-3 mb-3">
              <div>
                <label class="block text-xs text-slate-400 mb-1">Haussier</label>
                <div class="flex items-center gap-2">
                  <input type="color" v-model="prefs.smcObCouleurLong"
                    class="w-8 h-7 rounded cursor-pointer border border-white/15 bg-transparent" />
                  <span class="text-[10px] text-slate-500 font-mono">{{ prefs.smcObCouleurLong }}</span>
                </div>
              </div>
              <div>
                <label class="block text-xs text-slate-400 mb-1">Baissier</label>
                <div class="flex items-center gap-2">
                  <input type="color" v-model="prefs.smcObCouleurShort"
                    class="w-8 h-7 rounded cursor-pointer border border-white/15 bg-transparent" />
                  <span class="text-[10px] text-slate-500 font-mono">{{ prefs.smcObCouleurShort }}</span>
                </div>
              </div>
            </div>
            <label class="block text-xs text-slate-400 mb-1">Opacité <span class="text-white">{{ prefs.smcObOpacite }}</span></label>
            <input type="range" min="0.05" max="1" step="0.05" v-model.number="prefs.smcObOpacite"
              class="w-full accent-emerald-500" />
            <label class="block text-xs text-slate-400 mt-3 mb-1">Sensibilité <span class="text-white">{{ prefs.smcObSensibilite }}</span></label>
            <input type="range" min="1" max="100" step="1" v-model.number="prefs.smcObSensibilite"
              class="w-full accent-emerald-500" />
            <p class="text-[9px] text-slate-600 mt-0.5">Bas = plus d'OBs · Haut = moins d'OBs</p>
            <label class="block text-xs text-slate-400 mt-3 mb-1">Mitigation</label>
            <select v-model="prefs.smcObMitigationType"
              class="w-full border border-white/10 rounded-lg px-2 py-1.5 text-xs text-black bg-white focus:outline-none">
              <option value="close">Close (corps)</option>
              <option value="wick">Wick (mèche)</option>
            </select>
          </template>

          <!-- BPR -->
          <template v-else-if="indicateur === 'smcBpr'">
            <p class="text-[10px] text-slate-500 mb-3">
              <strong class="text-purple-400">IFVG</strong> (FVG inversé avec BOS confirmé) + <strong class="text-sky-400">BPR</strong> (overlap FVG haussier ✕ FVG baissier). Reproduit l'indicateur Kasper Bootcamp.
            </p>
            <div class="grid grid-cols-2 gap-3 mb-3">
              <div>
                <label class="block text-xs text-slate-400 mb-1">FVG haussier</label>
                <div class="flex items-center gap-2">
                  <input type="color" v-model="prefs.smcBprCouleurBull"
                    class="w-8 h-7 rounded cursor-pointer border border-white/15 bg-transparent" />
                  <span class="text-[10px] text-slate-500 font-mono">{{ prefs.smcBprCouleurBull }}</span>
                </div>
              </div>
              <div>
                <label class="block text-xs text-slate-400 mb-1">FVG baissier</label>
                <div class="flex items-center gap-2">
                  <input type="color" v-model="prefs.smcBprCouleurBear"
                    class="w-8 h-7 rounded cursor-pointer border border-white/15 bg-transparent" />
                  <span class="text-[10px] text-slate-500 font-mono">{{ prefs.smcBprCouleurBear }}</span>
                </div>
              </div>
            </div>
            <label class="block text-xs text-slate-400 mb-1">Opacité <span class="text-white">{{ prefs.smcBprOpacite }}</span></label>
            <input type="range" min="0.05" max="1" step="0.05" v-model.number="prefs.smcBprOpacite"
              class="w-full accent-sky-500 mb-3" />
            <div class="flex items-center justify-between mb-3">
              <label class="text-xs text-slate-400">Show Last</label>
              <input type="number" min="1" max="100" step="1" v-model.number="prefs.smcBprShowLast"
                class="w-16 text-center text-xs bg-[#1a2035] border border-white/10 rounded-lg px-2 py-1 text-white focus:outline-none" />
            </div>
            <div class="flex items-center justify-between mb-3">
              <label class="text-xs text-slate-400">ATR Multiplicateur</label>
              <input type="number" min="0.01" max="5" step="0.01" v-model.number="prefs.smcBprAtrMult"
                class="w-20 text-center text-xs bg-[#1a2035] border border-white/10 rounded-lg px-2 py-1 text-white focus:outline-none" />
            </div>
            <div class="flex items-center justify-between">
              <label class="text-xs text-slate-400">Fenêtre (bougies)</label>
              <input type="number" min="5" max="200" step="5" v-model.number="prefs.smcBprFenetre"
                class="w-20 text-center text-xs bg-[#1a2035] border border-white/10 rounded-lg px-2 py-1 text-white focus:outline-none" />
            </div>
            <div class="flex items-center justify-between mt-3">
              <label class="text-xs text-slate-400">Mitigation</label>
              <select v-model="prefs.smcBprMitigation"
                class="w-full border border-white/10 rounded-lg px-2 py-1.5 text-xs text-black bg-white focus:outline-none">
                <option value="close" style="color:#000;background:#fff">Clôture</option>
                <option value="wick" style="color:#000;background:#fff">Mèche</option>
              </select>
            </div>
          </template>

          <!-- IFVG -->
          <template v-else-if="indicateur === 'smcIfvg'">
            <p class="text-[10px] text-slate-500 mb-3">
              Inversion Fair Value Gap — FVG avec BOS (Break of Structure) confirmé dans la direction opposée.
            </p>
            <div class="grid grid-cols-2 gap-3 mb-3">
              <div>
                <label class="block text-xs text-slate-400 mb-1">IFVG haussier</label>
                <div class="flex items-center gap-2">
                  <input type="color" v-model="prefs.smcIfvgCouleurLong"
                    class="w-8 h-7 rounded cursor-pointer border border-white/15 bg-transparent" />
                  <span class="text-[10px] text-slate-500 font-mono">{{ prefs.smcIfvgCouleurLong }}</span>
                </div>
              </div>
              <div>
                <label class="block text-xs text-slate-400 mb-1">IFVG baissier</label>
                <div class="flex items-center gap-2">
                  <input type="color" v-model="prefs.smcIfvgCouleurShort"
                    class="w-8 h-7 rounded cursor-pointer border border-white/15 bg-transparent" />
                  <span class="text-[10px] text-slate-500 font-mono">{{ prefs.smcIfvgCouleurShort }}</span>
                </div>
              </div>
            </div>
            <label class="block text-xs text-slate-400 mb-1">Opacité <span class="text-white">{{ prefs.smcIfvgOpacite }}</span></label>
            <input type="range" min="0.05" max="1" step="0.05" v-model.number="prefs.smcIfvgOpacite"
              class="w-full accent-indigo-500 mb-3" />

            <!-- Show Last -->
            <div class="flex items-center justify-between mb-3">
              <label class="text-xs text-slate-400">Show Last</label>
              <input type="number" min="1" max="20" step="1" v-model.number="prefs.smcIfvgShowLast"
                class="w-16 text-center text-xs bg-[#1a2035] border border-white/10 rounded-lg px-2 py-1 text-white focus:outline-none" />
            </div>

            <!-- Signal Preference -->
            <label class="block text-xs text-slate-400 mb-1">Signal Preference</label>
            <select v-model="prefs.smcIfvgSignalPref"
              class="w-full border border-white/10 rounded-lg px-2 py-1.5 text-xs text-black bg-white focus:outline-none mb-3">
              <option value="close">Fermeture (close)</option>
              <option value="wick">Mèche (wick)</option>
            </select>

            <!-- ATR Multiplicateur -->
            <div class="flex items-center justify-between">
              <label class="text-xs text-slate-400">ATR Multiplicateur</label>
              <input type="number" min="0.01" max="5" step="0.01" v-model.number="prefs.smcIfvgAtrMult"
                class="w-20 text-center text-xs bg-[#1a2035] border border-white/10 rounded-lg px-2 py-1 text-white focus:outline-none" />
            </div>
          </template>

          <!-- Imbalances (FVG + OG) -->
          <template v-else-if="indicateur === 'smcImbalance'">
            <SmcImbalanceParams v-model="prefs" />
          </template>

          <!-- Fibonacci -->
          <template v-else-if="indicateur === 'smcFib'">
            <SmcFibonacciParams v-model="prefs" />
          </template>

          <!-- Liquidité (Swings + Sessions + Daily) -->
          <template v-else-if="indicateur === 'smcLiquidites'">
            <SmcLiquiditeParams v-model="prefs" />
          </template>

          <div class="flex justify-end gap-2 mt-5">
            <button
              @click="$emit('fermer')"
              class="px-4 py-1.5 rounded-lg text-xs text-slate-400 hover:text-white border border-white/10 hover:border-white/20 transition-colors"
            >Annuler</button>
            <button
              @click="$emit('appliquer')"
              class="px-4 py-1.5 rounded-lg text-xs font-medium bg-blue-600 hover:bg-blue-500 text-white transition-colors"
            >Appliquer</button>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import type { PrefsIndicateurs } from '@/stores/settings.store'
import SmcImbalanceParams from './SmcImbalanceParams.vue'
import SmcLiquiditeParams from './SmcLiquiditeParams.vue'
import SmcFibonacciParams from './SmcFibonacciParams.vue'

const props = defineProps<{ indicateur: string | null }>()
const prefs = defineModel<PrefsIndicateurs>({ required: true })
defineEmits<{ fermer: []; appliquer: [] }>()

const TITRES: Record<string, string> = {
  smcOb:          'Order Blocks — Paramètres',
  smcBpr:         'IFVG / BPR — Paramètres',
  smcIfvg:        'IFVG (Inversion FVG) — Paramètres',
  smcImbalance:   'Imbalances (FVG + OG) — Paramètres',
  smcFib:         'Fibonacci — Paramètres',
  smcLiquidites: 'Liquidité — Paramètres',
}

const titreModale = computed(() =>
  props.indicateur ? (TITRES[props.indicateur] ?? props.indicateur) : ''
)
</script>

<style scoped>
.modal-enter-active,
.modal-leave-active {
  transition: opacity 0.15s ease;
}
.modal-enter-from,
.modal-leave-to {
  opacity: 0;
}
</style>
