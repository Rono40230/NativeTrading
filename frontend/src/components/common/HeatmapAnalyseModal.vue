<template>
  <teleport to="body">
    <div v-if="visible" class="fixed inset-0 z-50 flex items-center justify-center p-3">
      <div class="absolute inset-0 bg-black/70 backdrop-blur-sm" @click="emit('close')" />
      <div class="relative modal-card w-full max-w-6xl max-h-[96vh] flex flex-col overflow-hidden">

        <!-- En-tête -->
        <div class="flex items-center justify-between px-6 py-3 border-b border-white/10 shrink-0">
          <div class="flex items-center gap-4">
            <h2 class="text-lg font-bold">📊 Analyse Volatilité ATR</h2>
            <span class="text-xs text-gray-500">
              {{ classementVol.length }} créneaux ·
              <span :class="nbEleve > 0 ? 'text-red-400 font-semibold' : 'text-gray-500'">{{ nbEleve }} élevés</span>
              · <span class="text-gray-600">fenêtre 80 bougies / TF</span>
            </span>
          </div>
          <button class="text-gray-400 hover:text-white text-xl leading-none px-2" @click="emit('close')">✕</button>
        </div>

        <!-- Corps -->
        <div class="flex-1 overflow-hidden px-5 py-4 flex flex-col gap-4 min-h-0">

          <!-- Bannière meilleur candidat -->
          <div v-if="top1" class="rounded-xl px-5 py-3 border border-red-500/40 bg-red-500/10 flex items-center gap-4 shrink-0">
            <span class="text-2xl">🎯</span>
            <div class="flex-1 min-w-0">
              <p class="text-[10px] text-red-300 uppercase tracking-wider font-semibold">Meilleur candidat Straddle</p>
              <p class="text-white font-bold text-base">{{ top1.asset }} — {{ top1.tf }}</p>
            </div>
            <div class="text-right shrink-0">
              <p class="text-red-400 font-bold text-2xl">{{ top1.atr.toFixed(0) }}%</p>
              <p class="text-[10px] text-gray-500">du ratio ATR moyen</p>
            </div>
          </div>

          <!-- 3 colonnes -->
          <div class="grid grid-cols-3 gap-4 flex-1 min-h-0">

            <!-- Plus volatile -->
            <div class="section-card flex flex-col min-h-0">
              <h3 class="section-title shrink-0">🔴 Plus volatile</h3>
              <div class="overflow-y-auto flex-1">
                <div v-for="(item, i) in classementVol.slice(0, 5)" :key="item.cle" class="flex items-center gap-2 py-2 border-b border-white/5 last:border-0">
                  <span class="text-[10px] text-gray-600 font-mono w-5 shrink-0">#{{ i + 1 }}</span>
                  <div class="flex-1 min-w-0">
                    <span class="text-sm font-semibold text-white">{{ item.asset }}</span>
                    <span class="ml-1.5 tf-badge">{{ item.tf }}</span>
                  </div>
                  <span class="text-red-400 font-bold text-sm shrink-0">{{ item.atr.toFixed(0) }}%</span>
                </div>
              </div>
            </div>

            <!-- Moins volatile -->
            <div class="section-card flex flex-col min-h-0">
              <h3 class="section-title shrink-0">🟢 Moins volatile</h3>
              <div class="overflow-y-auto flex-1">
                <div v-for="(item, i) in classementVol.slice(-5).reverse()" :key="item.cle" class="flex items-center gap-2 py-2 border-b border-white/5 last:border-0">
                  <span class="text-[10px] text-gray-600 font-mono w-5 shrink-0">#{{ i + 1 }}</span>
                  <div class="flex-1 min-w-0">
                    <span class="text-sm font-semibold text-white">{{ item.asset }}</span>
                    <span class="ml-1.5 tf-badge">{{ item.tf }}</span>
                  </div>
                  <span class="text-emerald-400 font-bold text-sm shrink-0">{{ item.atr.toFixed(0) }}%</span>
                </div>
              </div>
            </div>

            <!-- Synthèse -->
            <div class="section-card flex flex-col gap-3 min-h-0">
              <h3 class="section-title shrink-0">🔎 Synthèse</h3>
              <div class="grid grid-cols-2 gap-2 shrink-0">
                <div class="stat-box">
                  <p class="text-[10px] text-amber-400 uppercase mb-1">Le plus actif</p>
                  <p class="text-white font-bold text-base">{{ analyseAtr?.assetActif ?? '—' }}</p>
                </div>
                <div class="stat-box">
                  <p class="text-[10px] text-blue-400 uppercase mb-1">Le plus calme</p>
                  <p class="text-white font-bold text-base">{{ analyseAtr?.assetCalme ?? '—' }}</p>
                </div>
                <div class="stat-box relative group cursor-help">
                  <p class="text-[10px] text-red-400 uppercase mb-1">Créneaux élevés</p>
                  <p class="text-red-300 font-bold text-xl">{{ nbEleve }}</p>
                  <!-- Tooltip créneaux élevés -->
                  <div
                    v-if="creneauxEleves.length"
                    class="absolute bottom-full left-1/2 -translate-x-1/2 mb-2 z-20 hidden group-hover:block w-52 rounded-lg bg-gray-950 border border-white/10 shadow-xl px-3 py-2"
                  >
                    <p class="text-[10px] text-red-300 uppercase font-semibold mb-1.5">Créneaux &gt; 120 %</p>
                    <div v-for="c in creneauxEleves" :key="c.cle" class="flex justify-between text-[11px] py-0.5">
                      <span class="text-gray-300">{{ c.asset }} <span class="tf-badge">{{ c.tf }}</span></span>
                      <span class="text-red-400 font-mono">{{ c.atr.toFixed(0) }}%</span>
                    </div>
                  </div>
                </div>
                <div class="stat-box">
                  <p class="text-[10px] text-gray-400 uppercase mb-1">Assets actifs</p>
                  <p class="text-white font-bold text-xl">{{ nbAssetsActifs }}</p>
                </div>
              </div>
              <div v-if="analyseAtr" class="rounded-lg px-3 py-2.5 text-xs font-medium mt-auto shrink-0" :class="analyseAtr.straddleClass">
                {{ analyseAtr.straddleConseil }}
              </div>
            </div>

          </div>

          <!-- Tableau tendances -->
          <HeatmapTendancesTable
            v-if="analyseAtr"
            :classement-vol="classementVol"
            :tfs-actifs-par-asset="analyseAtr.tfsActifsParAsset"
            :assets="assets"
            class="shrink-0"
          />

        </div>
      </div>
    </div>
  </teleport>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import HeatmapTendancesTable from './HeatmapTendancesTable.vue'

const props = defineProps<{
  visible: boolean
  classementVol: { cle: string; asset: string; tf: string; atr: number }[]
  analyseAtr: {
    assetActif: string
    assetCalme: string
    tfsActifsParAsset: Record<string, string[]>
    straddleConseil: string
    straddleClass: string
  } | null
  assets: string[]
}>()

const emit = defineEmits<{ close: [] }>()

const creneauxEleves = computed(() => props.classementVol.filter(i => i.atr > 120))
const nbEleve        = computed(() => creneauxEleves.value.length)
const top1           = computed(() => props.classementVol[0] ?? null)
const nbAssetsActifs = computed(() =>
  props.assets.filter(a => (props.analyseAtr?.tfsActifsParAsset[a] ?? []).length > 0).length
)
</script>

<style scoped>
.modal-card    { @apply rounded-xl border border-white/15 bg-[#0f1225] shadow-2xl; }
.section-card  { @apply rounded-xl border border-white/10 bg-white/5 px-4 py-3; }
.section-title { @apply text-[10px] font-semibold text-gray-400 uppercase tracking-wider mb-3; }
.tf-badge      { @apply text-[10px] bg-white/10 text-gray-400 px-1.5 py-0.5 rounded font-mono; }
.stat-box      { @apply rounded-lg bg-white/5 border border-white/10 px-3 py-2; }
</style>
