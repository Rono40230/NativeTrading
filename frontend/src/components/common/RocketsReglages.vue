<template>
  <div class="space-y-6 p-2">

    <!-- Chargement -->
    <div v-if="chargement" class="text-gray-400 text-sm">Chargement de la configuration…</div>

    <template v-else-if="cfg">
      <!-- Score minimum -->
      <div class="glass-param">
        <div class="flex items-center justify-between mb-2">
          <div class="flex items-baseline gap-2">
            <label class="text-sm font-semibold text-white">Score minimum</label>
            <span class="text-xs text-gray-500">Seuls les signaux avec un score ≥ {{ cfg.score_min }} sont sauvegardés.</span>
          </div>
          <span class="text-emerald-400 font-mono font-bold">{{ cfg.score_min }}</span>
        </div>
        <input type="range" min="0" max="100" step="5" v-model.number="cfg.score_min" class="w-full accent-emerald-400" />
      </div>

      <!-- RSI max -->
      <div class="glass-param">
        <div class="flex items-center justify-between mb-2">
          <div class="flex items-baseline gap-2">
            <label class="text-sm font-semibold text-white">RSI maximum</label>
            <span class="text-xs text-gray-500">Filtrer les signaux en zone de surachat extrême (RSI > {{ cfg.rsi_max }}).</span>
          </div>
          <span class="text-emerald-400 font-mono font-bold">{{ cfg.rsi_max }}</span>
        </div>
        <input type="range" min="50" max="100" step="1" v-model.number="cfg.rsi_max" class="w-full accent-emerald-400" />
      </div>

      <!-- RSI min -->
      <div class="glass-param">
        <div class="flex items-center justify-between mb-2">
          <div class="flex items-baseline gap-2">
            <label class="text-sm font-semibold text-white">RSI minimum</label>
            <span class="text-xs text-gray-500">Filtrer les signaux en zone de survente (RSI < {{ cfg.rsi_min }}). 0 = désactivé.</span>
          </div>
          <span class="text-emerald-400 font-mono font-bold">{{ cfg.rsi_min }}</span>
        </div>
        <input type="range" min="0" max="50" step="1" v-model.number="cfg.rsi_min" class="w-full accent-emerald-400" />
      </div>

      <!-- Ratio volume minimum -->
      <div class="glass-param">
        <div class="flex items-center justify-between mb-2">
          <div class="flex items-baseline gap-2">
            <label class="text-sm font-semibold text-white">Ratio volume minimum</label>
            <span class="text-xs text-gray-500">Volume actuel ≥ {{ cfg.ratio_volume_min }}× la moyenne des 20 dernières bougies.</span>
          </div>
          <span class="text-emerald-400 font-mono font-bold">{{ cfg.ratio_volume_min }}×</span>
        </div>
        <input type="range" min="1.0" max="5.0" step="0.1" v-model.number="cfg.ratio_volume_min" class="w-full accent-emerald-400" />
      </div>

      <!-- Volume marché minimum -->
      <div class="glass-param">
        <div class="flex items-center justify-between mb-2">
          <div class="flex items-baseline gap-2">
            <label class="text-sm font-semibold text-white">Volume marché minimum (USDT/j)</label>
            <span class="text-xs text-gray-500">Ignorer les paires avec moins de {{ formatVol(cfg.vol_marche_min) }} de volume journalier.</span>
          </div>
          <span class="text-emerald-400 font-mono font-bold">{{ formatVol(cfg.vol_marche_min) }}</span>
        </div>
        <input type="range" min="100000" max="5000000" step="100000" v-model.number="cfg.vol_marche_min" class="w-full accent-emerald-400" />
      </div>

      <!-- Phases actives -->
      <div class="glass-param">
        <div class="flex items-baseline gap-2 mb-3">
          <label class="text-sm font-semibold text-white">Phases actives</label>
          <span class="text-xs text-gray-500">Seules les phases cochées génèrent des signaux sauvegardés.</span>
        </div>
        <div class="flex gap-3">
          <label v-for="p in PHASES" :key="p.val" class="flex items-center gap-2 cursor-pointer select-none">
            <input
              type="checkbox"
              :checked="cfg.phases_actives.includes(p.val)"
              @change="togglePhase(p.val)"
              class="w-4 h-4 accent-emerald-400"
            />
            <span class="text-sm" :class="cfg.phases_actives.includes(p.val) ? 'text-white' : 'text-gray-500'">{{ p.label }}</span>
          </label>
        </div>
      </div>

      <!-- Actions -->
      <div class="flex items-center gap-3 pt-2">
        <button
          class="px-5 py-2 rounded-lg bg-emerald-600 hover:bg-emerald-500 text-white text-sm font-semibold transition-all"
          :disabled="sauvegarde"
          @click="sauvegarder"
        >{{ sauvegarde ? '⏳ Sauvegarde…' : '💾 Appliquer les réglages' }}</button>
        <button
          class="px-4 py-2 rounded-lg border border-white/15 text-gray-400 hover:text-white text-sm transition-all"
          @click="reinitialiser"
        >↩ Réinitialiser</button>
        <span v-if="message" class="text-sm" :class="messageOk ? 'text-emerald-400' : 'text-red-400'">{{ message }}</span>
      </div>
    </template>

  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { apiService } from '@/services/api.service'
import type { RocketsConfig } from '@/services/api.types'

const PHASES = [
  { val: 'breakout',     label: '⚡ Breakout' },
  { val: 'prelancement', label: '🔶 Pré-lancement' },
]

const cfg = ref<RocketsConfig | null>(null)
const original = ref<RocketsConfig | null>(null)
const chargement = ref(true)
const sauvegarde = ref(false)
const message = ref('')
const messageOk = ref(true)

onMounted(async () => {
  try {
    const data = await apiService.getRocketsConfig()
    cfg.value = { ...data }
    original.value = { ...data }
  } catch {
    message.value = 'Erreur chargement configuration'
    messageOk.value = false
  } finally {
    chargement.value = false
  }
})

function togglePhase(val: string) {
  if (!cfg.value) return
  const idx = cfg.value.phases_actives.indexOf(val)
  if (idx >= 0) {
    cfg.value.phases_actives = cfg.value.phases_actives.filter(p => p !== val)
  } else {
    cfg.value.phases_actives = [...cfg.value.phases_actives, val]
  }
}

async function sauvegarder() {
  if (!cfg.value) return
  sauvegarde.value = true
  message.value = ''
  try {
    await apiService.putRocketsConfig(cfg.value)
    original.value = { ...cfg.value }
    message.value = '✅ Réglages appliqués — actifs au prochain scan'
    messageOk.value = true
  } catch {
    message.value = '❌ Erreur lors de la sauvegarde'
    messageOk.value = false
  } finally {
    sauvegarde.value = false
  }
}

function reinitialiser() {
  if (original.value) cfg.value = { ...original.value }
  message.value = ''
}

function formatVol(v: number): string {
  if (v >= 1_000_000) return `${(v / 1_000_000).toFixed(1)}M`
  if (v >= 1_000) return `${(v / 1_000).toFixed(0)}k`
  return String(v)
}
</script>

<style scoped>
.glass-param {
  @apply rounded-lg border border-white/10 bg-white/5 p-4;
}
</style>
