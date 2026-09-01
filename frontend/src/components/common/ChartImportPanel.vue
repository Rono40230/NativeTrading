<template>
  <div class="grid grid-cols-2 gap-5 h-full min-h-0">

    <!-- ── Colonne 1 : Importation ── -->
    <div class="flex flex-col gap-3 min-h-0">

      <!-- Zone drag & drop — image unique (flex-1 = prend tout l'espace dispo) -->
      <div
        class="flex-1 min-h-0 rounded-xl border-2 border-dashed transition-colors relative overflow-hidden"
        :class="dragActif ? 'border-blue-400 bg-blue-500/8' : images.length > 0 ? 'border-emerald-500/40 bg-emerald-900/5 cursor-default' : 'border-white/20 hover:border-white/40 cursor-pointer'"
        @dragover.prevent="setDragActif(true)"
        @dragleave.prevent="setDragActif(false)"
        @drop.prevent="onDrop"
        @click="images.length === 0 && fileInputEl?.click()"
      >
        <input ref="fileInputEl" type="file" accept="image/*" class="hidden" @change="onInputFile" />

        <!-- État vide -->
        <div v-if="images.length === 0" class="flex flex-col items-center justify-center gap-3 h-full pointer-events-none">
          <span class="text-5xl">📊</span>
          <p class="text-sm text-white">Glissez votre screenshot ici</p>
          <p class="text-xs text-white">Un seul graphique — cliquez ou déposez</p>
        </div>

        <!-- Image chargée -->
        <div v-else class="h-full flex flex-col" @click.stop>
          <img :src="images[0].preview" alt="Chart importé" class="flex-1 w-full object-contain min-h-0 bg-black" />
          <div class="flex-shrink-0 bg-black/90 px-3 py-2 flex items-center justify-between gap-2 border-t border-white/10">
            <div class="flex items-center gap-3">
              <div class="flex items-center gap-1.5">
                <span class="text-xs text-white">TF :</span>
                <select
                  :value="images[0].timeframe"
                  class="bg-white text-black text-xs font-semibold border-0 outline-none cursor-pointer rounded px-2 py-1"
                  @change="mettreAJourTF(0, ($event.target as HTMLSelectElement).value)"
                  @click.stop
                >
                  <option v-for="tf in TIMEFRAMES" :key="tf" :value="tf" class="bg-white text-black">{{ tf }}</option>
                </select>
              </div>
              <div class="flex items-center gap-1.5">
                <span class="text-xs text-white">Asset :</span>
                <select v-model="asset" class="bg-white text-black text-xs font-semibold border-0 outline-none cursor-pointer rounded px-2 py-1" @click.stop>
                  <option v-for="a in ASSETS" :key="a" :value="a" class="bg-white text-black">{{ a }}</option>
                </select>
              </div>
            </div>
            <button
              class="text-xs text-red-400 hover:text-red-300 px-3 py-1 rounded border border-red-500/30 hover:bg-red-500/10 transition-colors"
              @click.stop="supprimerImage(0)"
            >✕ Retirer</button>
          </div>
        </div>
      </div>

      <!-- Notes contextuelles -->
      <textarea
        v-model="notes"
        rows="2"
        placeholder="Notes contextuelles — contexte macro, niveaux clés, biais HTF…"
        class="flex-shrink-0 w-full bg-gray-800 border border-gray-600 text-white text-sm rounded-lg px-3 py-2 resize-none placeholder:text-white focus:outline-none focus:border-blue-500"
      />

      <!-- Bouton analyse -->
      <div class="flex-shrink-0">
        <button
          class="w-full py-2.5 px-4 rounded-lg text-sm font-semibold transition-all"
          :class="analyseEnCours || analyseLocalEnCours || images.length === 0 ? 'bg-gray-700 text-white cursor-not-allowed' : 'bg-gradient-to-r from-purple-600 to-blue-600 hover:brightness-110 text-white'"
          :disabled="analyseEnCours || analyseLocalEnCours || images.length === 0"
          @click="analyserImage(asset)"
        >
          {{ analyseEnCours || analyseLocalEnCours ? '⏳ Analyse en cours…' : anthropicActifChart ? '🔍 Analyser avec Claude' : '🔍 Analyser avec qwen2.5vl:7b' }}
        </button>
      </div>

    </div>

    <!-- ── Colonne 2 : Résultats ── -->
    <div class="flex flex-col h-full min-h-0 rounded-xl border border-white/8 bg-white/5">

      <!-- Placeholder vide -->
      <div
        v-if="activeSections.length === 0 && !analyseEnCours && !analyseLocalEnCours"
        class="flex flex-col items-center justify-center h-full text-white select-none"
      >
        <span class="text-5xl mb-4">🧠</span>
        <p class="text-sm">L’analyse apparaîtra ici</p>
        <p class="text-xs mt-1">Importez un graphique puis lancez l’analyse</p>
      </div>

      <!-- Loader -->
      <div
        v-else-if="(analyseEnCours || analyseLocalEnCours) && activeSections.length === 0"
        class="flex flex-col items-center justify-center h-full gap-4 text-white"
      >
        <div class="w-8 h-8 border-2 border-blue-500 border-t-transparent rounded-full animate-spin" />
        <p class="text-sm">Analyse en cours…</p>
      </div>

      <!-- Résultats -->
      <template v-else>

        <!-- Badge modèle + asset -->
        <div class="flex-shrink-0 flex items-center gap-2 px-4 pt-4 pb-3 border-b border-white/8">
          <span
            class="inline-flex items-center gap-1.5 text-xs font-semibold px-3 py-1 rounded-full border"
            :class="anthropicActifChart ? 'bg-purple-500/15 text-purple-300 border-purple-500/30' : 'bg-emerald-500/15 text-emerald-300 border-emerald-500/30'"
          >
            {{ anthropicActifChart ? '🧠 Claude Sonnet' : '🤖 qwen2.5vl:7b' }}
          </span>
          <span class="text-xs font-bold text-white/70 tracking-wide">{{ asset }}</span>
        </div>

        <!-- Sections scrollables -->
        <div class="flex-1 overflow-y-auto min-h-0 p-3 space-y-2.5">
          <template v-for="(section, si) in activeSections" :key="si">

            <!-- Section texte structurée -->
            <div v-if="section.type === 'section'" class="section-card" :class="section.colorClass">
              <div v-if="section.title" class="section-header" :class="section.headerClass">
                <span class="section-icon">{{ section.icon }}</span>
                <span class="section-title">{{ section.title }}</span>
              </div>
              <!-- eslint-disable-next-line vue/no-v-html -->
              <div class="section-body" v-html="section.html" />
            </div>

            <!-- Tableau -->
            <div v-else-if="section.type === 'table'" class="table-card">
              <!-- eslint-disable-next-line vue/no-v-html -->
              <div v-html="section.html" />
            </div>

            <!-- Diagramme -->
            <div v-else-if="section.type === 'diagram'" class="glass-card overflow-hidden">
              <div class="px-4 py-2 border-b border-white/10">
                <span class="text-xs font-semibold text-blue-400">△ Diagramme</span>
              </div>
              <iframe :srcdoc="buildSrcdoc(section.html)" sandbox="allow-scripts" class="w-full border-0 block" style="height:400px;background:#0d1117" title="Diagramme" />
            </div>

          </template>
        </div>
      </template>

    </div>

  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useChartImport, buildSections, anthropicActifChart } from '@/composables/useChartImport'
import { useSettingsStore } from '@/stores/settings.store'
import { useAssetsStore } from '@/stores/assets.store'

const settingsStore = useSettingsStore()
const assetsStore = useAssetsStore()

const ASSETS = computed(() =>
  assetsStore.assets.length > 0
    ? assetsStore.assets.map(a => a.id)
    : ['BTC', 'ETH', 'XAUUSD', 'XAGUSD', 'EURUSD', 'GBPJPY', 'USDJPY', 'DAX', 'SP500']
)
const TIMEFRAMES = ['M1', 'M5', 'M15', 'H1', 'H4', 'D1']

const asset = ref(settingsStore.assetActif)
const fileInputEl = ref<HTMLInputElement | null>(null)

onMounted(() => assetsStore.chargerAssets())

const {
  images,
  notes,
  analyseEnCours,
  partsResultat,
  dragActif,
  modeleUtilise,
  analyseLocalEnCours,
  partsResultatLocal,
  modeleLocalUtilise,
  onDrop,
  onInputFile,
  analyserImage,
  supprimerImage,
  mettreAJourTF,
  setDragActif,
} = useChartImport()

const activeParts = computed(() =>
  partsResultat.value.length > 0 ? partsResultat.value : partsResultatLocal.value
)

const activeSections = computed(() => buildSections(activeParts.value))

function buildSrcdoc(html: string): string {
  return `<!DOCTYPE html><html><head><meta charset="utf-8"><style>*{box-sizing:border-box}body{margin:0;padding:16px;background:#0d1117;color:#e6edf3;font-family:-apple-system,system-ui,sans-serif;font-size:13px;}</style></head><body>${html}</body></html>`
}
</script>

<style scoped>
.glass-card { @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm; }

.section-card {
  @apply rounded-xl border overflow-hidden;
}
.section-card.blue   { @apply border-blue-500/20 bg-blue-950/25; }
.section-card.green  { @apply border-emerald-500/20 bg-emerald-950/25; }
.section-card.red    { @apply border-red-500/20 bg-red-950/20; }
.section-card.yellow { @apply border-yellow-500/20 bg-yellow-950/20; }
.section-card.purple { @apply border-purple-500/20 bg-purple-950/25; }
.section-card.orange { @apply border-orange-500/20 bg-orange-950/20; }
.section-card.gray   { @apply border-white/10 bg-white/5; }

.section-header {
  @apply flex items-center gap-2 px-4 py-2 border-b border-white/10;
}
.section-header.blue   { @apply bg-blue-900/30; }
.section-header.green  { @apply bg-emerald-900/30; }
.section-header.red    { @apply bg-red-900/25; }
.section-header.yellow { @apply bg-yellow-900/25; }
.section-header.purple { @apply bg-purple-900/30; }
.section-header.orange { @apply bg-orange-900/25; }
.section-header.gray   { @apply bg-white/5; }

.section-icon  { @apply text-base leading-none; }
.section-title { @apply text-sm font-bold text-white/90 tracking-wide; }
.section-body  { @apply px-4 py-3 text-sm text-white leading-6; }

.table-card { @apply rounded-xl border border-white/10 bg-white/5 overflow-hidden px-4 py-3; }
</style>
