<template>
  <Teleport to="body">
    <div
      v-if="store.visible"
      class="alarme-wrapper"
      :style="{ left: pos.x + 'px', top: pos.y + 'px' }"
      @mousedown="startDrag"
    >
      <!-- En-tête draggable -->
      <div class="alarme-header">
        <div class="flex items-center gap-2 select-none cursor-grab active:cursor-grabbing">
          <span class="text-xs font-bold uppercase tracking-wider"
            :class="dirClass">{{ signal.direction }}</span>
          <span class="text-sm font-bold text-white">{{ signal.asset }}</span>
          <span class="text-xs text-gray-400">{{ signal.timeframe }}</span>
          <span class="text-[10px] px-1.5 py-0.5 rounded bg-gray-700 text-gray-300">{{ signal.strategie }}</span>
        </div>
        <div class="flex items-center gap-2">
          <!-- Navigation N/N -->
          <span v-if="store.total > 1" class="text-xs text-gray-400 select-none">
            <button class="px-1 hover:text-white disabled:opacity-30" :disabled="store.index === 0" @click.stop="store.precedent()">‹</button>
            {{ store.index + 1 }}/{{ store.total }}
            <button class="px-1 hover:text-white disabled:opacity-30" :disabled="store.index === store.total - 1" @click.stop="store.suivant()">›</button>
          </span>
          <button class="text-gray-500 hover:text-white text-lg leading-none" @click.stop="fermer">✕</button>
        </div>
      </div>

      <!-- Corps -->
      <div class="alarme-body">
        <!-- Prix clés -->
        <div class="grid grid-cols-3 gap-2 mb-3">
          <div class="prix-bloc">
            <span class="prix-label">Entrée</span>
            <span class="prix-val text-white">{{ fmt(signal.prix_entree) }}</span>
          </div>
          <div class="prix-bloc">
            <span class="prix-label">Stop-Loss</span>
            <span class="prix-val text-red-400">{{ fmt(signal.stop_loss) }}</span>
          </div>
          <div class="prix-bloc">
            <span class="prix-label">Score</span>
            <span class="prix-val" :class="signal.score >= 70 ? 'text-emerald-400' : 'text-yellow-400'">{{ signal.score.toFixed(0) }}/100</span>
          </div>
        </div>

        <!-- Take-profits -->
        <div class="grid grid-cols-3 gap-2 mb-3">
          <div v-for="(tp, i) in (signal.take_profit ?? []).slice(0, 3)" :key="i" class="prix-bloc">
            <span class="prix-label">TP{{ i + 1 }}</span>
            <span class="prix-val text-emerald-400">{{ fmt(tp) }}</span>
          </div>
        </div>

        <!-- Confiance ML -->
        <div v-if="signal.llm_conviction !== null" class="flex items-center gap-2 mb-3">
          <span class="text-xs text-gray-400">Conviction IA</span>
          <div class="flex-1 bg-gray-700 rounded-full h-1.5 overflow-hidden">
            <div class="h-full rounded-full transition-all"
              :style="{ width: signal.llm_conviction + '%' }"
              :class="signal.llm_conviction >= 70 ? 'bg-emerald-500' : signal.llm_conviction >= 50 ? 'bg-yellow-500' : 'bg-red-500'" />
          </div>
          <span class="text-xs font-mono font-bold" :class="signal.llm_conviction >= 70 ? 'text-emerald-400' : 'text-yellow-400'">
            {{ signal.llm_conviction }}%
          </span>
        </div>

        <!-- Justification LLM -->
        <div v-if="signal.llm_raison" class="raison-bloc">
          <span class="text-[10px] text-gray-500 uppercase tracking-wider block mb-1">Justification IA</span>
          <p class="text-xs text-gray-200 leading-relaxed">{{ signal.llm_raison }}</p>
        </div>
        <div v-else class="text-xs text-gray-600 italic">
          {{ signal.strategie === 'Straddle' ? 'Straddle : double position volatilité extrême' : 'Aucune justification LLM disponible' }}
        </div>
      </div>

      <!-- Pied -->
      <div class="alarme-footer">
        <span class="text-[10px] text-gray-600">{{ formatDate(signal.cree_le) }}</span>
        <button
          class="text-xs px-3 py-1 rounded bg-gray-700 hover:bg-gray-600 text-gray-300 transition-colors"
          @click.stop="fermer"
        >Ignorer ✕</button>
      </div>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useSignalAlarmeStore } from '@/stores/signal-alarme.store'
import { useNotification } from '@/composables/useNotification'

const store = useSignalAlarmeStore()
const { jouerSon } = useNotification()

const signal = computed(() => store.signalActuel!)

// ── Position draggable ────────────────────────────────────────────────────────
const pos = ref({ x: window.innerWidth - 380, y: 80 })
let dragging = false
let startMouse = { x: 0, y: 0 }
let startPos = { x: 0, y: 0 }

function startDrag(e: MouseEvent) {
  const target = e.target as HTMLElement
  if (target.closest('button')) return
  dragging = true
  startMouse = { x: e.clientX, y: e.clientY }
  startPos = { ...pos.value }
  window.addEventListener('mousemove', onDrag)
  window.addEventListener('mouseup', stopDrag)
}

function onDrag(e: MouseEvent) {
  if (!dragging) return
  pos.value = {
    x: Math.max(0, Math.min(window.innerWidth - 360, startPos.x + e.clientX - startMouse.x)),
    y: Math.max(0, Math.min(window.innerHeight - 200, startPos.y + e.clientY - startMouse.y)),
  }
}

function stopDrag() {
  dragging = false
  window.removeEventListener('mousemove', onDrag)
  window.removeEventListener('mouseup', stopDrag)
}

// ── Son à chaque nouveau signal ───────────────────────────────────────────────
watch(() => store.total, (n, prev) => {
  if (n > prev) jouerSon()
})

// ── Actions ───────────────────────────────────────────────────────────────────
function fermer() {
  store.fermerActuel()
}

// ── Helpers ───────────────────────────────────────────────────────────────────
const dirClass = computed(() => {
  const d = signal.value?.direction
  if (d === 'LONG') return 'text-emerald-400'
  if (d === 'SHORT') return 'text-red-400'
  return 'text-blue-400'
})

function fmt(n: number | null | undefined): string {
  if (n == null) return '—'
  return n >= 1000 ? n.toLocaleString('fr-FR', { maximumFractionDigits: 2 }) : n.toFixed(5)
}

function formatDate(ts: number): string {
  return new Date(ts * 1000).toLocaleString('fr-FR', { dateStyle: 'short', timeStyle: 'short' })
}
</script>

<style scoped>
.alarme-wrapper {
  position: fixed;
  z-index: 9999;
  width: 360px;
  border-radius: 0.75rem;
  background: rgba(10, 14, 39, 0.96);
  border: 1px solid rgba(255, 255, 255, 0.15);
  backdrop-filter: blur(20px);
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.7), 0 0 0 1px rgba(59, 130, 246, 0.2);
  user-select: none;
}
.alarme-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.75rem 1rem;
  border-bottom: 1px solid rgba(255, 255, 255, 0.07);
  cursor: grab;
}
.alarme-header:active { cursor: grabbing; }
.alarme-body {
  padding: 0.875rem 1rem;
}
.alarme-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.5rem 1rem;
  border-top: 1px solid rgba(255, 255, 255, 0.07);
}
.prix-bloc {
  display: flex;
  flex-direction: column;
  align-items: center;
  background: rgba(255, 255, 255, 0.04);
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 0.5rem;
  padding: 0.4rem 0.5rem;
}
.prix-label { font-size: 0.65rem; color: #6b7280; text-transform: uppercase; letter-spacing: 0.05em; }
.prix-val { font-size: 0.8rem; font-family: monospace; font-weight: 700; margin-top: 0.1rem; }
.raison-bloc {
  background: rgba(255, 255, 255, 0.03);
  border: 1px solid rgba(255, 255, 255, 0.06);
  border-radius: 0.5rem;
  padding: 0.5rem 0.625rem;
  max-height: 80px;
  overflow-y: auto;
}
</style>
