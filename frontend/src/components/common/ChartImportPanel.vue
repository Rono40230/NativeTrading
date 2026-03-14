<template>
  <div class="space-y-4">
    <!-- Zone drag & drop -->
    <div
      class="rounded-xl border-2 border-dashed transition-colors cursor-pointer relative min-h-[130px]"
      :class="[
        dragActif
          ? 'border-blue-400 bg-blue-500/8'
          : imagePreview
            ? 'border-emerald-500/40 bg-emerald-900/5'
            : 'border-white/20 hover:border-white/40',
      ]"
      @dragover.prevent="setDragActif(true)"
      @dragleave.prevent="setDragActif(false)"
      @drop.prevent="onDrop"
      @click="fileInputEl?.click()"
    >
      <input ref="fileInputEl" type="file" accept="image/*" class="hidden" @change="onInputFile" />

      <div v-if="!imagePreview" class="flex flex-col items-center justify-center gap-2 py-10 pointer-events-none">
        <span class="text-3xl">📊</span>
        <p class="text-sm text-gray-400">Glissez un screenshot de chart ici</p>
        <p class="text-xs text-gray-600">PNG, JPG, WebP — ou cliquez pour sélectionner</p>
      </div>

      <div v-else class="relative" @click.stop>
        <img :src="imagePreview" alt="Chart importé" class="w-full max-h-72 object-contain rounded-xl p-2" />
        <button
          class="absolute top-2 right-2 bg-black/70 hover:bg-black/90 text-white rounded-full w-7 h-7 text-xs flex items-center justify-center"
          @click="reinitialiser"
        >✕</button>
      </div>
    </div>

    <!-- Notes contextuelles -->
    <textarea
      v-model="notes"
      rows="2"
      placeholder="Notes contextuelles optionnelles — ex : OB visible à 1.0850, BOS haussier confirmé…"
      class="w-full bg-gray-800 border border-gray-600 text-white text-sm rounded-lg px-3 py-2 resize-none placeholder:text-gray-600 focus:outline-none focus:border-blue-500"
    />

    <!-- Asset / Timeframe / Bouton -->
    <div class="flex gap-3 items-end flex-wrap">
      <div class="min-w-[100px]">
        <label class="text-xs text-gray-400 font-medium block mb-1">Asset</label>
        <select v-model="asset" class="w-full bg-white border border-gray-300 text-black text-sm rounded-lg px-3 py-2">
          <option v-for="a in ASSETS" :key="a" :value="a">{{ a }}</option>
        </select>
      </div>
      <div class="min-w-[90px]">
        <label class="text-xs text-gray-400 font-medium block mb-1">Timeframe</label>
        <select v-model="timeframe" class="w-full bg-white border border-gray-300 text-black text-sm rounded-lg px-3 py-2">
          <option v-for="tf in TIMEFRAMES" :key="tf" :value="tf">{{ tf }}</option>
        </select>
      </div>
      <button
        class="flex-1 py-2 px-6 rounded-lg text-sm font-semibold transition-all"
        :class="
          analyseEnCours || !imageBase64
            ? 'bg-gray-700 text-gray-500 cursor-not-allowed'
            : 'bg-gradient-to-r from-purple-600 to-blue-600 hover:brightness-110 text-white'
        "
        :disabled="analyseEnCours || !imageBase64"
        @click="analyserImage(asset, timeframe)"
      >
        {{ analyseEnCours ? '⏳ Analyse en cours…' : '🔍 Analyser avec llama3.2-vision' }}
      </button>
    </div>

    <!-- Résultats : blocs texte markdown + diagrammes HTML entrelacés -->
    <template v-for="(part, idx) in partsResultat" :key="idx">
      <!-- Bloc texte markdown -->
      <div v-if="part.type === 'text' && part.content.trim()" class="glass-card p-5">
        <div class="flex items-center gap-2 mb-3">
          <span class="text-xs font-semibold text-purple-400">🧠 Analyse IA — {{ modeleUtilise }}</span>
        </div>
        <!-- eslint-disable-next-line vue/no-v-html -->
        <div class="text-sm text-gray-200 leading-relaxed" v-html="renderMd(part.content)" />
      </div>

      <!-- Bloc diagramme HTML isolé dans un iframe sandboxé -->
      <div v-else-if="part.type === 'diagram'" class="glass-card overflow-hidden">
        <div class="px-4 py-2 border-b border-white/10">
          <span class="text-xs font-semibold text-blue-400">📐 Diagramme interactif</span>
        </div>
        <iframe
          :srcdoc="buildSrcdoc(part.content)"
          sandbox="allow-scripts"
          class="w-full border-0 block"
          style="height: 440px; background: #0d1117;"
          title="Diagramme SMC"
        />
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useChartImport, renderMd } from '@/composables/useChartImport'
import { useSettingsStore } from '@/stores/settings.store'

const settingsStore = useSettingsStore()

const ASSETS = ['BTC', 'ETH', 'XAUUSD', 'XAGUSD', 'EURUSD', 'GBPJPY', 'USDJPY', 'DAX', 'NAS100', 'SP500']
const TIMEFRAMES = ['M1', 'M5', 'M15', 'H1', 'H4', 'D1']

const asset = ref(settingsStore.assetActif)
const timeframe = ref(settingsStore.timeframeActif)
const fileInputEl = ref<HTMLInputElement | null>(null)

const {
  imageBase64,
  imagePreview,
  notes,
  analyseEnCours,
  partsResultat,
  dragActif,
  modeleUtilise,
  onDrop,
  onInputFile,
  analyserImage,
  reinitialiser,
  setDragActif,
} = useChartImport()

function buildSrcdoc(html: string): string {
  return `<!DOCTYPE html><html><head><meta charset="utf-8"><style>*{box-sizing:border-box}body{margin:0;padding:16px;background:#0d1117;color:#e6edf3;font-family:-apple-system,system-ui,sans-serif;font-size:13px;}</style></head><body>${html}</body></html>`
}
</script>

<style scoped>
.glass-card { @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm; }
</style>
