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

    <!-- Paramètres SMC : supprimé ici, déplacé sous le bouton -->

    <!-- Formulaire signal -->
    <div class="glass-card-analyse">
      <div class="flex items-center gap-2 px-5 pt-4 pb-3 border-b border-white/5">
        <h2 class="text-sm font-semibold text-gray-300">🔍 Signal à analyser</h2>
        <span class="text-[10px] font-bold px-2 py-0.5 rounded-full bg-purple-600/30 text-purple-300 border border-purple-500/40 uppercase tracking-wider">Analyse Manuelle</span>
      </div>
      <div class="p-5 grid grid-cols-2 gap-4 lg:grid-cols-5">
        <div>
          <label class="label">Asset</label>
          <select v-model="form.asset" class="glass-select w-full">
            <option v-for="a in assetsIds" :key="a" :value="a">{{ a }}</option>
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
          <label class="label">Score SMC <span class="text-gray-600 font-normal">(calculé)</span></label>
          <div class="glass-input w-full flex items-center justify-between cursor-default">
            <span class="font-mono font-bold text-base" :class="scoreSmc >= smcParams.score_min ? 'text-emerald-400' : 'text-red-400'">{{ scoreSmc }}</span>
            <span class="text-xs text-gray-500">/ 80</span>
          </div>
        </div>
        <div>
          <label class="label">Confiance ML (%)</label>
          <input v-model.number="form.confiance_ml_pct" type="number" min="0" max="100" step="1" class="glass-input w-full" />
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
          <label class="label">TP1</label>
          <input v-model.number="form.tp1" type="number" step="0.01" class="glass-input w-full" />
        </div>
        <div>
          <label class="label">TP2</label>
          <input v-model.number="form.tp2" type="number" step="0.01" class="glass-input w-full" />
        </div>
        <div>
          <label class="label">TP3</label>
          <input v-model.number="form.tp3" type="number" step="0.01" class="glass-input w-full" />
        </div>
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

    <!-- Séparateur -->
    <div class="border-t-4 border-white/10 rounded-full" />

    <!-- Paramètres Moteur SMC -->
    <SmcParamsPanel v-model="smcParams" />

    <!-- Aide installation -->
    <div v-if="!ollamaOk" class="glass-card p-5 border-yellow-500/30 bg-yellow-900/10">
      <h3 class="text-yellow-400 font-semibold mb-2">⚠️ Ollama n'est pas démarré</h3>
      <pre class="text-xs text-gray-300 bg-black/30 p-3 rounded">curl -fsSL https://ollama.com/install.sh | sh
ollama pull qwen2.5vl:32b
ollama serve</pre>
    </div>

    <!-- Modale résultat draggable -->
    <SmcAnalyseResultModal
      :visible="modalVisible"
      :analyse="analyse"
      :modele="modeleActif"
      :asset="form.asset"
      :timeframe="form.timeframe"
      :direction="form.direction"
      :score="scoreSmc"
      :prix-entree="form.prix_entree"
      :stop-loss="form.stop_loss"
      :tp1="form.tp1"
      :tp2="form.tp2"
      :tp3="form.tp3"
      @fermer="modalVisible = false"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted } from 'vue'
import { useRoute } from 'vue-router'
import { apiService } from '@/services/api.service'
import { useAlerteStore } from '@/stores/alerte.store'
import { useAssetsStore } from '@/stores/assets.store'
import SmcParamsPanel, { type SmcParams } from '@/components/common/SmcParamsPanel.vue'
import SmcAnalyseResultModal from '@/components/common/SmcAnalyseResultModal.vue'

const smcParams = ref<SmcParams>({
  atr_periode: 14,
  score_min: 70,
  atr_tp1: 1.5,
  atr_tp2: 3.0,
  atr_tp3: 5.0,
  atr_sl: 1.0,
})

const alerteStore = useAlerteStore()
const assetsStore = useAssetsStore()
const assetsIds = computed(() =>
  assetsStore.assets.length > 0
    ? assetsStore.assets.map(a => a.id)
    : ['BTC', 'ETH']
)
const chargement = ref(false)
const ollamaOk = ref(false)
const modeleActif = ref('qwen2.5vl:32b')
const analyse = ref('')
const modalVisible = ref(false)

const scoreSmc = computed(() =>
  form.tendance + form.order_block + form.imbalance + form.ifvg + form.fibonacci
)

const form = reactive({
  asset: 'BTC',
  timeframe: 'M15',
  direction: 'LONG',
  prix_entree: 0,
  stop_loss: 0,
  tp1: 0,
  tp2: 0,
  tp3: 0,
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
      score_smc: scoreSmc.value,
      score_min: smcParams.value.score_min,
      prix_entree: form.prix_entree,
      stop_loss: form.stop_loss,
      take_profit_1: form.tp1,
      take_profit_2: form.tp2 || undefined,
      take_profit_3: form.tp3 || undefined,
      tendance: form.tendance,
      order_block: form.order_block,
      imbalance: form.imbalance,
      ifvg: form.ifvg,
      fibonacci: form.fibonacci,
      confiance_ml: form.confiance_ml_pct / 100,
    })
    analyse.value = res.analyse
    modeleActif.value = res.modele
    modalVisible.value = true
  } catch (e: unknown) {
    alerteStore.afficherErreur(`Ollama: ${(e as Error).message}`)
  } finally {
    chargement.value = false
  }
}

const route = useRoute()
onMounted(() => {
  assetsStore.chargerAssets()
  verifierStatut()
  const q = route.query
  if (q.asset) {
    form.asset = String(q.asset)
    form.timeframe = String(q.tf ?? 'M15')
    form.direction = String(q.dir ?? 'LONG')
    form.prix_entree = Number(q.entree ?? 0)
    form.stop_loss = Number(q.sl ?? 0)
    form.tp1 = Number(q.tp1 ?? 0)
    form.tp2 = Number(q.tp2 ?? 0)
    form.tp3 = Number(q.tp3 ?? 0)
  }
})
</script>

<style scoped>
.glass-card { @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm; }
.glass-card-analyse { @apply rounded-xl border border-purple-500/20 bg-purple-900/5 backdrop-blur-sm; }
.glass-select { @apply bg-white border border-gray-300 text-black text-sm rounded-lg px-3 py-2; }
.glass-select option { @apply text-black bg-white; }
.glass-input { @apply bg-gray-800 border border-gray-600 text-white text-sm rounded-lg px-3 py-2; }
.label { @apply text-xs text-gray-400 font-medium block mb-1; }
.fade-enter-active, .fade-leave-active { transition: opacity 0.4s; }
.fade-enter-from, .fade-leave-to { opacity: 0; }
</style>
