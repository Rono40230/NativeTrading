<template>
  <div class="space-y-6">
    <h2 class="text-lg font-semibold text-white">🤖 Seuils de confiance ML par stratégie</h2>

    <div v-if="chargement" class="text-slate-400 text-sm">Chargement…</div>
    <div v-else class="space-y-5">

      <!-- Rockets -->
      <div class="glass-card p-4 space-y-3">
        <div class="flex items-center justify-between">
          <div>
            <p class="font-medium text-white">🚀 Rockets</p>
            <p class="text-xs text-slate-400">Rejette un signal Rockets si score XGBoost &lt; seuil</p>
          </div>
          <span class="text-emerald-400 font-mono font-bold">{{ (seuils.rockets * 100).toFixed(0) }}%</span>
        </div>
        <input
          type="range" min="0.30" max="0.90" step="0.05"
          v-model.number="seuils.rockets"
          class="w-full accent-emerald-500"
        />
        <div class="flex justify-between text-xs text-slate-500">
          <span>30% — permissif</span>
          <span>90% — strict</span>
        </div>
      </div>

      <!-- Straddle -->
      <div class="glass-card p-4 space-y-3">
        <div class="flex items-center justify-between">
          <div>
            <p class="font-medium text-white">⚡ Straddle</p>
            <p class="text-xs text-slate-400">Skip si ML trop confiant d'une direction (&gt; seuil) — signal directionnel préférable</p>
          </div>
          <span class="text-blue-400 font-mono font-bold">{{ (seuils.straddle * 100).toFixed(0) }}%</span>
        </div>
        <input
          type="range" min="0.50" max="0.95" step="0.05"
          v-model.number="seuils.straddle"
          class="w-full accent-blue-500"
        />
        <div class="flex justify-between text-xs text-slate-500">
          <span>50% — très sélectif</span>
          <span>95% — très permissif</span>
        </div>
      </div>

      <!-- SMC -->
      <div class="glass-card p-4 space-y-3">
        <div class="flex items-center justify-between">
          <div>
            <p class="font-medium text-white">△ SMC Directionnel</p>
            <p class="text-xs text-slate-400">Rejette un signal SMC si confiance ML &lt; seuil</p>
          </div>
          <span class="text-violet-400 font-mono font-bold">{{ (seuils.smc * 100).toFixed(0) }}%</span>
        </div>
        <input
          type="range" min="0.30" max="0.90" step="0.05"
          v-model.number="seuils.smc"
          class="w-full accent-violet-500"
        />
        <div class="flex justify-between text-xs text-slate-500">
          <span>30% — permissif</span>
          <span>90% — strict</span>
        </div>
      </div>

      <!-- Bouton enregistrer -->
      <button
        @click="enregistrer"
        :disabled="sauvegarde"
        class="w-full py-2 rounded-lg bg-emerald-600 hover:bg-emerald-500 disabled:opacity-50
               text-white font-semibold transition-colors"
      >
        {{ sauvegarde ? 'Enregistrement…' : '💾 Enregistrer les seuils' }}
      </button>

      <p v-if="message" class="text-center text-sm" :class="messageOk ? 'text-emerald-400' : 'text-red-400'">
        {{ message }}
      </p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { apiService } from '@/services/api.service'

const chargement = ref(true)
const sauvegarde = ref(false)
const message = ref('')
const messageOk = ref(true)

const seuils = ref({ rockets: 0.60, straddle: 0.75, smc: 0.60 })

async function chargerSeuil(cle: string): Promise<number | null> {
  try {
    const data = await apiService.obtenirConfig(cle)
    const val = parseFloat(data?.valeur ?? '')
    return isNaN(val) ? null : val
  } catch {
    return null
  }
}

onMounted(async () => {
  const [r, s, m] = await Promise.all([
    chargerSeuil('seuil_confiance_rockets'),
    chargerSeuil('seuil_confiance_straddle'),
    chargerSeuil('seuil_confiance_smc'),
  ])
  if (r !== null) seuils.value.rockets = r
  if (s !== null) seuils.value.straddle = s
  if (m !== null) seuils.value.smc = m
  chargement.value = false
})

async function enregistrer() {
  sauvegarde.value = true
  message.value = ''
  try {
    await Promise.all([
      apiService.sauvegarderConfig('seuil_confiance_rockets', String(seuils.value.rockets)),
      apiService.sauvegarderConfig('seuil_confiance_straddle', String(seuils.value.straddle)),
      apiService.sauvegarderConfig('seuil_confiance_smc', String(seuils.value.smc)),
    ])
    messageOk.value = true
    message.value = '✅ Seuils enregistrés'
  } catch {
    messageOk.value = false
    message.value = '❌ Échec de l\'enregistrement'
  } finally {
    sauvegarde.value = false
  }
}
</script>

<style scoped>
.glass-card {
  background: rgba(255, 255, 255, 0.05);
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 0.75rem;
  backdrop-filter: blur(12px);
}
</style>
