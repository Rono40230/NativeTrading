<template>
  <Teleport to="body">
    <Transition name="modal">
      <div
        v-if="visible"
        class="fixed inset-0 z-[10000]"
        style="background: rgba(0,0,0,0.6); backdrop-filter: blur(4px)"
        @click.self="$emit('close')"
      >
        <div
          class="modal-card fixed flex flex-col rounded-2xl border border-white/15 overflow-hidden"
          :style="{ left: pos.x + 'px', top: pos.y + 'px', width: MODAL_W + 'px', maxHeight: '92vh' }"
        >
          <!-- En-tête draggable -->
          <div
            class="flex items-center justify-between p-5 border-b border-white/10 shrink-0 select-none"
            :class="dragging ? 'cursor-grabbing' : 'cursor-grab'"
            @mousedown="startDrag"
          >
            <div>
              <p class="text-sm font-bold text-white tracking-wide">🚀 Opportunités Rockets — Analyse IA</p>
              <p class="text-[10px] text-gray-500 mt-0.5">Top candidats par score · Support / TP · Analyse Ollama</p>
            </div>
            <div class="flex items-center gap-3">
              <!-- Filtres phase -->
              <div class="flex gap-1.5">
                <button
                  v-for="f in filtres"
                  :key="f.val"
                  class="text-[9px] font-semibold px-2 py-0.5 rounded-lg border transition-all"
                  :class="filtre === f.val ? f.classeActif : 'border-white/10 text-gray-500 hover:text-gray-300'"
                  @click="filtre = f.val"
                >{{ f.label }}</button>
              </div>
              <button
                class="text-gray-500 hover:text-white transition-colors p-1 rounded-lg hover:bg-white/10"
                @click="$emit('close')"
              >✕</button>
            </div>
          </div>

          <div class="overflow-y-auto flex-1 p-5 scroll-zone">
            <!-- Stats -->
            <div class="grid grid-cols-3 gap-3 mb-5">
              <div class="stat-bloc">
                <p class="text-[10px] text-gray-500 mb-0.5">Signaux détectés</p>
                <p class="text-2xl font-bold text-white">{{ signaux.length }}</p>
              </div>
              <div class="stat-bloc">
                <p class="text-[10px] text-gray-500 mb-0.5">Score moyen</p>
                <p class="text-2xl font-bold" :class="classeScore(scoreMoyen)">{{ scoreMoyen.toFixed(0) }}/100</p>
              </div>
              <div class="stat-bloc">
                <p class="text-[10px] text-gray-500 mb-0.5">Phase dominante</p>
                <p class="text-lg font-bold text-white">{{ icone(phaseDominante) }} {{ labelPhase(phaseDominante) }}</p>
              </div>
            </div>

            <!-- Cartes top5 | Analyse IA -->
            <div class="grid gap-5" style="grid-template-columns: 420px 1fr">
              <!-- Cartes -->
              <div>
                <p class="text-[10px] font-semibold uppercase tracking-widest text-gray-400 mb-3">Top {{ top5.length }} signaux</p>
                <div class="flex flex-col gap-3">
                  <div
                    v-for="s in top5"
                    :key="s.symbol"
                    class="rounded-xl border bg-white/5 p-3 flex gap-3 items-center"
                    :class="classeBordure(s.phase)"
                  >
                    <svg viewBox="0 0 120 40" style="width:120px;height:40px;flex-shrink:0">
                      <template v-if="s.closes.length >= 2">
                        <polyline
                          :points="sparklinePath(s.closes)"
                          fill="none"
                          :stroke="s.change1h >= 0 ? '#10b981' : '#ef4444'"
                          stroke-width="1.5" stroke-linejoin="round" stroke-linecap="round"
                        />
                      </template>
                      <text v-else x="60" y="22" text-anchor="middle" fill="#374151" font-size="8">…</text>
                    </svg>
                    <div class="flex-1 min-w-0">
                      <div class="flex items-center justify-between mb-1">
                        <span class="text-sm font-bold text-white">{{ s.ticker }}</span>
                        <span class="text-[10px]">{{ icone(s.phase) }} <span class="text-gray-400">{{ labelPhase(s.phase) }}</span></span>
                      </div>
                      <p class="text-[11px] font-mono text-gray-400 mb-1">{{ formatPrix(s.prix) }}$
                        <span :class="s.change1h >= 0 ? 'text-emerald-400' : 'text-red-400'" class="ml-2 font-bold">
                          {{ s.change1h >= 0 ? '+' : '' }}{{ s.change1h.toFixed(2) }}%
                        </span>
                      </p>
                      <div class="grid grid-cols-2 gap-x-2 text-[9px]">
                        <span class="text-gray-500">SL   <span class="text-red-400 font-mono">{{ formatPrix(s.support) }}</span></span>
                        <span class="text-gray-500">TP   <span class="text-emerald-400 font-mono">{{ formatPrix(s.target20) }}</span></span>
                        <span class="text-gray-500">Vol× <span :class="s.ratioVolume >= 2 ? 'text-orange-400 font-bold' : 'text-gray-300'">{{ s.ratioVolume.toFixed(2) }}×</span></span>
                        <span class="text-gray-500">RSI  <span :class="s.rsi > 70 ? 'text-orange-400' : 'text-gray-300'">{{ s.rsi.toFixed(1) }}</span></span>
                      </div>
                    </div>
                    <span class="text-[11px] font-bold shrink-0" :class="classeScore(s.score)">{{ s.score }}/100</span>
                  </div>
                </div>
              </div>

              <!-- Analyse IA -->
              <div class="flex flex-col">
                <div class="flex items-center gap-2 mb-3">
                  <span class="text-[10px] font-semibold uppercase tracking-widest text-orange-400">Analyse IA (Ollama)</span>
                  <div v-if="chargementIA" class="h-2 w-2 animate-pulse rounded-full bg-orange-500" />
                  <button
                    v-if="!chargementIA"
                    class="ml-auto text-[9px] text-gray-500 hover:text-gray-300 transition-colors border border-white/10 rounded px-2 py-0.5"
                    @click="lancerAnalyse"
                  >↺ Relancer</button>
                </div>
                <div v-if="chargementIA" class="flex-1 flex items-center justify-center gap-2 text-xs text-gray-500">
                  <div class="h-4 w-4 animate-spin rounded-full border-2 border-orange-500 border-t-transparent" />
                  Analyse en cours…
                </div>
                <div v-else-if="erreurIA" class="text-xs text-red-400">{{ erreurIA }}</div>
                <div v-else-if="paragraphes.length" class="flex flex-col gap-3 overflow-y-auto flex-1 pr-1">
                  <div
                    v-for="(p, i) in paragraphes"
                    :key="i"
                    class="rounded-lg p-3"
                    :class="p.type"
                  >
                    <p class="text-[11px] leading-relaxed" :class="p.textClass">{{ p.texte }}</p>
                  </div>
                </div>
                <p v-else class="text-xs text-gray-600 italic">Aucune analyse pour l'instant.</p>
              </div>
            </div>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import type { SignalRocket, PhaseRocket } from '@/composables/useVeilleRockets'
import { apiService } from '@/services/api.service'

const props = defineProps<{ visible: boolean; signaux: SignalRocket[] }>()
defineEmits<{ close: [] }>()

const MODAL_W = Math.min(1500, Math.round(window.innerWidth * 0.95))
const pos = ref({ x: 0, y: 0 })
const dragging = ref(false)
let dragOffset = { x: 0, y: 0 }

function initPos() {
  pos.value = {
    x: Math.max(0, (window.innerWidth - MODAL_W) / 2),
    y: Math.max(0, (window.innerHeight - window.innerHeight * 0.92) / 2),
  }
}
function startDrag(e: MouseEvent) {
  dragging.value = true
  dragOffset = { x: e.clientX - pos.value.x, y: e.clientY - pos.value.y }
  window.addEventListener('mousemove', onDrag)
  window.addEventListener('mouseup', stopDrag)
}
function onDrag(e: MouseEvent) {
  pos.value = {
    x: Math.max(0, Math.min(window.innerWidth - MODAL_W, e.clientX - dragOffset.x)),
    y: Math.max(0, Math.min(window.innerHeight - 80, e.clientY - dragOffset.y)),
  }
}
function stopDrag() {
  dragging.value = false
  window.removeEventListener('mousemove', onDrag)
  window.removeEventListener('mouseup', stopDrag)
}

type FiltrePhaseTous = 'tous' | PhaseRocket
const filtre = ref<FiltrePhaseTous>('tous')
const filtres = [
  { val: 'tous' as const,        label: 'Tous',         classeActif: 'border-white/30 text-white bg-white/10' },
  { val: 'compression' as const, label: '🌀 Compression', classeActif: 'border-blue-500/60 text-blue-300 bg-blue-500/10' },
  { val: 'prelancement' as const, label: '⚡ Pré-lancement', classeActif: 'border-yellow-500/60 text-yellow-300 bg-yellow-500/10' },
  { val: 'breakout' as const,    label: '🚀 Breakout',   classeActif: 'border-emerald-500/60 text-emerald-300 bg-emerald-500/10' },
]

const signauxFiltres = computed(() =>
  filtre.value === 'tous' ? props.signaux : props.signaux.filter(s => s.phase === filtre.value)
)
const top5 = computed(() => [...signauxFiltres.value].sort((a, b) => b.score - a.score).slice(0, 5))
const scoreMoyen = computed(() => props.signaux.length === 0 ? 0 : props.signaux.reduce((s, r) => s + r.score, 0) / props.signaux.length)
const phaseDominante = computed<PhaseRocket>(() => {
  const counts = { breakout: 0, prelancement: 0, compression: 0 }
  props.signaux.forEach(s => counts[s.phase]++)
  return (Object.entries(counts).sort((a, b) => b[1] - a[1])[0][0]) as PhaseRocket
})

function icone(phase: PhaseRocket) { return phase === 'breakout' ? '🚀' : phase === 'prelancement' ? '⚡' : '🌀' }
function labelPhase(phase: PhaseRocket) { return phase === 'breakout' ? 'Breakout' : phase === 'prelancement' ? 'Pré-lancement' : 'Compression' }
function classeBordure(phase: PhaseRocket) { return phase === 'breakout' ? 'border-emerald-500/40' : phase === 'prelancement' ? 'border-yellow-500/30' : 'border-blue-500/25' }
function classeScore(s: number) { return s >= 70 ? 'text-orange-400' : s >= 50 ? 'text-emerald-400' : 'text-gray-400' }
function formatPrix(v: number) { return v >= 1000 ? new Intl.NumberFormat('en-US', { maximumFractionDigits: 0 }).format(v) : v >= 1 ? v.toFixed(4) : v.toFixed(6) }
function sparklinePath(closes: number[]) {
  const W = 120, H = 32, min = Math.min(...closes), max = Math.max(...closes), range = max - min || 1
  return closes.map((v, i) => `${((i / (closes.length - 1)) * W).toFixed(1)},${(H - ((v - min) / range) * (H - 4) - 2).toFixed(1)}`).join(' ')
}

const analyseIA = ref(''), chargementIA = ref(false), erreurIA = ref('')
interface Paragraphe { texte: string; type: string; textClass: string }
const paragraphes = computed<Paragraphe[]>(() =>
  !analyseIA.value ? [] : analyseIA.value.split(/\n{2,}/).map(b => b.trim()).filter(Boolean).map(texte => {
    if (/breakout|lancement|entrée|acheter|long|hausse confirm/i.test(texte)) return { texte, type: 'bloc-buy', textClass: 'text-emerald-300' }
    if (/épuis|attention|risque|éviter|baisse|correction|suracheté/i.test(texte)) return { texte, type: 'bloc-warn', textClass: 'text-orange-300' }
    if (/compression|volume|structur|global|contexte|conclusion/i.test(texte)) return { texte, type: 'bloc-context', textClass: 'text-blue-200' }
    return { texte, type: 'bloc-neutral', textClass: 'text-gray-300' }
  })
)

async function lancerAnalyse() {
  if (top5.value.length === 0) return
  chargementIA.value = true; erreurIA.value = ''; analyseIA.value = ''
  try {
    const liste = top5.value.map((s, i) =>
      `${i + 1}. ${s.ticker} — Phase: ${labelPhase(s.phase)} | Variation 1h: ${s.change1h >= 0 ? '+' : ''}${s.change1h.toFixed(2)}% | Vol×: ${s.ratioVolume.toFixed(2)} | ATR ratio: ${s.atrRatio.toFixed(2)} | RSI: ${s.rsi.toFixed(1)} | Support/SL: ${formatPrix(s.support)}$ | Résistance/TP: ${formatPrix(s.target20)}$ | Score: ${s.score}/100`
    ).join('\n')
    const res = await apiService.chatIA([
      {
        role: 'system',
        contenu: `Tu es un analyste spécialisé en stratégie Rocket sur crypto (compression volatilité → breakout). Règles strictes :
- Réponds TOUJOURS en français
- Pour chaque signal : 2-3 phrases MAX avec VERDICT clair : « Breakout imminent », « Phase de compression avancée » ou « Signal épuisé »
- Analyse ATR ratio, volume spike et RSI — déduis la probabilité de breakout
- Conclusion globale en 3 phrases : phase de marché globale, meilleur setup, niveau de risque
- Sépare chaque signal et la conclusion par une ligne vide`
      },
      { role: 'user', contenu: `Top signaux détectés par la stratégie Rocket :\n\n${liste}\n\nAnalyse chaque signal puis donne ta recommandation globale.` }
    ])
    analyseIA.value = res.reponse
  } catch (err) {
    erreurIA.value = `Erreur IA : ${err instanceof Error ? err.message : String(err)}`
  } finally {
    chargementIA.value = false
  }
}

watch(() => props.visible, async (ouvert) => {
  if (!ouvert) { stopDrag(); return }
  initPos(); analyseIA.value = ''; erreurIA.value = ''
  await lancerAnalyse()
})
</script>

<style scoped>
.modal-card { background: #0b0f28; }
.stat-bloc { @apply rounded-xl border border-white/10 bg-white/5 p-4; }
.scroll-zone { scrollbar-width: thin; scrollbar-color: rgba(255,255,255,0.1) transparent; }
.scroll-zone::-webkit-scrollbar { width: 4px; }
.scroll-zone::-webkit-scrollbar-thumb { background: rgba(255,255,255,0.1); border-radius: 2px; }
.bloc-buy     { background: rgba(16,185,129,0.08); border: 1px solid rgba(16,185,129,0.25); }
.bloc-warn    { background: rgba(249,115,22,0.08); border: 1px solid rgba(249,115,22,0.25); }
.bloc-context { background: rgba(59,130,246,0.08); border: 1px solid rgba(59,130,246,0.25); }
.bloc-neutral { background: rgba(255,255,255,0.04); border: 1px solid rgba(255,255,255,0.08); }
.modal-enter-active, .modal-leave-active { transition: opacity 0.18s, transform 0.18s; }
.modal-enter-from, .modal-leave-to { opacity: 0; transform: scale(0.96); }
.modal-enter-to, .modal-leave-from { opacity: 1; transform: scale(1); }
</style>
