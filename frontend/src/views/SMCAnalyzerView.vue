<template>
  <div class="space-y-6">
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-2xl font-bold">🧠 Analyse IA SMC</h1>
        <p class="text-xs text-gray-500 mt-0.5">Via Ollama local — {{ modeleActif }}</p>
      </div>
      <span
        class="text-xs px-2 py-1 rounded-full font-semibold"
        :class="ollamaOk ? 'bg-emerald-900/50 text-emerald-300' : 'bg-red-900/50 text-red-300'"
      >
        {{ ollamaOk ? '🟢 Ollama actif' : '🔴 Ollama hors ligne' }}
      </span>
    </div>

    <!-- Onglets -->
    <div class="flex gap-1 p-1 bg-white/5 rounded-xl border border-white/10 self-start">
      <button
        class="px-4 py-1.5 rounded-lg text-sm font-medium transition-colors"
        :class="onglet === 'signal' ? 'bg-blue-600 text-white' : 'text-gray-400 hover:text-white'"
        @click="onglet = 'signal'"
      >📊 Signal</button>
      <button
        class="px-4 py-1.5 rounded-lg text-sm font-medium transition-colors"
        :class="onglet === 'chart' ? 'bg-purple-600 text-white' : 'text-gray-400 hover:text-white'"
        @click="onglet = 'chart'"
      >🖼️ Chart Import</button>
    </div>

    <!-- Onglet Signal -->
    <div v-show="onglet === 'signal'" class="space-y-4">

    <!-- Formulaire signal -->
    <div class="glass-card p-5 grid grid-cols-2 gap-4 lg:grid-cols-4">
      <div>
        <label class="label">Asset</label>
        <select v-model="form.asset" class="glass-select w-full">
          <option v-for="a in ['BTC', 'ETH']" :key="a" :value="a">{{ a }}</option>
        </select>
      </div>
      <div>
        <label class="label">Timeframe</label>
        <select v-model="form.timeframe" class="glass-select w-full">
          <option v-for="tf in ['M5', 'M15', 'H1', 'H4']" :key="tf" :value="tf">{{ tf }}</option>
        </select>
      </div>
      <div>
        <label class="label">Direction</label>
        <select v-model="form.direction" class="glass-select w-full">
          <option value="LONG">LONG</option>
          <option value="SHORT">SHORT</option>
        </select>
      </div>
      <div>
        <label class="label">Score SMC</label>
        <input v-model.number="form.score_smc" type="number" min="0" max="100" step="1" class="glass-input w-full" />
      </div>
      <div>
        <label class="label">Prix d'entrée</label>
        <input v-model.number="form.prix_entree" type="number" step="0.01" class="glass-input w-full" />
      </div>
      <div>
        <label class="label">Stop-Loss</label>
        <input v-model.number="form.stop_loss" type="number" step="0.01" class="glass-input w-full" />
      </div>
      <div>
        <label class="label">Take-Profit</label>
        <input v-model.number="form.take_profit" type="number" step="0.01" class="glass-input w-full" />
      </div>
      <div>
        <label class="label">Confiance ML (%)</label>
        <input v-model.number="form.confiance_ml_pct" type="number" min="0" max="100" step="1" class="glass-input w-full" />
      </div>
    </div>

    <!-- Curseurs SMC -->
    <div class="glass-card p-5 grid grid-cols-2 gap-4 lg:grid-cols-5">
      <div v-for="champ in champsSmc" :key="champ.key" class="space-y-1">
        <div class="flex justify-between">
          <label class="label">{{ champ.label }}</label>
          <span class="text-xs text-white font-mono">{{ form[champ.key] }}</span>
        </div>
        <input v-model.number="form[champ.key]" type="range" min="0" :max="champ.max" step="1" class="w-full accent-blue-500" />
      </div>
    </div>

    <!-- Bouton -->
    <button
      class="w-full py-3 rounded-xl font-bold text-lg transition-all"
      :class="chargement || !ollamaOk ? 'bg-gray-700 cursor-not-allowed' : 'bg-gradient-to-r from-blue-600 to-purple-600 hover:brightness-110'"
      :disabled="chargement || !ollamaOk"
      @click="analyser"
    >
      {{ chargement ? '⏳ Analyse en cours...' : '🔍 Analyser ce signal' }}
    </button>

    <!-- Résultat -->
    <transition name="fade">
      <div v-if="analyse" class="glass-card p-6 space-y-3">
        <div class="flex items-center justify-between">
          <h2 class="text-sm font-semibold text-gray-400 uppercase tracking-wider">Analyse IA</h2>
          <span class="text-xs text-gray-500">{{ modeleActif }}</span>
        </div>
        <div class="text-gray-100 leading-relaxed whitespace-pre-wrap text-sm">{{ analyse }}</div>
      </div>
    </transition>

    <!-- Aide installation -->
    <div v-if="!ollamaOk" class="glass-card p-5 border-yellow-500/30 bg-yellow-900/10">
      <h3 class="text-yellow-400 font-semibold mb-2">⚠️ Ollama n'est pas démarré</h3>
      <pre class="text-xs text-gray-300 bg-black/30 p-3 rounded">curl -fsSL https://ollama.com/install.sh | sh
ollama pull qwen2.5:14b
ollama serve</pre>
    </div>
    </div><!-- /onglet signal -->

    <!-- Onglet Chart Import -->
    <ChartImportPanel v-show="onglet === 'chart'" />
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { apiService } from '@/services/api.service'
import { useAlerteStore } from '@/stores/alerte.store'
import ChartImportPanel from '@/components/common/ChartImportPanel.vue'

const alerteStore = useAlerteStore()
const onglet = ref<'signal' | 'chart'>('signal')
const chargement = ref(false)
const ollamaOk = ref(false)
const modeleActif = ref('qwen2.5:14b')
const analyse = ref('')

const form = reactive({
  asset: 'BTC',
  timeframe: 'M15',
  direction: 'LONG',
  score_smc: 75,
  prix_entree: 0,
  stop_loss: 0,
  take_profit: 0,
  confiance_ml_pct: 65,
  tendance: 30,
  order_block: 20,
  imbalance: 15,
  ifvg: 10,
  fibonacci: 5,
})

const champsSmc = [
  { key: 'tendance' as const, label: 'Tendance', max: 30 },
  { key: 'order_block' as const, label: 'Order Block', max: 20 },
  { key: 'imbalance' as const, label: 'Imbalance', max: 15 },
  { key: 'ifvg' as const, label: 'IFVG', max: 10 },
  { key: 'fibonacci' as const, label: 'Fibonacci', max: 5 },
]

async function verifierStatut() {
  try {
    const s = await apiService.statutIA()
    ollamaOk.value = s.ollama_disponible
    modeleActif.value = s.modele
  } catch {
    ollamaOk.value = false
  }
}

async function analyser() {
  chargement.value = true
  analyse.value = ''
  try {
    const res = await apiService.analyserIA({
      asset: form.asset,
      timeframe: form.timeframe,
      direction: form.direction,
      score_smc: form.score_smc,
      prix_entree: form.prix_entree,
      stop_loss: form.stop_loss,
      take_profit: form.take_profit,
      tendance: form.tendance,
      order_block: form.order_block,
      imbalance: form.imbalance,
      ifvg: form.ifvg,
      fibonacci: form.fibonacci,
      confiance_ml: form.confiance_ml_pct / 100,
    })
    analyse.value = res.analyse
    modeleActif.value = res.modele
  } catch (e: unknown) {
    alerteStore.afficherErreur(`Ollama: ${(e as Error).message}`)
  } finally {
    chargement.value = false
  }
}

onMounted(verifierStatut)
</script>

<style scoped>
.glass-card { @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm; }
.glass-select { @apply bg-white border border-gray-300 text-black text-sm rounded-lg px-3 py-2; }
.glass-select option { @apply text-black bg-white; }
.glass-input { @apply bg-gray-800 border border-gray-600 text-white text-sm rounded-lg px-3 py-2; }
.label { @apply text-xs text-gray-400 font-medium block mb-1; }
.fade-enter-active, .fade-leave-active { transition: opacity 0.4s; }
.fade-enter-from, .fade-leave-to { opacity: 0; }
</style>
