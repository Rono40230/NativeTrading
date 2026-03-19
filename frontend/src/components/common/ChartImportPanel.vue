<template>
  <div class="space-y-4">
    <!-- Zone drag & drop -->
    <div
      class="rounded-xl border-2 border-dashed transition-colors cursor-pointer relative min-h-[260px]"
      :class="[
        dragActif
          ? 'border-blue-400 bg-blue-500/8'
          : images.length > 0
            ? 'border-emerald-500/40 bg-emerald-900/5'
            : 'border-white/20 hover:border-white/40',
      ]"
      @dragover.prevent="setDragActif(true)"
      @dragleave.prevent="setDragActif(false)"
      @drop.prevent="onDrop"
      @click="fileInputEl?.click()"
    >
      <input ref="fileInputEl" type="file" accept="image/*" multiple class="hidden" @change="onInputFile" />

      <div v-if="images.length === 0" class="flex flex-col items-center justify-center gap-2 py-10 pointer-events-none">
        <span class="text-3xl">📊</span>
        <p class="text-sm text-gray-400">Glissez vos screenshots de charts ici</p>
        <p class="text-xs text-gray-600">Plusieurs images acceptées — analyse top-down multi-TF</p>
      </div>

      <div v-else class="p-3 grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-4" @click.stop>
        <div
          v-for="(img, idx) in images"
          :key="idx"
          class="relative rounded-lg overflow-hidden border border-white/10 bg-black/20"
        >
          <img :src="img.preview" :alt="`Chart ${idx + 1}`" class="w-full h-56 object-cover" />
          <div class="absolute bottom-0 left-0 right-0 bg-black/70 px-2 py-1 flex items-center justify-between gap-1">
            <select
              :value="img.timeframe"
              class="flex-1 bg-white text-black text-xs font-semibold border-0 outline-none cursor-pointer rounded"
              @change="mettreAJourTF(idx, ($event.target as HTMLSelectElement).value)"
              @click.stop
            >
              <option v-for="tf in TIMEFRAMES" :key="tf" :value="tf" class="bg-white text-black">{{ tf }}</option>
            </select>
            <button
              class="text-gray-400 hover:text-red-400 text-xs leading-none ml-1"
              @click.stop="supprimerImage(idx)"
            >✕</button>
          </div>
        </div>
        <!-- Tuile Ajouter -->
        <div
          class="flex flex-col items-center justify-center gap-1 h-56 rounded-lg border border-dashed border-white/20 hover:border-white/40 transition-colors text-gray-600 hover:text-gray-400 cursor-pointer"
          @click.stop="fileInputEl?.click()"
        >
          <span class="text-2xl leading-none">＋</span>
          <span class="text-xs">Ajouter</span>
        </div>
      </div>
    </div>

    <!-- Notes contextuelles -->
    <textarea
      v-model="notes"
      rows="2"
      placeholder="Notes contextuelles optionnelles — contexte macro, niveaux importants…"
      class="w-full bg-gray-800 border border-gray-600 text-white text-sm rounded-lg px-3 py-2 resize-none placeholder:text-gray-600 focus:outline-none focus:border-blue-500"
    />

    <!-- Asset + Bouton -->
    <div class="flex gap-3 items-end flex-wrap">
      <div class="min-w-[100px]">
        <label class="text-xs text-gray-400 font-medium block mb-1">Asset</label>
        <select v-model="asset" class="w-full bg-white border border-gray-300 text-black text-sm rounded-lg px-3 py-2">
          <option v-for="a in ASSETS" :key="a" :value="a">{{ a }}</option>
        </select>
      </div>
      <button
        class="flex-1 py-2 px-6 rounded-lg text-sm font-semibold transition-all"
        :class="
          analyseEnCours || images.length === 0
            ? 'bg-gray-700 text-gray-500 cursor-not-allowed'
            : 'bg-gradient-to-r from-purple-600 to-blue-600 hover:brightness-110 text-white'
        "
        :disabled="analyseEnCours || images.length === 0"
        @click="analyserImage(asset)"
      >
        {{ analyseEnCours ? '⏳ Analyse en cours…' : `🔍 Analyser (${images.length} image${images.length > 1 ? 's' : ''})` }}
      </button>
    </div>

    <!-- Résultats -->
    <template v-for="(part, idx) in partsResultat" :key="idx">
      <div v-if="part.type === 'text' && part.content.trim()" class="glass-card p-5">
        <div class="flex items-center gap-2 mb-3">
          <span class="text-xs font-semibold text-purple-400">🧠 Analyse IA — {{ modeleUtilise }}</span>
        </div>
        <!-- eslint-disable-next-line vue/no-v-html -->
        <div class="text-sm text-gray-200 leading-relaxed" v-html="renderMd(part.content)" />
      </div>
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

const ASSETS = ['BTC', 'ETH', 'XAUUSD', 'XAGUSD', 'EURUSD', 'GBPJPY', 'USDJPY', 'DAX', 'SP500']
const TIMEFRAMES = ['M1', 'M5', 'M15', 'H1', 'H4', 'D1']

const asset = ref(settingsStore.assetActif)
const fileInputEl = ref<HTMLInputElement | null>(null)

const {
  images,
  notes,
  analyseEnCours,
  partsResultat,
  dragActif,
  modeleUtilise,
  onDrop,
  onInputFile,
  analyserImage,
  supprimerImage,
  mettreAJourTF,
  setDragActif,
} = useChartImport()

function buildSrcdoc(html: string): string {
  return `<!DOCTYPE html><html><head><meta charset="utf-8"><style>*{box-sizing:border-box}body{margin:0;padding:16px;background:#0d1117;color:#e6edf3;font-family:-apple-system,system-ui,sans-serif;font-size:13px;}</style></head><body>${html}</body></html>`
}
</script>

<style scoped>
.glass-card { @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm; }
</style>
