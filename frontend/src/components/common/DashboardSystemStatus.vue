<template>
  <div class="rounded-xl border border-cyan-500/25 bg-cyan-500/10 backdrop-blur-sm flex flex-col p-2 min-h-0 shrink-0">
    <div class="shrink-0 mb-1 border-b border-white/10 pb-1">
      <p class="text-xs uppercase font-bold text-white whitespace-nowrap truncate">⚙️ Data & IA Engine</p>
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
        <span v-else :class="mt5Ok ? 'text-emerald-400' : 'text-yellow-400'" class="text-[10px] font-semibold cursor-help"
              :title="`Flux M1 de l'EA — X/Y = symboles avec bougie récente (< 120 s) sur symboles suivis`">
          {{ mt5Ok ? `🟢 ${mt5Compteur} frais` : '🟡 Silence' }}
        </span>
      </div>
      <div class="flex items-center justify-between bg-white/5 rounded px-1.5 py-0.5 shrink-0">
        <span class="text-white text-[9px] uppercase">Bybit WS</span>
        <span :class="btcPrix ? 'text-emerald-400' : 'text-red-400'" class="text-[10px] font-semibold cursor-help"
              :title="`Flux temps réel crypto/métaux — bougies insérées depuis minuit (Paris)`">
          {{ btcPrix ? `🟢 ${bybitFlux}` : '🔴 Erreur' }}
        </span>
      </div>
      <div class="flex items-center justify-between bg-white/5 rounded px-1.5 py-0.5 shrink-0">
        <span class="text-white text-[9px] uppercase">LLM (Ollama)</span>
        <span v-if="ollamaOk === null" class="text-white text-[10px] font-semibold animate-pulse">⏳ Vérif</span>
        <span v-else :class="ollamaOk ? 'text-emerald-400' : 'text-red-400'" class="text-[10px] font-semibold cursor-help"
              :title="`Appels LLM passés aujourd'hui — ranker rockets, catalyseur news, analyses`">
          {{ ollamaOk ? `🟢 ${llmAppels} appel${llmAppels > 1 ? 's' : ''}` : '🔴 Hors ligne' }}
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
        <span :class="presseOk ? 'text-emerald-400' : 'text-yellow-400'" class="text-[10px] font-semibold cursor-help"
              :title="`Bibliothèque d'articles collectés (cycle 30 min)`">
          {{ presseOk ? `🟢 ${presseTotal} articles` : '🟡 Arrêt' }}
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
const mt5Compteur = ref('0/0')
/// Tiingo (veille actions) : vert si l'endpoint répond, avec avancement.
const tiingoOk = ref<boolean | null>(null)
const tiingoAvancement = ref('')
const tiingoTitre = ref('')
const presseOk = ref(false)
const presseTotal = ref(0)
const bybitFlux = ref('…')
const llmAppels = ref(0)

async function sonder() {
  try {
    const r = await http.get('/api/mt5/statut')
    mt5Ok.value = !!r.data?.connecte
    const syms: { age_s: number }[] = r.data?.symboles ?? []
    const frais = syms.filter(x => x.age_s >= 0 && x.age_s < 120).length
    mt5Compteur.value = `${frais}/${syms.length}`
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
  try {
    const r = await http.get('/api/presse/articles', { params: { page: 1 } })
    presseTotal.value = r.data?.total ?? 0
  } catch { presseTotal.value = 0 }
  try {
    const r = await http.get('/api/data/coverage')
    bybitFlux.value = `+${(r.data?.bougies_aujourd_hui ?? 0).toLocaleString('fr-FR')} bougies/j`
  } catch { bybitFlux.value = '—' }
  try {
    const r = await http.get('/api/ia/status')
    llmAppels.value = r.data?.appels_jour ?? 0
  } catch { llmAppels.value = 0 }
}

let poll: ReturnType<typeof setInterval> | null = null
onMounted(() => {
  void sonder()
  poll = setInterval(sonder, 30_000)
})
onUnmounted(() => { if (poll !== null) clearInterval(poll) })
</script>
