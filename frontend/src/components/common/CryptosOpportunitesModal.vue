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
              <p class="text-sm font-bold text-white tracking-wide">🎯 Opportunités Crypto — Analyse IA</p>
              <p class="text-[10px] text-gray-500 mt-0.5">Top 5 cryptos en forte hausse · TP/SL indicatifs · Analyse Ollama</p>
            </div>
            <button
              class="text-gray-500 hover:text-white transition-colors p-1 rounded-lg hover:bg-white/10"
              @click="$emit('close')"
            >✕</button>
          </div>

          <div class="overflow-y-auto flex-1 p-5 scroll-zone">
            <!-- Stats -->
            <div class="grid grid-cols-3 gap-3 mb-5">
              <div class="stat-bloc">
                <p class="text-[10px] text-gray-500 mb-0.5">Cryptos en hausse ≥10%</p>
                <p class="text-2xl font-bold text-white">{{ top20.length }}</p>
              </div>
              <div class="stat-bloc">
                <p class="text-[10px] text-gray-500 mb-0.5">Score momentum global</p>
                <p class="text-2xl font-bold" :class="classeScoreGlobal">{{ scoreMoyen.toFixed(1) }}/100</p>
              </div>
              <div class="stat-bloc">
                <p class="text-[10px] text-gray-500 mb-0.5">Condition de marché</p>
                <p class="text-lg font-bold" :class="classeScoreGlobal">{{ conditionMarche }}</p>
              </div>
            </div>

            <!-- Deux colonnes : cartes | analyse -->
            <div class="grid gap-5" style="grid-template-columns: 420px 1fr">
              <!-- Cartes Top 5 -->
              <div>
                <p class="text-[10px] font-semibold uppercase tracking-widest text-gray-400 mb-3">Top 5 opportunités</p>
                <div class="flex flex-col gap-3">
                  <div
                    v-for="c in top5"
                    :key="c.symbol"
                    class="opport-card rounded-xl border border-white/10 bg-white/5 p-3 flex gap-3 items-center"
                  >
                    <!-- Sparkline -->
                    <svg viewBox="0 0 120 40" style="width:120px;height:40px;flex-shrink:0">
                      <template v-if="(sparklines[c.symbol] ?? []).length >= 2">
                        <polyline
                          :points="sparklinePath(sparklines[c.symbol] ?? [])"
                          fill="none" stroke="#10b981" stroke-width="1.5"
                          stroke-linejoin="round" stroke-linecap="round"
                        />
                      </template>
                      <text v-else x="60" y="22" text-anchor="middle" fill="#374151" font-size="8">…</text>
                    </svg>
                    <!-- Infos -->
                    <div class="flex-1 min-w-0">
                      <div class="flex items-center justify-between mb-1">
                        <span class="text-sm font-bold text-white">{{ c.ticker }}</span>
                        <span class="text-xs font-bold text-emerald-400">+{{ c.change24h.toFixed(2) }}%</span>
                      </div>
                      <p class="text-[11px] font-mono text-gray-400 mb-1">{{ formatPrix(c.prix) }}$</p>
                      <div class="grid grid-cols-2 gap-x-2 text-[9px]">
                        <span class="text-gray-500">TP1 <span class="text-emerald-400 font-mono">{{ formatPrix(c.prix * 1.05) }}</span></span>
                        <span class="text-gray-500">TP2 <span class="text-emerald-400 font-mono">{{ formatPrix(c.prix * 1.10) }}</span></span>
                        <span class="text-gray-500">TP3 <span class="text-emerald-300 font-mono">{{ formatPrix(c.prix * 1.20) }}</span></span>
                        <span class="text-gray-500">SL   <span class="text-red-400 font-mono">{{ formatPrix(c.prix * 0.97) }}</span></span>
                      </div>
                    </div>
                  </div>
                </div>
              </div>

              <!-- Analyse IA -->
              <div class="flex flex-col">
                <div class="flex items-center gap-2 mb-3">
                  <span class="text-[10px] font-semibold uppercase tracking-widest text-blue-400">Analyse IA (Ollama)</span>
                  <div v-if="chargementIA" class="h-2 w-2 animate-pulse rounded-full bg-blue-500" />
                  <button
                    v-if="!chargementIA"
                    class="ml-auto text-[9px] text-gray-500 hover:text-gray-300 transition-colors border border-white/10 rounded px-2 py-0.5"
                    @click="lancerAnalyse"
                  >↺ Relancer</button>
                </div>
                <div v-if="chargementIA" class="flex-1 flex items-center justify-center gap-2 text-xs text-gray-500">
                  <div class="h-4 w-4 animate-spin rounded-full border-2 border-blue-500 border-t-transparent" />
                  Analyse en cours…
                </div>
                <div v-else-if="erreurIA" class="text-xs text-red-400">{{ erreurIA }}</div>
                <div v-else-if="paragraphes.length" class="flex flex-col gap-3 overflow-y-auto flex-1 pr-1">
                  <div
                    v-for="(p, i) in paragraphes"
                    :key="i"
                    class="analyse-bloc rounded-lg p-3"
                    :class="p.type"
                  >
                    <p class="text-[11px] leading-relaxed" :class="p.textClass">{{ p.texte }}</p>
                  </div>
                </div>
                <p v-else class="text-xs text-gray-600 italic">Aucune analyse pour l’instant.</p>
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
import type { CryptoAlert } from '@/composables/useCryptosAlert'
import { apiService } from '@/services/api.service'

const props = defineProps<{
  visible: boolean
  top20: CryptoAlert[]
}>()

defineEmits<{ close: [] }>()

// --- Drag ---
const MODAL_W = Math.min(1500, Math.round(window.innerWidth * 0.95))
const pos = ref({ x: 0, y: 0 })
const dragging = ref(false)
let dragOffset = { x: 0, y: 0 }

function initPos() {
  pos.value = {
    x: Math.max(0, (window.innerWidth  - MODAL_W) / 2),
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
    x: Math.max(0, Math.min(window.innerWidth  - MODAL_W, e.clientX - dragOffset.x)),
    y: Math.max(0, Math.min(window.innerHeight - 80,      e.clientY - dragOffset.y)),
  }
}

function stopDrag() {
  dragging.value = false
  window.removeEventListener('mousemove', onDrag)
  window.removeEventListener('mouseup', stopDrag)
}

const top5 = computed(() => props.top20.slice(0, 5))

const scoreMoyen = computed(() =>
  props.top20.length === 0 ? 0 : props.top20.reduce((s, c) => s + c.score, 0) / props.top20.length
)

const conditionMarche = computed(() => {
  const s = scoreMoyen.value
  if (s >= 70) return '🔥 Euphorie'
  if (s >= 50) return '⚡ Bull fort'
  if (s >= 30) return '📈 Hausse modérée'
  return '😐 Neutre'
})

const classeScoreGlobal = computed(() => {
  const s = scoreMoyen.value
  if (s >= 70) return 'text-red-400'
  if (s >= 50) return 'text-orange-400'
  if (s >= 30) return 'text-emerald-400'
  return 'text-gray-400'
})

const sparklines = ref<Record<string, number[]>>({})
const analyseIA = ref('')
const chargementIA = ref(false)
const erreurIA = ref('')

interface Paragraphe { texte: string; type: string; textClass: string }

const paragraphes = computed<Paragraphe[]>(() => {
  if (!analyseIA.value) return []
  return analyseIA.value
    .split(/\n{2,}/)
    .map(bloc => bloc.trim())
    .filter(Boolean)
    .map(texte => {
      const low = texte.toLowerCase()
      if (/opportunit|entrée|acheter|long|hausse confirm/i.test(texte))
        return { texte, type: 'bloc-buy', textClass: 'text-emerald-300' }
      if (/épuis|survente|attention|risque|éviter|baisse|correction/i.test(texte))
        return { texte, type: 'bloc-warn', textClass: 'text-orange-300' }
      if (/vision|marché|global|contexte|conclusion/i.test(texte))
        return { texte, type: 'bloc-context', textClass: 'text-blue-200' }
      return { texte, type: 'bloc-neutral', textClass: 'text-gray-300' }
    })
})

function sparklinePath(closes: number[]): string {
  const W = 120, H = 32
  const min = Math.min(...closes), max = Math.max(...closes)
  const range = max - min || 1
  return closes.map((v, i) => {
    const x = (i / (closes.length - 1)) * W
    const y = H - ((v - min) / range) * (H - 4) - 2
    return `${x.toFixed(1)},${y.toFixed(1)}`
  }).join(' ')
}

function formatPrix(v: number): string {
  if (v >= 1000) return new Intl.NumberFormat('en-US', { maximumFractionDigits: 0 }).format(v)
  return v >= 1 ? v.toFixed(4) : v.toFixed(6)
}

async function fetchSparklines() {
  await Promise.all(top5.value.map(async (c) => {
    try {
      const res = await fetch(`https://api.binance.com/api/v3/klines?symbol=${c.ticker}USDT&interval=1h&limit=24`)
      if (!res.ok) return
      const data = await res.json() as unknown[][]
      sparklines.value[c.symbol] = data.map(k => parseFloat(k[4] as string))
    } catch { /* silencieux */ }
  }))
}

async function lancerAnalyse() {
  if (top5.value.length === 0) return
  chargementIA.value = true
  erreurIA.value = ''
  analyseIA.value = ''
  try {
    const liste = top5.value.map((c, i) =>
      `${i + 1}. ${c.ticker} — Hausse: +${c.change24h.toFixed(2)}% | Prix: ${formatPrix(c.prix)}$ | Volume 24h: ${(c.volume24h / 1_000_000).toFixed(1)}M$ | Trades: ${c.nbTrades.toLocaleString('fr-FR')} | Score momentum: ${c.score.toFixed(0)}/100`
    ).join('\n')
    const res = await apiService.chatIA([
      {
        role: 'system',
        contenu: `Tu es un analyste quantitatif spécialisé en cryptomonnaies. Tu fournis des analyses courtes, structurées et actionnables. Règles strictes :
- Réponds TOUJOURS en français
- Pour chaque crypto : 2-3 phrases MAX avec VERDICT clair : « Opportunité d’entrée » ou « Mouvement épuisé » ou « Attention risque »
- Analyse uniquement momentum, volume et trades (pas d’indicateurs non fournis)
- Conclusion globale en 3 phrases : condition de marché, crypto favorite, niveau de risque global
- Sépare chaque crypto et la conclusion par une ligne vide`
      },
      {
        role: 'user',
        contenu: `Top 5 cryptos avec la plus forte hausse 24h sur Binance :\n\n${liste}\n\nAnalyse chaque crypto puis donne ta conclusion globale sur le marché crypto actuel.`
      }
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
  initPos()
  analyseIA.value = ''
  erreurIA.value = ''
  await fetchSparklines()
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
