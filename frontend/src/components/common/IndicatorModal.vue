<template>
  <Teleport to="body">
    <Transition name="modal">
      <div
        v-if="indicateur"
        class="fixed inset-0 z-50 flex items-center justify-center"
      >
        <div class="absolute inset-0 bg-black/60 backdrop-blur-sm" @click="$emit('fermer')" />
        <div class="relative bg-[#0f1629] border border-white/10 rounded-2xl p-6 w-72 shadow-2xl">
          <h3 class="text-sm font-semibold text-white mb-4">{{ titreModale }}</h3>

          <!-- EMA -->
          <template v-if="indicateur === 'ema'">
            <label class="block text-xs text-slate-400 mb-1">Période</label>
            <input
              type="number" min="2" max="500"
              v-model.number="prefs.emaPeriode"
              class="w-full bg-white/10 border border-white/15 rounded-lg px-3 py-2 text-sm text-white outline-none focus:border-amber-500/60 transition-colors"
            />
            <p class="text-[10px] text-slate-500 mt-1">Recommandé : 9, 20, 50, 200</p>
          </template>

          <!-- RSI -->
          <template v-else-if="indicateur === 'rsi'">
            <label class="block text-xs text-slate-400 mb-1">Période</label>
            <input
              type="number" min="2" max="100"
              v-model.number="prefs.rsiPeriode"
              class="w-full bg-white/10 border border-white/15 rounded-lg px-3 py-2 text-sm text-white outline-none focus:border-purple-500/60 transition-colors"
            />
            <p class="text-[10px] text-slate-500 mt-1">Recommandé : 14 (standard), 7 (scalping)</p>
          </template>

          <!-- Kasper Tendance -->
          <template v-else-if="indicateur === 'kasperTendance'">
            <label class="block text-xs text-slate-400 mb-1">MM Rapide (période)</label>
            <input
              type="number" min="1" max="200"
              v-model.number="prefs.kasperMmRapide"
              class="w-full bg-white/10 border border-white/15 rounded-lg px-3 py-2 text-sm text-white outline-none focus:border-sky-500/60 transition-colors"
            />
            <label class="block text-xs text-slate-400 mb-1 mt-3">MM Lente (période)</label>
            <input
              type="number" min="1" max="500"
              v-model.number="prefs.kasperMmLente"
              class="w-full bg-white/10 border border-white/15 rounded-lg px-3 py-2 text-sm text-white outline-none focus:border-sky-500/60 transition-colors"
            />
            <label class="block text-xs text-slate-400 mb-1 mt-3">Type de MA</label>
            <select
              v-model="prefs.kasperMaType"
              class="w-full bg-white border border-white/15 rounded-lg px-3 py-2 text-sm text-black outline-none focus:border-sky-500/60 transition-colors"
            >
              <option value="ema">MME (EMA)</option>
              <option value="sma">SMA</option>
            </select>
            <p class="text-[10px] text-slate-500 mt-1">Défaults Kasper Bootcamp : 9 / 21 / MME</p>
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

const props = defineProps<{ indicateur: string | null }>()
const prefs = defineModel<PrefsIndicateurs>({ required: true })
defineEmits<{ fermer: []; appliquer: [] }>()

const TITRES: Record<string, string> = {
  ema: 'Paramètres EMA',
  rsi: 'Paramètres RSI',
  kasperTendance: 'Tendance Kasper Bootcamp',
}

const titreModale = computed(() =>
  props.indicateur ? (TITRES[props.indicateur] ?? props.indicateur.toUpperCase()) : ''
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
