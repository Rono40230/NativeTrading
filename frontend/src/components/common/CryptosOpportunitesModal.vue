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
          :style="{ left: pos.x + 'px', top: pos.y + 'px', width: MODAL_W + 'px', minHeight: '80vh', maxHeight: '96vh' }"
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

            <!-- Tableau top 5 -->
            <div class="overflow-x-auto rounded-xl border border-white/10 mb-5">
              <table class="w-full text-[11px]">
                <thead>
                  <tr class="border-b border-white/10 bg-white/[0.03] text-[9px] text-gray-500 uppercase tracking-widest font-semibold">
                    <th class="text-left px-3 py-2 w-44">Tendance 1h</th>
                    <th class="text-left px-3 py-2">Ticker</th>
                    <th class="text-right px-3 py-2">+24h%</th>
                    <th class="text-right px-3 py-2">Prix</th>
                    <th class="text-right px-3 py-2 text-red-400">SL (−3%)</th>
                    <th class="text-right px-3 py-2 text-emerald-400">TP1 (+5%)</th>
                    <th class="text-right px-3 py-2 text-emerald-400">TP2 (+10%)</th>
                    <th class="text-right px-3 py-2 text-emerald-300">TP3 (+20%)</th>
                    <th class="text-right px-3 py-2">Score</th>
                  </tr>
                </thead>
                <tbody>
                  <tr v-for="c in top5" :key="c.symbol" class="border-b border-white/5 hover:bg-white/[0.03] transition-colors">
                    <td class="px-3 py-2">
                      <svg viewBox="0 0 160 52" style="width:160px;height:52px">
                        <polyline v-if="(sparklines[c.symbol]??[]).length>=2" :points="sparklinePath(sparklines[c.symbol]??[])" fill="none" stroke="#10b981" stroke-width="1.5" stroke-linejoin="round" stroke-linecap="round"/>
                        <text v-else x="80" y="28" text-anchor="middle" fill="#374151" font-size="9">…</text>
                      </svg>
                    </td>
                    <td class="px-3 py-2 font-bold text-white">{{ c.ticker }}</td>
                    <td class="px-3 py-2 text-right font-bold text-emerald-400">+{{ c.change24h.toFixed(2) }}%</td>
                    <td class="px-3 py-2 text-right font-mono text-gray-300">{{ formatPrix(c.prix) }}$</td>
                    <td class="px-3 py-2 text-right font-mono text-red-400">{{ formatPrix(c.prix * 0.97) }}</td>
                    <td class="px-3 py-2 text-right font-mono text-emerald-400">{{ formatPrix(c.prix * 1.05) }}</td>
                    <td class="px-3 py-2 text-right font-mono text-emerald-400">{{ formatPrix(c.prix * 1.10) }}</td>
                    <td class="px-3 py-2 text-right font-mono text-emerald-300">{{ formatPrix(c.prix * 1.20) }}</td>
                    <td class="px-3 py-2 text-right">
                      <span class="font-bold" :class="classeScore(c.score)">{{ c.score.toFixed(0) }}</span><span class="text-gray-600">/100</span>
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>

            <!-- Analyse IA -->
            <div>
              <div class="flex items-center gap-2 mb-3">
                <span class="text-[10px] font-semibold uppercase tracking-widest text-blue-400">Analyse IA (Ollama)</span>
                <div v-if="chargementIA" class="h-2 w-2 animate-pulse rounded-full bg-blue-500" />
                <button v-if="!chargementIA" class="ml-auto text-[9px] text-gray-500 hover:text-gray-300 border border-white/10 rounded px-2 py-0.5" @click="lancerAnalyse">↺ Relancer</button>
              </div>
              <div v-if="chargementIA" class="flex items-center justify-center gap-2 py-6 text-xs text-gray-500">
                <div class="h-4 w-4 animate-spin rounded-full border-2 border-blue-500 border-t-transparent" />Analyse en cours…
              </div>
              <div v-else-if="erreurIA" class="text-xs text-red-400">{{ erreurIA }}</div>
              <div v-else-if="paragraphes.length" class="grid grid-cols-2 gap-2">
                <div
                  v-for="(p, i) in paragraphes" :key="i"
                  class="rounded-lg p-3"
                  :class="[p.type, i === paragraphes.length - 1 ? 'col-span-2' : '']"
                >
                  <p class="text-[11px] leading-relaxed" :class="p.textClass">{{ p.texte }}</p>
                </div>
              </div>
              <p v-else class="text-xs text-gray-600 italic">Aucune analyse pour l'instant.</p>
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

function classeScore(s: number) { return s >= 70 ? 'text-orange-400' : s >= 50 ? 'text-emerald-400' : 'text-gray-400' }

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
  const W = 160, H = 44
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
        contenu: `Tu es un trader algorithmique SMC Directionnel. Les positions sont EXCLUSIVEMENT des achats LONG avec sortie pyramidale :
- TP1 = 1×SL → clôture 33%, SL remonte au BreakEven | TP2 = 2×SL → clôture 50% | TP3 = trailing ATR (laisser courir)
Pour chaque crypto : évalue si le momentum haussier justifie une entrée LONG maintenant ou s'il faut attendre un pullback, estime le R-multiple réaliste.
VERDICT : « LONG — viser R2/R3 » | « LONG — viser R2 » | « Mouvement épuisé, éviter » | « Attendre pullback »
Règles : français uniquement, 2-3 phrases MAX par crypto, conclusion globale (meilleure crypto LONG + R-multiple), sépare par ligne vide.`
      },
      {
        role: 'user',
        contenu: `Top 5 cryptos avec fort momentum haussier sur Binance :\n\n${liste}\n\nPour chaque crypto : évalue si l'entrée LONG est justifiée maintenant ou s'il faut attendre un pullback, et indique le R-multiple réaliste (R1/R2/R3). Conclus sur la meilleure opportunité d'achat du moment.`
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
