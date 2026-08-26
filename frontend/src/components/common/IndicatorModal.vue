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
            <label class="block text-xs text-slate-400 mb-1 mt-3">Type de MA</label>
            <select
              v-model="prefs.emaMaType"
              class="w-full bg-white border border-white/15 rounded-lg px-3 py-2 text-sm text-black outline-none focus:border-amber-500/60 transition-colors"
            >
              <option value="ema">MME (EMA)</option>
              <option value="sma">SMA</option>
            </select>
            <label class="block text-xs text-slate-400 mb-1 mt-3">Couleur</label>
            <div class="flex items-center gap-3">
              <input
                type="color"
                v-model="prefs.emaCouleur"
                class="w-10 h-8 rounded cursor-pointer border border-white/15 bg-transparent"
              />
              <span class="text-xs text-slate-400 font-mono">{{ prefs.emaCouleur }}</span>
            </div>
          </template>

          <!-- ATR -->
          <template v-else-if="indicateur === 'atr'">
            <label class="block text-xs text-slate-400 mb-1">Période</label>
            <input type="number" min="1" max="200" v-model.number="prefs.atrPeriode"
              class="w-full bg-white/10 border border-white/15 rounded-lg px-3 py-2 text-sm text-white outline-none focus:border-rose-500/60 transition-colors" />
            <p class="text-[10px] text-slate-500 mt-1">Standard : 14 (Wilder)</p>
            <label class="block text-xs text-slate-400 mb-1 mt-3">Couleur</label>
            <div class="flex items-center gap-3">
              <input type="color" v-model="prefs.atrCouleur"
                class="w-10 h-8 rounded cursor-pointer border border-white/15 bg-transparent" />
              <span class="text-xs text-slate-400 font-mono">{{ prefs.atrCouleur }}</span>
            </div>
          </template>

          <!-- Bollinger Bands -->
          <template v-else-if="indicateur === 'bollinger'">
            <div class="flex gap-2">
              <div class="flex-1">
                <label class="block text-xs text-slate-400 mb-1">Période</label>
                <input type="number" min="2" max="500" v-model.number="prefs.bollingerPeriode"
                  class="w-full bg-white/10 border border-white/15 rounded-lg px-3 py-2 text-sm text-white outline-none focus:border-indigo-500/60 transition-colors" />
              </div>
              <div class="flex-1">
                <label class="block text-xs text-slate-400 mb-1">Multiplicateur σ</label>
                <input type="number" min="0.1" max="5" step="0.1" v-model.number="prefs.bollingerStdDev"
                  class="w-full bg-white/10 border border-white/15 rounded-lg px-3 py-2 text-sm text-white outline-none focus:border-indigo-500/60 transition-colors" />
              </div>
            </div>
            <label class="block text-xs text-slate-400 mb-1 mt-3">Type de MA (base)</label>
            <select
              v-model="prefs.bollingerMaType"
              class="w-full bg-white border border-white/15 rounded-lg px-3 py-2 text-sm text-black outline-none focus:border-indigo-500/60 transition-colors"
            >
              <option value="sma">SMA (défaut)</option>
              <option value="ema">MME (EMA)</option>
            </select>
            <div class="grid grid-cols-3 gap-2 mt-3">
              <div>
                <label class="block text-[10px] text-slate-400 mb-1">Bande haute</label>
                <div class="flex items-center gap-1.5">
                  <input type="color" v-model="prefs.bollingerCouleurHaute"
                    class="w-8 h-7 rounded cursor-pointer border border-white/15 bg-transparent" />
                  <span class="text-[10px] text-slate-500 font-mono truncate">{{ prefs.bollingerCouleurHaute }}</span>
                </div>
              </div>
              <div>
                <label class="block text-[10px] text-slate-400 mb-1">Basis</label>
                <div class="flex items-center gap-1.5">
                  <input type="color" v-model="prefs.bollingerCouleurMilieu"
                    class="w-8 h-7 rounded cursor-pointer border border-white/15 bg-transparent" />
                  <span class="text-[10px] text-slate-500 font-mono truncate">{{ prefs.bollingerCouleurMilieu }}</span>
                </div>
              </div>
              <div>
                <label class="block text-[10px] text-slate-400 mb-1">Bande basse</label>
                <div class="flex items-center gap-1.5">
                  <input type="color" v-model="prefs.bollingerCouleurBasse"
                    class="w-8 h-7 rounded cursor-pointer border border-white/15 bg-transparent" />
                  <span class="text-[10px] text-slate-500 font-mono truncate">{{ prefs.bollingerCouleurBasse }}</span>
                </div>
              </div>
            </div>
            <p class="text-[10px] text-slate-500 mt-2">Standard : 20 / 2.0 / SMA</p>
          </template>

          <!-- MACD -->
          <template v-else-if="indicateur === 'macd'">
            <div class="flex gap-2">
              <div class="flex-1">
                <label class="block text-xs text-slate-400 mb-1">Rapide</label>
                <input type="number" min="2" max="100" v-model.number="prefs.macdRapide"
                  class="w-full bg-white/10 border border-white/15 rounded-lg px-3 py-2 text-sm text-white outline-none focus:border-emerald-500/60 transition-colors" />
              </div>
              <div class="flex-1">
                <label class="block text-xs text-slate-400 mb-1">Lente</label>
                <input type="number" min="2" max="200" v-model.number="prefs.macdLente"
                  class="w-full bg-white/10 border border-white/15 rounded-lg px-3 py-2 text-sm text-white outline-none focus:border-emerald-500/60 transition-colors" />
              </div>
            </div>
            <label class="block text-xs text-slate-400 mb-1 mt-3">Signal</label>
            <input type="number" min="2" max="50" v-model.number="prefs.macdSignal"
              class="w-full bg-white/10 border border-white/15 rounded-lg px-3 py-2 text-sm text-white outline-none focus:border-emerald-500/60 transition-colors" />
            <p class="text-[10px] text-slate-500 mt-1">Standard : 12 / 26 / 9</p>
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
            <div class="flex gap-2 mt-3">
              <div class="flex-1">
                <label class="block text-xs text-slate-400 mb-1">Surachat</label>
                <input
                  type="number" min="50" max="100"
                  v-model.number="prefs.rsiSurachat"
                  class="w-full bg-white/10 border border-white/15 rounded-lg px-3 py-2 text-sm text-white outline-none focus:border-red-500/60 transition-colors"
                />
              </div>
              <div class="flex-1">
                <label class="block text-xs text-slate-400 mb-1">Survente</label>
                <input
                  type="number" min="0" max="50"
                  v-model.number="prefs.rsiSurvente"
                  class="w-full bg-white/10 border border-white/15 rounded-lg px-3 py-2 text-sm text-white outline-none focus:border-emerald-500/60 transition-colors"
                />
              </div>
            </div>
            <label class="block text-xs text-slate-400 mb-1 mt-3">Couleur</label>
            <div class="flex items-center gap-3">
              <input
                type="color"
                v-model="prefs.rsiCouleur"
                class="w-10 h-8 rounded cursor-pointer border border-white/15 bg-transparent"
              />
              <span class="text-xs text-slate-400 font-mono">{{ prefs.rsiCouleur }}</span>
            </div>
          </template>

          <!-- Kasper Tendance (EMA Crossover MTF) -->
          <template v-else-if="indicateur === 'kasperTendance'">
            <label class="block text-xs text-slate-400 mb-1">Période EMA rapide</label>
            <input
              type="number" min="1" max="200"
              v-model.number="prefs.kasperPeriodeRapide"
              class="w-full bg-white/10 border border-white/15 rounded-lg px-3 py-2 text-sm text-white outline-none focus:border-sky-500/60 transition-colors"
            />
            <label class="block text-xs text-slate-400 mb-1 mt-3">Période EMA lente</label>
            <input
              type="number" min="2" max="500"
              v-model.number="prefs.kasperPeriodeLente"
              class="w-full bg-white/10 border border-white/15 rounded-lg px-3 py-2 text-sm text-white outline-none focus:border-sky-500/60 transition-colors"
            />
            <label class="block text-xs text-slate-400 mb-1 mt-3">Mode de calcul</label>
            <select
              v-model="prefs.kasperModeCalcul"
              class="w-full bg-white border border-white/15 rounded-lg px-3 py-2 text-sm text-black outline-none focus:border-sky-500/60 transition-colors"
            >
              <option value="bougie_cloturee">Bougie cloturee</option>
              <option value="bougie_en_cours">Bougie en cours</option>
            </select>
            <p class="text-[10px] text-slate-500 mt-2">Défauts : EMA 9 (rapide) / EMA 21 (lente)</p>
          </template>

          <!-- Fibonacci (retracement auto sur le dernier swing) -->
          <template v-else-if="indicateur === 'fibonacci'">
            <p class="text-[10px] text-slate-500 mb-2">Retracement calculé sur le dernier swing (hauteur 100 % → profondeur 0 %).</p>
            <label class="block text-xs text-slate-400 mb-1">Niveaux affichés</label>
            <div class="space-y-1.5">
              <label class="flex items-center gap-2 text-xs text-slate-300 cursor-pointer">
                <input type="checkbox" v-model="prefs.fibNiveau500" class="accent-slate-400 w-3.5 h-3.5" />
                0.500
              </label>
              <label class="flex items-center gap-2 text-xs text-slate-300 cursor-pointer">
                <input type="checkbox" v-model="prefs.fibNiveau618" class="accent-slate-400 w-3.5 h-3.5" />
                0.618
              </label>
              <label class="flex items-center gap-2 text-xs text-slate-300 cursor-pointer">
                <input type="checkbox" v-model="prefs.fibNiveau786" class="accent-slate-400 w-3.5 h-3.5" />
                0.786
              </label>
              <label class="flex items-center gap-2 text-xs text-slate-300 cursor-pointer">
                <input type="checkbox" v-model="prefs.fibSwings" class="accent-slate-400 w-3.5 h-3.5" />
                Swings 0 % / 100 %
              </label>
            </div>
            <label class="block text-xs text-slate-400 mb-1 mt-3">Couleur</label>
            <div class="flex items-center gap-3">
              <input
                type="color"
                v-model="prefs.fibCouleur"
                class="w-10 h-8 rounded cursor-pointer border border-white/15 bg-transparent"
              />
              <span class="text-xs text-slate-400 font-mono">{{ prefs.fibCouleur }}</span>
            </div>
            <p class="text-[10px] text-slate-500 mt-2">La zone 0.618 → 0.786 correspond à l'OTE (cf. dropdown SMC).</p>
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
  macd: 'Paramètres MACD',
  atr: 'ATR — Average True Range',
  bollinger: 'Bollinger Bands',
  fibonacci: 'Fibonacci — retracement auto',
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
