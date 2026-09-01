<template>
  <div class="glass-card flex flex-col p-2 min-h-0 shrink-0">
    <div class="shrink-0 mb-1 border-b border-white/10 pb-1">
      <p class="text-xs uppercase font-bold text-white whitespace-nowrap truncate">⚙️ Data & IA Engine</p>
      <p class="text-[9px] text-emerald-400 whitespace-nowrap">runtime v12 · signaux officiels</p>
    </div>

    <div class="flex flex-col gap-1.5 flex-1 overflow-y-auto">
      <div class="flex items-center justify-between bg-white/5 rounded px-1.5 py-0.5 shrink-0">
        <span class="text-white text-[9px] uppercase">API Serveur</span>
        <span :class="backendOk ? 'text-emerald-400' : 'text-red-400'" class="text-[10px] font-semibold">
          {{ backendOk ? '🟢 Actif' : '🔴 Erreur' }}
        </span>
      </div>
      <div class="flex items-center justify-between bg-white/5 rounded px-1.5 py-0.5 shrink-0">
        <span class="text-white text-[9px] uppercase">MT5 / Axi EA</span>
        <span v-if="mt5Ok === null" class="text-white text-[10px] font-semibold animate-pulse">⏳</span>
        <span v-else :class="mt5Ok ? 'text-emerald-400' : 'text-yellow-400'" class="text-[10px] font-semibold">
          {{ mt5Ok ? '🟢 Connecté' : '🟡 Silence' }}
        </span>
      </div>
      <div class="flex items-center justify-between bg-white/5 rounded px-1.5 py-0.5 shrink-0">
        <span class="text-white text-[9px] uppercase">Bybit WS</span>
        <span :class="btcPrix ? 'text-emerald-400' : 'text-red-400'" class="text-[10px] font-semibold">
          {{ btcPrix ? '🟢 Connecté' : '🔴 Erreur' }}
        </span>
      </div>
      <div class="flex items-center justify-between bg-white/5 rounded px-1.5 py-0.5 shrink-0">
        <span class="text-white text-[9px] uppercase">LLM (Ollama)</span>
        <span v-if="ollamaOk === null" class="text-white text-[10px] font-semibold animate-pulse">⏳ Vérif</span>
        <span v-else :class="ollamaOk ? 'text-emerald-400' : 'text-red-400'" class="text-[10px] font-semibold">
          {{ ollamaOk ? '🟢 Local' : '🔴 Hors ligne' }}
        </span>
      </div>
      <div class="flex items-center justify-between bg-white/5 rounded px-1.5 py-0.5 shrink-0">
        <span class="text-white text-[9px] uppercase">Tiingo Actions</span>
        <span v-if="tiingoOk === null" class="text-white text-[10px] font-semibold animate-pulse">⏳</span>
        <span v-else :class="tiingoOk ? 'text-emerald-400' : 'text-red-400'" class="text-[10px] font-semibold cursor-help" :title="tiingoTitre">
          {{ tiingoOk ? `🟢 ${tiingoAvancement}` : '🔴 Erreur' }}
        </span>
      </div>
      <div class="flex items-center justify-between bg-white/5 rounded px-1.5 py-0.5 shrink-0">
        <span class="text-white text-[9px] uppercase">Presse FR</span>
        <span :class="presseOk ? 'text-emerald-400' : 'text-yellow-400'" class="text-[10px] font-semibold">
          {{ presseOk ? '🟢 Collecteur' : '🟡 Arrêt' }}
        </span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { http } from '@/services/http.client'

defineProps<{
  backendOk: boolean
  btcPrix: number | null
  ollamaOk: boolean | null
}>()

/// MT5 EA : heartbeat < 120 s (l'endpoint porte le seuil).
const mt5Ok = ref<boolean | null>(null)
/// Tiingo (veille actions) : vert si l'endpoint répond, avec avancement.
const tiingoOk = ref<boolean | null>(null)
const tiingoAvancement = ref('')
const tiingoTitre = ref('')
const presseOk = ref(false)

async function sonder() {
  try {
    const r = await http.get('/api/mt5/statut')
    mt5Ok.value = !!r.data?.connecte
  } catch { mt5Ok.value = false }
  try {
    const r = await http.get('/api/rockets/actions/backfill/etat')
    const d = r.data as { univers_avec_bougies?: number; univers_total?: number; progression_pct?: number }
    tiingoOk.value = true
    tiingoAvancement.value = `${d.univers_avec_bougies ?? 0}/${d.univers_total ?? 0}`
    tiingoTitre.value = `Veille actions — backfill ${d.progression_pct ?? 0} % de l'univers (volume réel Tiingo)`
  } catch {
    tiingoOk.value = false
    tiingoTitre.value = 'Endpoint veille actions injoignable'
  }
  try {
    const r = await http.get('/api/presse/briefs')
    presseOk.value = Array.isArray(r.data) ? r.data.length > 0 : !!r.data
  } catch { presseOk.value = false }
}

let poll: ReturnType<typeof setInterval> | null = null
onMounted(() => {
  void sonder()
  poll = setInterval(sonder, 30_000)
})
onUnmounted(() => { if (poll !== null) clearInterval(poll) })
</script>
